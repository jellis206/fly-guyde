mod db;
mod fuzzy;
mod openai;
mod ui;

use anyhow::{Context, Result};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use db::Database;
use openai::{OpenAIClient, PromptStrategy};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use ui::App;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let api_key = std::env::var("OPENAI_API_KEY")
        .context("OPENAI_API_KEY not found in environment. Please set it in .env file")?;

    let db = Database::new("flies.db")
        .context("Failed to open database. Make sure flies.db exists in the current directory")?;

    let openai = OpenAIClient::new(api_key);

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Load persistent query history from database
    let history = db.load_query_history().unwrap_or_else(|e| {
        eprintln!("Warning: Failed to load query history: {}", e);
        Vec::new()
    });

    let mut app = App::new(history);

    let result = run(&mut terminal, &mut app, &db, &openai).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {}", e);
    }

    Ok(())
}

async fn run<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    db: &Database,
    openai: &OpenAIClient,
) -> Result<()> {
    loop {
        ui::run_app(terminal, app)?;

        if app.should_quit {
            break;
        }

        if !app.input.is_empty() && app.submitted {
            let user_question = app.input.clone();
            app.add_to_history(user_question.clone());

            // Save query to persistent history
            if let Err(e) = db.save_query_to_history(&user_question) {
                eprintln!("Warning: Failed to save query to history: {}", e);
            }

            // Set the question for display
            app.set_question(user_question.clone());

            app.clear_input();
            app.set_loading(true);

            terminal.draw(|f| ui::draw(f, app))?;

            let schema_info = match app.current_strategy {
                PromptStrategy::Basic => db.get_schema_info(),
                PromptStrategy::Intermediate => db.get_schema_with_relationships(),
                PromptStrategy::Advanced => db.get_schema_with_sample_data(),
            };

            let schema_info = match schema_info {
                Ok(info) => info,
                Err(e) => {
                    app.set_loading(false);
                    app.set_error(&format!("Failed to get schema: {}", e));
                    continue;
                }
            };

            // Try traditional SQL generation first
            let sql_result = openai
                .generate_sql(&user_question, &schema_info, app.current_strategy)
                .await;

            let mut should_try_recommendations = false;

            match sql_result {
                Ok(sql) => {
                    // SQL generated successfully, try to execute it
                    match db.execute_query(&sql) {
                        Ok(result) if result.row_count > 0 => {
                            // Success! Found results
                            let summary = openai
                                .format_results_as_natural_language(&user_question, &sql, result.row_count)
                                .await
                                .ok();

                            app.set_success(sql, result, summary);
                            app.set_loading(false);
                        }
                        Ok(_result) => {
                            // Query succeeded but returned 0 results - try recommendations
                            should_try_recommendations = true;
                        }
                        Err(_e) => {
                            // Query failed - try recommendations
                            should_try_recommendations = true;
                        }
                    }
                }
                Err(_e) => {
                    // SQL generation failed - try recommendations
                    should_try_recommendations = true;
                }
            }

            // Fallback to recommendation mode if needed and enabled
            if should_try_recommendations && app.recommendation_fallback_enabled {
                use fuzzy::FuzzyMatcher;

                app.status_message = "No results found. Trying recommendation mode...".to_string();
                terminal.draw(|f| ui::draw(f, app))?;

                // Get fly recommendations from ChatGPT
                match openai.get_fly_recommendations(&user_question).await {
                    Ok(recommendations) => {
                        let mut all_results = Vec::new();
                        let mut all_queries = Vec::new();

                        // For each recommendation, generate patterns and search
                        for recommendation in &recommendations {
                            let patterns = FuzzyMatcher::generate_patterns(recommendation);

                            // Build SQL queries for display
                            for pattern in &patterns {
                                all_queries.push(format!(
                                    "-- Searching for: {} (pattern: {})\nSELECT DISTINCT p.title, p.vendor, p.price_min, p.description\nFROM products p\nWHERE p.title LIKE '{}' OR p.description LIKE '{}'\nLIMIT 20;",
                                    recommendation, pattern, pattern, pattern
                                ));
                                all_queries.push(format!(
                                    "SELECT DISTINCT p.title, p.vendor, p.price_min\nFROM products p\nJOIN product_tags pt ON p.id = pt.product_id\nJOIN tags t ON pt.tag_id = t.id\nWHERE t.name LIKE '{}'\nLIMIT 20;",
                                    pattern
                                ));
                            }

                            match db.fuzzy_search_products(recommendation, &patterns) {
                                Ok(results) => {
                                    all_results.extend(results);
                                }
                                Err(e) => {
                                    // Log error but continue with other recommendations
                                    eprintln!("Error searching for {}: {}", recommendation, e);
                                }
                            }
                        }

                        if all_results.is_empty() {
                            app.set_loading(false);
                            app.set_error("No matches found even with recommendations. Try a different query.");
                        } else {
                            // Merge and score results
                            match db.merge_and_score_results(all_results) {
                                Ok(merged_result) => {
                                    let queries_sql = all_queries.join("\n\n");
                                    app.set_recommendation_success(recommendations, merged_result, queries_sql);
                                    app.set_loading(false);
                                }
                                Err(e) => {
                                    app.set_loading(false);
                                    app.set_error(&format!("Failed to process results: {}", e));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        app.set_loading(false);
                        app.set_error(&format!("Recommendation mode failed: {}", e));
                    }
                }
            } else if should_try_recommendations {
                // Fallback is disabled, show error message
                app.set_loading(false);
                app.set_error("No results found. (Recommendation fallback is disabled - press 'r' to enable)");
            }
        }
    }

    Ok(())
}
