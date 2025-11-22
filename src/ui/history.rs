use anyhow::Result;
use chrono::{Local, Utc};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
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
    table_state: TableState,
    show_filters: bool,
    user_host_string: String,
}

impl SessionHistoryBrowser {
    pub async fn new() -> Result<Self> {
        let db_path = get_database_path()?;
        let db = Database::new(&db_path)?;

        // Load recent sessions (last 100)
        let sessions =
            SessionQueries::list_with_filter(&db.connection, None, None, None, Some(100))?;

        let mut table_state = TableState::default();
        if !sessions.is_empty() {
            table_state.select(Some(0));
        }

        // Get user@machine string
        let user = std::process::Command::new("git")
            .args(["config", "user.name"])
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    None
                }
            })
            .or_else(|| std::env::var("USER").ok())
            .unwrap_or_else(|| "user".to_string());

        let host = std::process::Command::new("hostname")
            .output()
            .ok()
            .and_then(|output| {
                if output.status.success() {
                    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
                } else {
                    None
                }
            })
            .or_else(|| std::env::var("HOSTNAME").ok())
            .unwrap_or_else(|| "machine".to_string());

        let user_host_string = format!("{}@{}", user, host);

        Ok(Self {
            sessions,
            table_state,
            show_filters: false,
            user_host_string,
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
                                let i = match self.table_state.selected() {
                                    Some(i) => {
                                        if i == 0 {
                                            self.sessions.len() - 1
                                        } else {
                                            i - 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.table_state.select(Some(i));
                            }
                            KeyCode::Down => {
                                let i = match self.table_state.selected() {
                                    Some(i) => {
                                        if i >= self.sessions.len() - 1 {
                                            0
                                        } else {
                                            i + 1
                                        }
                                    }
                                    None => 0,
                                };
                                self.table_state.select(Some(i));
                            }
                            KeyCode::Char('f') => {
                                self.show_filters = !self.show_filters;
                            }
                            KeyCode::Enter => {
                                // Could toggle details expansion or similar
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

    fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Header
                Constraint::Length(3), // Filter Bar
                Constraint::Min(0),    // Main Content (Table + Details)
                Constraint::Length(1), // Footer
            ])
            .split(f.size());

        // 0. Header
        self.render_header(f, chunks[0]);

        // 1. Filter Bar
        self.render_filter_bar(f, chunks[1]);

        // 2. Main Content
        self.render_main_content(f, chunks[2]);

        // 3. Footer
        self.render_footer(f, chunks[3]);
    }

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Title
                Constraint::Percentage(50), // User@Host
            ])
            .split(area);

        f.render_widget(
            Paragraph::new("Tempo TUI :: History Browser").style(
                Style::default()
                    .fg(ColorScheme::WHITE_TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
            chunks[0],
        );

        f.render_widget(
            Paragraph::new(self.user_host_string.as_str())
                .alignment(Alignment::Right)
                .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
            chunks[1],
        );
    }

    fn render_filter_bar(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ColorScheme::GRAY_TEXT))
            .title(" Filters ")
            .style(Style::default().bg(ColorScheme::CLEAN_BG));

        let filter_text = if self.show_filters {
            "Project: [All]  Date: [Any]  Duration: [Any]"
        } else {
            "Press 'f' to show filters"
        };

        let p = Paragraph::new(filter_text)
            .block(block)
            .style(Style::default().fg(ColorScheme::GRAY_TEXT));

        f.render_widget(p, area);
    }

    fn render_main_content(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(70), // Table
                Constraint::Percentage(30), // Details
            ])
            .split(area);

        // Table
        self.render_session_table(f, chunks[0]);

        // Details
        self.render_details_panel(f, chunks[1]);
    }

    fn render_session_table(&mut self, f: &mut Frame, area: Rect) {
        let header_cells = ["ID", "Date", "Project", "Duration", "Status"]
            .iter()
            .map(|h| {
                Cell::from(*h).style(
                    Style::default()
                        .fg(ColorScheme::GRAY_TEXT)
                        .add_modifier(Modifier::BOLD),
                )
            });

        let header = Row::new(header_cells).height(1).bottom_margin(1);

        let rows = self.sessions.iter().map(|session| {
            let duration = if let Some(end) = session.end_time {
                (end - session.start_time).num_seconds() - session.paused_duration.num_seconds()
            } else {
                (Utc::now() - session.start_time).num_seconds()
                    - session.paused_duration.num_seconds()
            };

            let start_time = session.start_time.with_timezone(&Local);
            let date_str = start_time.format("%Y-%m-%d").to_string();
            let duration_str = Formatter::format_duration(duration);
            let status = if session.end_time.is_none() {
                "Active"
            } else {
                "Done"
            };

            // Placeholder for project name until we join with projects
            let project_str = format!("Project {}", session.project_id);

            let cells = vec![
                Cell::from(session.id.unwrap_or(0).to_string()),
                Cell::from(date_str),
                Cell::from(project_str),
                Cell::from(duration_str),
                Cell::from(status),
            ];

            Row::new(cells).height(1)
        });

        let table = Table::new(rows)
            .widths(&[
                Constraint::Length(5),
                Constraint::Length(12),
                Constraint::Min(20),
                Constraint::Length(10),
                Constraint::Length(8),
            ])
            .header(header)
            .block(Block::default().borders(Borders::ALL).title(" Sessions "))
            .highlight_style(
                Style::default()
                    .bg(ColorScheme::CLEAN_BLUE)
                    .fg(ColorScheme::CLEAN_BG)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_stateful_widget(table, area, &mut self.table_state);
    }

    fn render_details_panel(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Details ")
            .style(Style::default().bg(ColorScheme::CLEAN_BG));

        f.render_widget(block.clone(), area);

        let inner_area = block.inner(area);

        if let Some(selected_index) = self.table_state.selected() {
            if let Some(session) = self.sessions.get(selected_index) {
                let start_time = session.start_time.with_timezone(&Local);
                let end_time_str = if let Some(end) = session.end_time {
                    end.with_timezone(&Local).format("%H:%M:%S").to_string()
                } else {
                    "Now".to_string()
                };

                let details = vec![
                    Line::from(Span::styled(
                        "Context:",
                        Style::default().fg(ColorScheme::GRAY_TEXT),
                    )),
                    Line::from(Span::styled(
                        session.context.to_string(),
                        Style::default().fg(ColorScheme::WHITE_TEXT),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Start Time:",
                        Style::default().fg(ColorScheme::GRAY_TEXT),
                    )),
                    Line::from(Span::styled(
                        start_time.format("%H:%M:%S").to_string(),
                        Style::default().fg(ColorScheme::WHITE_TEXT),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "End Time:",
                        Style::default().fg(ColorScheme::GRAY_TEXT),
                    )),
                    Line::from(Span::styled(
                        end_time_str,
                        Style::default().fg(ColorScheme::WHITE_TEXT),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Notes:",
                        Style::default().fg(ColorScheme::GRAY_TEXT),
                    )),
                    Line::from(Span::styled(
                        session.notes.clone().unwrap_or("-".to_string()),
                        Style::default().fg(ColorScheme::WHITE_TEXT),
                    )),
                ];

                f.render_widget(
                    Paragraph::new(details).wrap(ratatui::widgets::Wrap { trim: true }),
                    inner_area,
                );
            }
        } else {
            f.render_widget(
                Paragraph::new("Select a session to view details")
                    .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
                inner_area,
            );
        }
    }

    fn render_footer(&self, f: &mut Frame, area: Rect) {
        let help_text = "[↑/↓] Navigate  [F] Filter  [Enter] Details  [Q] Quit";
        f.render_widget(
            Paragraph::new(help_text)
                .alignment(Alignment::Center)
                .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
            area,
        );
    }
}
