use crate::db::QueryResult;
use crate::openai::PromptStrategy;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Row, Table, Wrap},
    Frame, Terminal,
};
use std::io;

pub enum InputMode {
    Normal,
    Editing,
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
}

impl App {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            input_mode: InputMode::Normal,
            current_strategy: PromptStrategy::Basic,
            generated_sql: None,
            query_result: None,
            status_message: "Ready. Press 'i' to enter a query, Tab to change strategy, 'q' to quit".to_string(),
            natural_language_summary: None,
            should_quit: false,
            is_loading: false,
        }
    }

    pub fn cycle_strategy(&mut self) {
        self.current_strategy = match self.current_strategy {
            PromptStrategy::Basic => PromptStrategy::Intermediate,
            PromptStrategy::Intermediate => PromptStrategy::Advanced,
            PromptStrategy::Advanced => PromptStrategy::Basic,
        };
        self.status_message = format!("Strategy changed to: {}", self.current_strategy.name());
    }

    pub fn handle_input(&mut self, key: KeyEvent) {
        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Char('q') => {
                    self.should_quit = true;
                }
                KeyCode::Char('i') => {
                    self.input_mode = InputMode::Editing;
                    self.status_message = "Editing mode - Enter your question, press Enter to submit, Esc to cancel".to_string();
                }
                KeyCode::Tab => {
                    self.cycle_strategy();
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
                    }
                }
                KeyCode::Backspace => {
                    self.input.pop();
                }
                KeyCode::Esc => {
                    self.input_mode = InputMode::Normal;
                    self.status_message = "Cancelled. Press 'i' to try again".to_string();
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
        self.status_message = format!(
            "Success! Found {} result{}",
            result.row_count,
            if result.row_count == 1 { "" } else { "s" }
        );
    }

    pub fn clear_input(&mut self) {
        self.input.clear();
    }
}

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Input
            Constraint::Length(8), // SQL display
            Constraint::Length(3), // Summary
            Constraint::Min(10),   // Results table
            Constraint::Length(3), // Status bar
        ])
        .split(f.area());

    draw_input(f, app, chunks[0]);
    draw_sql(f, app, chunks[1]);
    draw_summary(f, app, chunks[2]);
    draw_results(f, app, chunks[3]);
    draw_status(f, app, chunks[4]);
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
    let sql_text = if app.is_loading {
        "Generating SQL...".to_string()
    } else {
        app.generated_sql.clone().unwrap_or_else(|| "No query yet".to_string())
    };

    let strategy_info = format!(" [Strategy: {}] ", app.current_strategy.name());

    let sql = Paragraph::new(sql_text)
        .style(Style::default().fg(Color::Green))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Generated SQL")
                .title(strategy_info)
        )
        .wrap(Wrap { trim: false });

    f.render_widget(sql, area);
}

fn draw_summary(f: &mut Frame, app: &App, area: Rect) {
    let summary_text = app.natural_language_summary
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
        let header_cells = result.columns.iter().map(|h| {
            ratatui::widgets::Cell::from(h.as_str())
                .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
        });
        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let rows = result.rows.iter().map(|row| {
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

        let table = Table::new(rows, widths)
            .header(header)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!("Results ({} rows)", result.row_count))
            )
            .style(Style::default().fg(Color::White));

        f.render_widget(table, area);
    } else {
        let placeholder = Paragraph::new("No results yet. Enter a question to search the fly database!")
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
