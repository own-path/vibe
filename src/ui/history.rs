use anyhow::Result;
use chrono::{Local, Utc};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::time::Duration;

use crate::{
    db::queries::SessionQueries,
    db::{get_database_path, Database},
    models::Session,
    ui::{formatter::Formatter, widgets::ColorScheme},
};

pub struct SessionHistoryBrowser {
    sessions: Vec<Session>,
    selected_index: usize,
    filter_project: Option<String>,
    filter_date_from: Option<chrono::NaiveDate>,
    filter_date_to: Option<chrono::NaiveDate>,
    show_filters: bool,
}

impl SessionHistoryBrowser {
    pub async fn new() -> Result<Self> {
        let db_path = get_database_path()?;
        let db = Database::new(&db_path)?;

        // Load recent sessions (last 100)
        let sessions =
            SessionQueries::list_with_filter(&db.connection, None, None, None, Some(100))?;

        Ok(Self {
            sessions,
            selected_index: 0,
            filter_project: None,
            filter_date_from: None,
            filter_date_to: None,
            show_filters: false,
        })
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => break,
                            KeyCode::Up => {
                                if self.selected_index > 0 {
                                    self.selected_index -= 1;
                                }
                            }
                            KeyCode::Down => {
                                if self.selected_index < self.sessions.len().saturating_sub(1) {
                                    self.selected_index += 1;
                                }
                            }
                            KeyCode::Char('f') => {
                                self.show_filters = !self.show_filters;
                            }
                            KeyCode::Enter => {
                                // Could show session details here
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn render(&self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Title
                Constraint::Min(0),    // Session list
                Constraint::Length(3), // Help
            ])
            .split(f.size());

        // Title
        // Title
        let title = Paragraph::new("Session History Browser")
            .style(
                Style::default()
                    .fg(ColorScheme::CLEAN_ACCENT)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(ColorScheme::clean_block());
        f.render_widget(title, chunks[0]);

        // Session list
        // Session list
        if self.sessions.is_empty() {
            let no_sessions = Paragraph::new("No sessions found")
                .style(Style::default().fg(ColorScheme::CLEAN_GOLD))
                .alignment(Alignment::Center)
                .block(ColorScheme::clean_block().title("Sessions"));
            f.render_widget(no_sessions, chunks[1]);
        } else {
            let session_items: Vec<ListItem> = self
                .sessions
                .iter()
                .enumerate()
                .map(|(i, session)| {
                    let style = if i == self.selected_index {
                        Style::default()
                            .fg(ColorScheme::CLEAN_BG)
                            .bg(ColorScheme::CLEAN_ACCENT)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(ColorScheme::WHITE_TEXT)
                    };

                    let duration = if let Some(end) = session.end_time {
                        (end - session.start_time).num_seconds()
                            - session.paused_duration.num_seconds()
                    } else {
                        (Utc::now() - session.start_time).num_seconds()
                            - session.paused_duration.num_seconds()
                    };

                    let context_color = if i == self.selected_index {
                        ColorScheme::CLEAN_BG
                    } else {
                        match session.context.to_string().as_str() {
                            "terminal" => ColorScheme::CLEAN_ACCENT,
                            "ide" => ColorScheme::CLEAN_BLUE,
                            "linked" => ColorScheme::CLEAN_GOLD,
                            "manual" => ColorScheme::CLEAN_GREEN,
                            _ => ColorScheme::WHITE_TEXT,
                        }
                    };

                    let start_time = session.start_time.with_timezone(&Local);
                    let duration_color = if i == self.selected_index {
                        ColorScheme::CLEAN_BG
                    } else {
                        ColorScheme::CLEAN_GREEN
                    };
                    let date_color = if i == self.selected_index {
                        ColorScheme::CLEAN_BG
                    } else {
                        ColorScheme::GRAY_TEXT
                    };

                    let content = vec![
                        Line::from(vec![
                            Span::styled(format!("Session #{}", session.id.unwrap_or(0)), style),
                            Span::raw("  "),
                            Span::styled(
                                Formatter::format_duration(duration),
                                Style::default().fg(duration_color),
                            ),
                        ]),
                        Line::from(vec![
                            Span::styled(
                                format!("{}", start_time.format("%Y-%m-%d %H:%M:%S")),
                                Style::default().fg(date_color),
                            ),
                            Span::raw("  "),
                            Span::styled(
                                session.context.to_string(),
                                Style::default().fg(context_color),
                            ),
                        ]),
                    ];

                    ListItem::new(content).style(style)
                })
                .collect();

            let sessions_list = List::new(session_items)
                .block(ColorScheme::clean_block().title("Sessions"))
                .style(Style::default().fg(ColorScheme::WHITE_TEXT));
            f.render_widget(sessions_list, chunks[1]);
        }

        // Help
        let help_text = if self.show_filters {
            "Filters: [f] Toggle | [q/Esc] Quit"
        } else {
            "[Up/Down] Navigate | [Enter] View Details | [f] Filters | [q/Esc] Quit"
        };

        let help = Paragraph::new(help_text)
            .style(Style::default().fg(ColorScheme::GRAY_TEXT))
            .alignment(Alignment::Center)
            .block(ColorScheme::clean_block());
        f.render_widget(help, chunks[2]);
    }
}
