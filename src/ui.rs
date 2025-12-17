use crate::db::QueryResult;
use crate::openai::PromptStrategy;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, Wrap},
};
use std::io;

pub enum InputMode {
    Normal,
    Editing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    QueryDetails,
    Results,
}

pub struct App {
    pub input: String,
    pub input_mode: InputMode,
    pub current_strategy: PromptStrategy,
    pub generated_sql: Option<String>,
    pub query_result: Option<QueryResult>,
    pub status_message: String,
    pub natural_language_summary: Option<String>,
    pub should_quit: bool,
    pub is_loading: bool,
    pub show_help: bool,
    pub recommendations: Option<Vec<String>>,
    pub is_recommendation_mode: bool,
    pub query_history: Vec<String>,
    pub history_index: Option<usize>,
    pub current_input_buffer: String,
    pub scroll_offset: usize,
    pub last_question: Option<String>,
    pub sql_scroll_offset: usize,
    pub focused_pane: FocusedPane,
    pub recommendation_fallback_enabled: bool,
}

impl App {
    pub fn new(initial_history: Vec<String>) -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            current_strategy: PromptStrategy::Basic,
            generated_sql: None,
            query_result: None,
            status_message: "Ready. Press '?' for help".to_string(),
            natural_language_summary: None,
            should_quit: false,
            is_loading: false,
            show_help: false,
            recommendations: None,
            is_recommendation_mode: false,
            query_history: initial_history,
            history_index: None,
            current_input_buffer: String::new(),
            scroll_offset: 0,
            last_question: None,
            sql_scroll_offset: 0,
            focused_pane: FocusedPane::Results, // Default focus on results
            recommendation_fallback_enabled: true, // Enabled by default
        }
    }

    pub fn set_strategy(&mut self, strategy: PromptStrategy) {
        self.current_strategy = strategy;
        self.status_message = format!(
            "Strategy: {} - {}",
            self.current_strategy.name(),
            self.current_strategy.description()
        );
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
        if self.show_help {
            self.status_message = "Showing help. Press '?' again to close".to_string();
        } else {
            self.status_message = "Ready. Press '?' for help".to_string();
        }
    }

    pub fn toggle_focus(&mut self) {
        self.focused_pane = match self.focused_pane {
            FocusedPane::QueryDetails => FocusedPane::Results,
            FocusedPane::Results => FocusedPane::QueryDetails,
        };
        let focus_name = match self.focused_pane {
            FocusedPane::QueryDetails => "Query Details",
            FocusedPane::Results => "Results",
        };
        self.status_message = format!("Focus: {} panel (use ↑↓ to scroll)", focus_name);
    }

    pub fn toggle_recommendation_fallback(&mut self) {
        self.recommendation_fallback_enabled = !self.recommendation_fallback_enabled;
        self.status_message = format!(
            "Recommendation fallback: {}",
            if self.recommendation_fallback_enabled {
                "ENABLED (will try recommendations if SQL fails)"
            } else {
                "DISABLED (SQL mode only)"
            }
        );
    }

    pub fn handle_input(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.should_quit = true;
                }
                KeyCode::Char('?') => {
                    self.toggle_help();
                }
                KeyCode::Tab => {
                    self.toggle_focus();
                }
                KeyCode::Char('i') | KeyCode::Char('I') => {
                    if !self.is_loading {
                        self.input_mode = InputMode::Editing;
                        self.status_message =
                            "Type your question, then press Enter to submit (Esc to cancel)"
                                .to_string();
                        self.show_help = false;
                    }
                }
                KeyCode::Char('1') => {
                    self.set_strategy(PromptStrategy::Basic);
                }
                KeyCode::Char('2') => {
                    self.set_strategy(PromptStrategy::Intermediate);
                }
                KeyCode::Char('3') => {
                    self.set_strategy(PromptStrategy::Advanced);
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    self.toggle_recommendation_fallback();
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    match self.focused_pane {
                        FocusedPane::QueryDetails => self.sql_scroll_up(),
                        FocusedPane::Results => self.scroll_up(),
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    match self.focused_pane {
                        FocusedPane::QueryDetails => self.sql_scroll_down(1000),
                        FocusedPane::Results => self.scroll_down(),
                    }
                }
                KeyCode::PageUp => {
                    match self.focused_pane {
                        FocusedPane::QueryDetails => {
                            for _ in 0..10 {
                                self.sql_scroll_up();
                            }
                        }
                        FocusedPane::Results => self.scroll_page_up(10),
                    }
                }
                KeyCode::PageDown => {
                    match self.focused_pane {
                        FocusedPane::QueryDetails => {
                            for _ in 0..10 {
                                self.sql_scroll_down(1000);
                            }
                        }
                        FocusedPane::Results => self.scroll_page_down(10),
                    }
                }
                KeyCode::Home => {
                    match self.focused_pane {
                        FocusedPane::QueryDetails => self.sql_scroll_offset = 0,
                        FocusedPane::Results => self.scroll_offset = 0,
                    }
                }
                KeyCode::End => {
                    match self.focused_pane {
                        FocusedPane::QueryDetails => {
                            self.sql_scroll_down(1000); // Will be bounded in draw
                        }
                        FocusedPane::Results => {
                            if let Some(result) = &self.query_result {
                                self.scroll_offset = result.row_count.saturating_sub(1);
                            }
                        }
                    }
                }
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Enter => {
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Char(c) => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) && c == 'c' {
                        self.should_quit = true;
                    } else {
                        self.input.push(c);
                        // Reset history navigation when user types
                        self.history_index = None;
                    }
                }
                KeyCode::Backspace => {
                    self.input.pop();
                    // Reset history navigation when user edits
                    self.history_index = None;
                }
                KeyCode::Up => {
                    self.history_previous();
                }
                KeyCode::Down => {
                    self.history_next();
                }
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                    self.history_index = None;
                    self.status_message =
                        "Cancelled. Press 'i' to enter a query, '?' for help".to_string();
                }
                _ => {}
            },
        }
    }

    pub fn set_loading(&mut self, loading: bool) {
        self.is_loading = loading;
        if loading {
            self.status_message = "Processing query with OpenAI...".to_string();
        }
    }

    pub fn set_error(&mut self, error: &str) {
        self.status_message = format!("Error: {}", error);
        self.generated_sql = None;
        self.query_result = None;
        self.natural_language_summary = None;
    }

    pub fn set_success(&mut self, sql: String, result: QueryResult, summary: Option<String>) {
        self.generated_sql = Some(sql);
        self.query_result = Some(result.clone());
        self.natural_language_summary = summary;
        self.recommendations = None;
        self.is_recommendation_mode = false;
        self.scroll_offset = 0; // Reset scroll for new results
        self.sql_scroll_offset = 0; // Reset SQL scroll for new query
        self.status_message = format!(
            "Success! Found {} result{}",
            result.row_count,
            if result.row_count == 1 { "" } else { "s" }
        );
    }

    pub fn set_recommendation_success(
        &mut self,
        recommendations: Vec<String>,
        result: QueryResult,
        fuzzy_sql: String,
    ) {
        self.recommendations = Some(recommendations.clone());
        self.is_recommendation_mode = true;
        self.query_result = Some(result.clone());
        self.generated_sql = Some(fuzzy_sql); // Show the fuzzy search SQL queries
        self.scroll_offset = 0; // Reset scroll for new results
        self.sql_scroll_offset = 0; // Reset SQL scroll for new query
        self.natural_language_summary = Some(format!(
            "Based on recommendations: {}",
            recommendations.join(", ")
        ));
        self.status_message = format!(
            "Recommendation Mode: Found {} result{} from {} recommendations",
            result.row_count,
            if result.row_count == 1 { "" } else { "s" },
            recommendations.len()
        );
    }

    pub fn clear_input(&mut self) {
        self.input.clear();
    }

    pub fn set_question(&mut self, question: String) {
        self.last_question = Some(question);
    }

    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        if let Some(result) = &self.query_result {
            // Don't scroll past the last row
            if self.scroll_offset < result.row_count.saturating_sub(1) {
                self.scroll_offset += 1;
            }
        }
    }

    pub fn scroll_page_up(&mut self, page_size: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(page_size);
    }

    pub fn scroll_page_down(&mut self, page_size: usize) {
        if let Some(result) = &self.query_result {
            self.scroll_offset =
                (self.scroll_offset + page_size).min(result.row_count.saturating_sub(1));
        }
    }

    pub fn sql_scroll_up(&mut self) {
        if self.sql_scroll_offset > 0 {
            self.sql_scroll_offset -= 1;
        }
    }

    pub fn sql_scroll_down(&mut self, max_lines: usize) {
        // max_lines is the total number of lines in the SQL content
        if max_lines > 0 {
            self.sql_scroll_offset = (self.sql_scroll_offset + 1).min(max_lines.saturating_sub(1));
        }
    }

    pub fn add_to_history(&mut self, query: String) {
        if !query.trim().is_empty() {
            // Don't add duplicates of the last query
            if self.query_history.last() != Some(&query) {
                self.query_history.push(query);
            }
            self.history_index = None;
            self.current_input_buffer.clear();
        }
    }

    pub fn history_previous(&mut self) {
        if self.query_history.is_empty() {
            return;
        }

        // Save current input if we're not already navigating history
        if self.history_index.is_none() {
            self.current_input_buffer = self.input.clone();
        }

        let new_index = match self.history_index {
            None => Some(self.query_history.len() - 1),
            Some(0) => Some(0), // Stay at oldest
            Some(i) => Some(i - 1),
        };

        if let Some(idx) = new_index {
            self.history_index = Some(idx);
            self.input = self.query_history[idx].clone();
        }
    }

    pub fn history_next(&mut self) {
        if self.query_history.is_empty() {
            return;
        }

        match self.history_index {
            None => {} // Already at current input
            Some(idx) if idx >= self.query_history.len() - 1 => {
                // Restore the buffer (what user was typing before pressing up)
                self.input = self.current_input_buffer.clone();
                self.history_index = None;
            }
            Some(idx) => {
                let new_idx = idx + 1;
                self.history_index = Some(new_idx);
                self.input = self.query_history[new_idx].clone();
            }
        }
    }
}

