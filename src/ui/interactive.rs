use anyhow::Result;
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame, Terminal,
};
use std::time::Duration;

use crate::{
    models::{Project, Session},
    ui::formatter::Formatter,
    utils::ipc::IpcClient,
};

pub struct InteractiveViewer {
    client: IpcClient,
    projects: Vec<Project>,
    sessions: Vec<Session>,
    selected_project: Option<usize>,
    project_list_state: ListState,
}

impl InteractiveViewer {
    pub fn new() -> Result<Self> {
        let client = IpcClient::new()?;
        let mut viewer = Self {
            client,
            projects: Vec::new(),
            sessions: Vec::new(),
            selected_project: None,
            project_list_state: ListState::default(),
        };
        
        viewer.load_data()?;
        Ok(viewer)
    }

    pub fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            terminal.draw(|f| {
                self.render(f);
            })?;

            // Handle input
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Up => self.previous_project(),
                        KeyCode::Down => self.next_project(),
                        KeyCode::Enter => self.select_project(),
                        KeyCode::Char('r') => self.load_data()?,
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    fn render(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(f.size());

        self.render_project_list(f, chunks[0]);
        self.render_session_details(f, chunks[1]);
    }

    fn render_project_list(&mut self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .projects
            .iter()
            .enumerate()
            .map(|(i, project)| {
                let style = if Some(i) == self.selected_project {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                ListItem::new(Line::from(Span::styled(project.name.clone(), style)))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title("Projects")
                    .borders(Borders::ALL)
                    .style(Style::default().fg(Color::Cyan)),
            )
            .highlight_style(Style::default().bg(Color::DarkGray));

        f.render_stateful_widget(list, area, &mut self.project_list_state);
    }

    fn render_session_details(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(area);

        // Project info
        if let Some(selected_idx) = self.selected_project {
            if let Some(project) = self.projects.get(selected_idx) {
                let project_info = Formatter::format_project_info(project);
                let paragraph = Paragraph::new(project_info)
                    .block(Formatter::create_header_block("Project Details"));
                f.render_widget(paragraph, chunks[0]);

                // Sessions for this project
                let project_sessions: Vec<&Session> = self
                    .sessions
                    .iter()
                    .filter(|s| s.project_id == project.id.unwrap_or(-1))
                    .collect();

                if !project_sessions.is_empty() {
                    let sessions_summary = Formatter::format_sessions_summary(&project_sessions.into_iter().cloned().collect::<Vec<_>>());
                    let sessions_widget = Paragraph::new(sessions_summary)
                        .block(Formatter::create_info_block());
                    f.render_widget(sessions_widget, chunks[1]);
                } else {
                    let no_sessions = Paragraph::new("No sessions found for this project")
                        .style(Style::default().fg(Color::Gray))
                        .block(Formatter::create_info_block());
                    f.render_widget(no_sessions, chunks[1]);
                }
            }
        } else {
            let help_text = vec![
                Line::from("Select a project to view details"),
                Line::from(""),
                Line::from("Controls:"),
                Line::from("  ↑/↓  Navigate projects"),
                Line::from("  Enter  Select project"),
                Line::from("  r      Refresh data"),
                Line::from("  q/Esc  Quit"),
            ];

            let paragraph = Paragraph::new(help_text)
                .block(
                    Block::default()
                        .title("Help")
                        .borders(Borders::ALL)
                        .style(Style::default().fg(Color::Cyan)),
                );
            f.render_widget(paragraph, area);
        }
    }

    fn previous_project(&mut self) {
        if self.projects.is_empty() {
            return;
        }

        let i = match self.project_list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.projects.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.project_list_state.select(Some(i));
        self.selected_project = Some(i);
    }

    fn next_project(&mut self) {
        if self.projects.is_empty() {
            return;
        }

        let i = match self.project_list_state.selected() {
            Some(i) => {
                if i >= self.projects.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.project_list_state.select(Some(i));
        self.selected_project = Some(i);
    }

    fn select_project(&mut self) {
        if let Some(i) = self.project_list_state.selected() {
            self.selected_project = Some(i);
        }
    }

    fn load_data(&mut self) -> Result<()> {
        // This would use IPC to load actual data
        // For now, create placeholder data
        use chrono::Utc;
        use std::path::PathBuf;
        
        self.projects = vec![
            Project {
                id: Some(1),
                name: "Sample Project".to_string(),
                path: PathBuf::from("/Users/example/sample"),
                git_hash: Some("abc123".to_string()),
                created_at: chrono::Local::now().with_timezone(&Utc),
                updated_at: chrono::Local::now().with_timezone(&Utc),
                is_archived: false,
                description: Some("A sample project for demo".to_string()),
            },
        ];

        use crate::models::session::SessionContext;
        
        self.sessions = vec![
            Session {
                id: Some(1),
                project_id: 1,
                start_time: (chrono::Local::now() - chrono::Duration::hours(2)).with_timezone(&Utc),
                end_time: Some((chrono::Local::now() - chrono::Duration::hours(1)).with_timezone(&Utc)),
                context: SessionContext::Terminal,
                paused_duration: chrono::Duration::minutes(5),
                notes: Some("Working on initial setup".to_string()),
                created_at: chrono::Local::now().with_timezone(&Utc),
            },
        ];

        Ok(())
    }
}