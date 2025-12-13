mod db;
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

    let mut app = App::new();

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

        if !app.input.is_empty() {
            let user_question = app.input.clone();
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

            let sql_result = openai
                .generate_sql(&user_question, &schema_info, app.current_strategy)
                .await;

            let sql = match sql_result {
                Ok(sql) => sql,
                Err(e) => {
                    app.set_loading(false);
                    app.set_error(&format!("Failed to generate SQL: {}", e));
                    continue;
                }
            };

            let query_result = db.execute_query(&sql);

            match query_result {
                Ok(result) => {
                    let summary = openai
                        .format_results_as_natural_language(&user_question, &sql, result.row_count)
                        .await
                        .ok();

                    app.set_success(sql, result, summary);
                }
                Err(e) => {
                    app.set_loading(false);
                    app.set_error(&format!("SQL execution failed: {}", e));
                    app.generated_sql = Some(sql);
                }
            }

            app.set_loading(false);
        }
    }

    Ok(())
}