pub fn draw(f: &mut Frame, app: &App) {
    if app.show_help {
        draw_help(f);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Length(3),  // Input
            Constraint::Length(12), // SQL display (increased for question + queries)
            Constraint::Length(3),  // Summary
            Constraint::Min(10),    // Results table
            Constraint::Length(3),  // Status bar
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_input(f, app, chunks[1]);
    draw_sql(f, app, chunks[2]);
    draw_summary(f, app, chunks[3]);
    draw_results(f, app, chunks[4]);
    draw_status(f, app, chunks[5]);
}

fn draw_help(f: &mut Frame) {
    let help_text = vec![
        "╔═══════════════════════════════════════════════════════════════════════════════╗",
        "║                                 FLY-GUYDE HELP                                ║",
        "╠═══════════════════════════════════════════════════════════════════════════════╣",
        "║                                                                               ║",
        "║  KEYBOARD SHORTCUTS:                                                          ║",
        "║                                                                               ║",
        "║    i / I      Enter query mode (type your question)                           ║",
        "║    Enter      Submit your query                                               ║",
        "║    Tab        Switch focus between Query Details and Results panels           ║",
        "║    ↑ / ↓      Navigate query history (while editing)                          ║",
        "║    ↑ / ↓      Scroll the focused panel (in normal mode)                       ║",
        "║    j / k      Scroll the focused panel (vim-style)                            ║",
        "║    PgUp/PgDn  Scroll the focused panel by page                                ║",
        "║    Home/End   Jump to start/end of focused panel                              ║",
        "║    Esc        Cancel input                                                    ║",
        "║    q / Q      Quit application                                                ║",
        "║    ?          Toggle this help screen                                         ║",
        "║                                                                               ║",
        "║  PROMPTING STRATEGIES:                                                        ║",
        "║                                                                               ║",
        "║    1          Basic (Zero-shot) - Schema only                                 ║",
        "║    2          Intermediate - Schema + relationships + samples                 ║",
        "║    3          Advanced (Few-shot) - Includes example queries                  ║",
        "║                                                                               ║",
        "║  RECOMMENDATION FALLBACK:                                                     ║",
        "║                                                                               ║",
        "║    r / R      Toggle recommendation fallback ON/OFF                           ║",
        "║               When ON: tries recommendations if SQL returns no results        ║",
        "║               When OFF: only uses SQL mode (no fallback)                      ║",
        "║                                                                               ║",
        "║  EXAMPLE QUERIES:                                                             ║",
        "║                                                                               ║",
        "║    • Show me all dry flies under $5                                           ║",
        "║    • What streamers does Umpqua make?                                         ║",
        "║    • Find flies for euro nymphing                                             ║",
        "║    • Show the most expensive flies                                            ║",
        "║    • List all vendors and their product counts                                ║",
        "║    • Find beadhead nymphs                                                     ║",
        "║                                                                               ║",
        "║  TIPS:                                                                        ║",
        "║                                                                               ║",
        "║    • The Basic strategy is fastest but may struggle with complex queries      ║",
        "║    • The Advanced strategy is most accurate but uses more tokens              ║",
        "║    • Try different strategies to compare results                              ║",
        "║    • The generated SQL is shown for transparency                              ║",
        "║                                                                               ║",
        "║  SMART FALLBACK (RECOMMENDATION MODE):                                        ║",
        "║                                                                               ║",
        "║    If your query returns no results, the app automatically:                   ║",
        "║      1. Asks ChatGPT for fly fishing recommendations                          ║",
        "║      2. Searches the database with fuzzy matching                             ║",
        "║      3. Shows results sorted by relevance                                     ║",
        "║                                                                               ║",
        "║    Example: \"What flies for bonefish in Hawaii?\" might not match exact      ║",
        "║    database entries, but recommendations like \"Gotcha\" or \"Crazy Charlie\" ║",
        "║    will find similar products with fuzzy search.                              ║",
        "║                                                                               ║",
        "╚═══════════════════════════════════════════════════════════════════════════════╝",
        "",
        "                    Press '?' to close this help screen",
    ];

    let help_paragraph = Paragraph::new(help_text.join("\n"))
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default())
        .wrap(Wrap { trim: false });

    f.render_widget(help_paragraph, f.area());
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let fallback_status = if app.recommendation_fallback_enabled {
        "Fallback: ON"
    } else {
        "Fallback: OFF"
    };

    let header_text = format!(
        "Fly-Guyde 🎣  |  Strategy: {} - {}  |  {}  |  Press '?' for help",
        app.current_strategy.name(),
        app.current_strategy.description(),
        fallback_status
    );

    let header = Paragraph::new(header_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));

    f.render_widget(header, area);
}

