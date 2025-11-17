use anyhow::Result;
use chrono::Local;
use crossterm::event;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Wrap},
    Frame, Terminal,
};
use std::time::Duration;

use crate::{
    models::{Project, Session},
    ui::{formatter::Formatter, should_quit},
    utils::ipc::IpcClient,
};

pub struct Dashboard {
    client: IpcClient,
}

impl Dashboard {
    pub fn new() -> Result<Self> {
        // For now, we'll create a placeholder
        // In a real implementation, this would connect to the daemon via socket
        Ok(Self { 
            client: IpcClient::new()?
        })
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Get current status
            let current_session = self.get_current_session()?;
            let current_project = if let Some(ref session) = current_session {
                self.get_project_by_session(session)?
            } else {
                None
            };

            terminal.draw(|f| {
                self.render_dashboard(f, &current_session, &current_project);
            })?;

            // Handle input
            if event::poll(Duration::from_millis(100))? {
                if should_quit(event::read()?) {
                    break;
                }
            }
        }

        Ok(())
    }

    fn render_dashboard(
        &self,
        f: &mut Frame,
        current_session: &Option<Session>,
        current_project: &Option<Project>,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(8),  // Current session info
                Constraint::Length(6),  // Project info
                Constraint::Min(0),     // Statistics
                Constraint::Length(3),  // Help
            ])
            .split(f.size());

        // Title
        let title = Paragraph::new("Vibe - Time Tracking Dashboard")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        // Current session info
        self.render_session_info(f, chunks[1], current_session);

        // Project info
        self.render_project_info(f, chunks[2], current_project);

        // Statistics
        self.render_statistics(f, chunks[3]);

        // Help
        self.render_help(f, chunks[4]);
    }

    fn render_session_info(&self, f: &mut Frame, area: Rect, session: &Option<Session>) {
        let block = Block::default()
            .title("Current Session")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White));

        if let Some(session) = session {
            let now = Local::now();
            let elapsed_seconds = (now.timestamp() - session.start_time.timestamp()) 
                - session.paused_duration.num_seconds();
            
            let status_text = vec![
                Line::from(vec![
                    Span::raw("Status: "),
                    Span::styled("Active", Formatter::create_success_style()),
                ]),
                Line::from(vec![
                    Span::raw("Started: "),
                    Span::styled(
                        Formatter::format_timestamp(&session.start_time.with_timezone(&Local)),
                        Style::default().fg(Color::White)
                    ),
                ]),
                Line::from(vec![
                    Span::raw("Elapsed: "),
                    Span::styled(
                        Formatter::format_duration(elapsed_seconds),
                        Formatter::create_highlight_style()
                    ),
                ]),
                Line::from(vec![
                    Span::raw("Context: "),
                    Span::styled(
                        session.context.to_string(),
                        Style::default().fg(Color::Yellow)
                    ),
                ]),
            ];

            let paragraph = Paragraph::new(status_text)
                .block(block)
                .wrap(Wrap { trim: true });
            f.render_widget(paragraph, area);
        } else {
            let no_session_text = vec![
                Line::from(Span::styled(
                    "No active session",
                    Formatter::create_warning_style()
                )),
                Line::from(Span::raw("")),
                Line::from(Span::raw("Use 'vibe start' to begin tracking time")),
            ];

            let paragraph = Paragraph::new(no_session_text)
                .block(block)
                .wrap(Wrap { trim: true });
            f.render_widget(paragraph, area);
        }
    }

    fn render_project_info(&self, f: &mut Frame, area: Rect, project: &Option<Project>) {
        let block = Block::default()
            .title("Current Project")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White));

        if let Some(project) = project {
            let project_text = vec![
                Line::from(vec![
                    Span::raw("Name: "),
                    Span::styled(project.name.clone(), Formatter::create_highlight_style()),
                ]),
                Line::from(vec![
                    Span::raw("Path: "),
                    Span::styled(project.path.to_string_lossy().to_string(), Style::default().fg(Color::Gray)),
                ]),
            ];

            let paragraph = Paragraph::new(project_text)
                .block(block)
                .wrap(Wrap { trim: true });
            f.render_widget(paragraph, area);
        } else {
            let no_project_text = vec![
                Line::from(Span::styled(
                    "No active project",
                    Formatter::create_warning_style()
                )),
            ];

            let paragraph = Paragraph::new(no_project_text)
                .block(block)
                .wrap(Wrap { trim: true });
            f.render_widget(paragraph, area);
        }
    }

    fn render_statistics(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Today's Summary")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White));

        // This would be expanded with actual statistics
        let stats_text = vec![
            Line::from(Span::raw("Coming soon: Daily statistics, weekly summaries, and more...")),
        ];

        let paragraph = Paragraph::new(stats_text)
            .block(block)
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }

    fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_text = Paragraph::new("Press 'q' or 'Esc' to quit | Updates every 100ms")
            .style(Style::default().fg(Color::Gray))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(help_text, area);
    }

    fn get_current_session(&mut self) -> Result<Option<Session>> {
        // This would use IPC to get current session
        // For now, return None as placeholder
        Ok(None)
    }

    fn get_project_by_session(&mut self, _session: &Session) -> Result<Option<Project>> {
        // This would use IPC to get project info
        // For now, return None as placeholder
        Ok(None)
    }
}