fn draw_input(f: &mut Frame, app: &App, area: Rect) {
    let input_text = if matches!(app.input_mode, InputMode::Editing) {
        format!("{}_", app.input)
    } else {
        app.input.clone()
    };

    let style = match app.input_mode {
        InputMode::Normal => Style::default().fg(Color::White),
        InputMode::Editing => Style::default().fg(Color::Yellow),
    };

    let input = Paragraph::new(input_text)
        .style(style)
        .block(Block::default().borders(Borders::ALL).title("Question"));

    f.render_widget(input, area);
}

fn draw_sql(f: &mut Frame, app: &App, area: Rect) {
    // Build the full content with question + SQL/recommendations
    let mut full_content = String::new();

    // Add the original question if available
    if let Some(question) = &app.last_question {
        full_content.push_str(&format!("Question: {}\n\n", question));
    }

    // Add the SQL or recommendations
    let (title, content_text) = if app.is_recommendation_mode {
        let recommendations_text = if let Some(recs) = &app.recommendations {
            let mut text = format!("Recommendations:\n  • {}\n\n", recs.join("\n  • "));
            // Add SQL queries if available
            if let Some(sql) = &app.generated_sql {
                text.push_str("Fuzzy Search Queries:\n");
                text.push_str(sql);
            } else {
                text.push_str("Using fuzzy search patterns to find matching products...");
            }
            text
        } else {
            "Recommendation mode active...".to_string()
        };
        ("Query Details & Recommendations", recommendations_text)
    } else if app.is_loading {
        ("Query Details", "Generating SQL...".to_string())
    } else {
        (
            "Query Details & SQL",
            app.generated_sql
                .clone()
                .unwrap_or_else(|| "No query yet".to_string()),
        )
    };

    full_content.push_str(&content_text);

    // Split into lines for scrolling
    let lines: Vec<&str> = full_content.lines().collect();
    let total_lines = lines.len();

    // Calculate visible window (subtract 2 for borders)
    let visible_height = area.height.saturating_sub(2) as usize;
    let start_line = app.sql_scroll_offset.min(total_lines.saturating_sub(1));
    let end_line = (start_line + visible_height).min(total_lines);

    // Get visible lines
    let visible_text = if total_lines == 0 {
        String::new()
    } else {
        lines[start_line..end_line].join("\n")
    };

    // Create scroll indicator
    let scroll_indicator = if total_lines > visible_height {
        format!(" [{}-{}/{}] [ ] to scroll ", start_line + 1, end_line, total_lines)
    } else {
        String::new()
    };

    let strategy_info = if app.is_recommendation_mode {
        " [Recommendation Mode] ".to_string()
    } else {
        format!(" [Strategy: {}] ", app.current_strategy.name())
    };

    let focus_indicator = if matches!(app.focused_pane, FocusedPane::QueryDetails) {
        " [FOCUSED - Tab to switch] "
    } else {
        ""
    };

    let border_style = if matches!(app.focused_pane, FocusedPane::QueryDetails) {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let sql = Paragraph::new(visible_text)
        .style(Style::default().fg(if app.is_recommendation_mode {
            Color::Cyan
        } else {
            Color::Green
        }))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title)
                .title(strategy_info)
                .title(scroll_indicator)
                .title(focus_indicator),
        )
        .wrap(Wrap { trim: false });

    f.render_widget(sql, area);
}

fn draw_summary(f: &mut Frame, app: &App, area: Rect) {
    let summary_text = app
        .natural_language_summary
        .clone()
        .unwrap_or_else(|| "Query results will appear below".to_string());

    let summary = Paragraph::new(summary_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL).title("Summary"))
        .wrap(Wrap { trim: false });

    f.render_widget(summary, area);
}

fn draw_results(f: &mut Frame, app: &App, area: Rect) {
    if let Some(result) = &app.query_result {
        // Calculate visible window
        // Subtract 3 for borders and header
        let visible_height = area.height.saturating_sub(3) as usize;
        let start_row = app.scroll_offset;
        let end_row = (start_row + visible_height).min(result.row_count);

        let header_cells = result.columns.iter().map(|h| {
            ratatui::widgets::Cell::from(h.as_str()).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        });
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        // Only render visible rows
        let rows = result
            .rows
            .iter()
            .skip(start_row)
            .take(visible_height)
            .map(|row| {
                let cells = row.iter().map(|c| {
                    let truncated = if c.len() > 100 {
                        format!("{}...", &c[..100])
                    } else {
                        c.clone()
                    };
                    ratatui::widgets::Cell::from(truncated)
                });
                Row::new(cells).height(1)
            });

        let widths: Vec<Constraint> = result
            .columns
            .iter()
            .map(|_| Constraint::Percentage((100 / result.columns.len().max(1)) as u16))
            .collect();

        // Create scroll indicator
        let scroll_indicator = if result.row_count > visible_height {
            format!(
                " [{}-{}/{}] ↑↓ to scroll ",
                start_row + 1,
                end_row,
                result.row_count
            )
        } else {
            format!(" [All {} rows shown] ", result.row_count)
        };

        let focus_indicator = if matches!(app.focused_pane, FocusedPane::Results) {
            " [FOCUSED - Tab to switch] "
        } else {
            ""
        };

        let border_style = if matches!(app.focused_pane, FocusedPane::Results) {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(format!("Results ({} rows)", result.row_count))
                    .title(scroll_indicator)
                    .title(focus_indicator),
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(table, area);
    } else {
        let placeholder =
            Paragraph::new("No results yet. Enter a question to search the fly database!")
                .style(Style::default().fg(Color::DarkGray))
                .block(Block::default().borders(Borders::ALL).title("Results"))
                .wrap(Wrap { trim: false });

        f.render_widget(placeholder, area);
    }
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let status_color = if app.status_message.starts_with("Error") {
        Color::Red
    } else if app.is_loading {
        Color::Yellow
    } else {
        Color::Green
    };

    let status = Paragraph::new(app.status_message.as_str())
        .style(Style::default().fg(status_color))
        .block(Block::default().borders(Borders::ALL).title("Status"));

    f.render_widget(status, area);
}

pub fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    loop {
        terminal.draw(|f| draw(f, app))?;

        if app.should_quit {
            return Ok(());
        }

        if let Event::Key(key) = event::read()? {
            app.handle_input(key);

            if matches!(app.input_mode, InputMode::Normal) && !app.input.is_empty() {
                return Ok(());
            }
        }
    }
}
