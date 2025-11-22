use anyhow::Result;
use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use log::debug;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::time::Duration;

use crate::{
    db::queries::ProjectQueries,
    db::{get_database_path, Database},
    models::{Project, Session},
    ui::formatter::Formatter,
    ui::widgets::{ColorScheme, Spinner, Throbber},
    utils::ipc::{get_socket_path, is_daemon_running, IpcClient, IpcMessage, IpcResponse},
};

pub struct Dashboard {
    client: IpcClient,
    show_project_switcher: bool,
    available_projects: Vec<Project>,
    selected_project_index: usize,
    spinner: Spinner,
    throbber: Throbber,
}

impl Dashboard {
    pub async fn new() -> Result<Self> {
        let socket_path = get_socket_path()?;
        let client = if socket_path.exists() {
            match IpcClient::connect(&socket_path).await {
                Ok(client) => client,
                Err(_) => IpcClient::new()?,
            }
        } else {
            IpcClient::new()?
        };

        Ok(Self {
            client,
            show_project_switcher: false,
            available_projects: Vec::new(),
            selected_project_index: 0,
            spinner: Spinner::new(),
            throbber: Throbber::new(),
        })
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        let mut heartbeat_counter = 0;

        loop {
            // Send activity heartbeat every 30 iterations (3 seconds at 100ms intervals)
            if heartbeat_counter >= 30 {
                if let Err(e) = self.send_activity_heartbeat().await {
                    // Ignore heartbeat errors to avoid interrupting the dashboard
                    debug!("Heartbeat error: {}", e);
                }
                heartbeat_counter = 0;
            }
            heartbeat_counter += 1;

            // Tick animations
            self.spinner.next();
            self.throbber.next();

            // Get current status
            let current_session = self.get_current_session().await?;
            let current_project = if let Some(ref session) = current_session {
                self.get_project_by_session(session).await?
            } else {
                None
            };
            let daily_stats = self.get_today_stats().await.unwrap_or((0, 0, 0));
            let session_metrics = self.get_session_metrics().await.unwrap_or(None);

            terminal.draw(|f| {
                self.render_dashboard_sync(
                    f,
                    &current_session,
                    &current_project,
                    &daily_stats,
                    &session_metrics,
                );
            })?;

            // Handle input
            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            if self.show_project_switcher {
                                self.show_project_switcher = false;
                            } else {
                                break;
                            }
                        }
                        KeyCode::Char('p') => {
                            self.toggle_project_switcher().await?;
                        }
                        KeyCode::Up => {
                            if self.show_project_switcher {
                                self.navigate_projects(-1);
                            }
                        }
                        KeyCode::Down => {
                            if self.show_project_switcher {
                                self.navigate_projects(1);
                            }
                        }
                        KeyCode::Enter => {
                            if self.show_project_switcher {
                                self.switch_to_selected_project().await?;
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn render_dashboard_sync(
        &self,
        f: &mut Frame,
        current_session: &Option<Session>,
        current_project: &Option<Project>,
        daily_stats: &(i64, i64, i64),
        session_metrics: &Option<crate::utils::ipc::SessionMetrics>,
    ) {
        // Modern clean layout: Minimal, focused, professional
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Top bar (Logo/Status)
                Constraint::Length(1), // Spacer
                Constraint::Length(7), // Main Status / Activity Stream (reduced from 10)
                Constraint::Length(1), // Spacer
                Constraint::Min(0),    // Details / Metrics (flexible)
                Constraint::Length(1), // Bottom bar (Input/Help)
            ])
            .split(f.size());

        // Top Bar
        self.render_top_bar(f, chunks[0]);

        // Main Status Area (The "Stream")
        self.render_main_status(f, chunks[2], current_session, current_project);

        // Details / Metrics Area
        self.render_metrics_area(f, chunks[4], daily_stats, session_metrics);

        // Bottom Bar
        self.render_bottom_bar(f, chunks[5]);

        // Project switcher overlay
        if self.show_project_switcher {
            self.render_project_switcher(f, f.size());
        }
    }

    fn render_bottom_bar(&self, f: &mut Frame, area: Rect) {
        let help_text = if self.show_project_switcher {
            vec![
                Span::styled(
                    "[Q]",
                    Style::default()
                        .fg(ColorScheme::CLEAN_ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Close  "),
                Span::raw("[↑/↓] Navigate  "),
                Span::raw("[Enter] Select"),
            ]
        } else {
            vec![
                Span::styled(
                    "[Q]",
                    Style::default()
                        .fg(ColorScheme::CLEAN_ACCENT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" Quit  "),
                Span::raw("[P] Projects  "),
                Span::raw("[R] Refresh"),
            ]
        };

        let help_paragraph = Paragraph::new(Line::from(help_text))
            .alignment(Alignment::Center)
            .style(Style::default().fg(ColorScheme::GRAY_TEXT));

        f.render_widget(help_paragraph, area);
    }

    fn render_top_bar(&self, f: &mut Frame, area: Rect) {
        let title_text = vec![
            Span::styled(
                "Tempo",
                Style::default()
                    .fg(ColorScheme::CLEAN_ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled("CLI", Style::default().fg(ColorScheme::GRAY_TEXT)),
        ];

        let title = Paragraph::new(Line::from(title_text)).alignment(Alignment::Left);

        f.render_widget(title, area);

        // Right side status
        let status_text = if is_daemon_running() {
            Span::styled(
                "Daemon Active",
                Style::default().fg(ColorScheme::CLEAN_GREEN),
            )
        } else {
            Span::styled(
                "Daemon Offline",
                Style::default().fg(ColorScheme::NEON_PINK),
            )
        };

        let status = Paragraph::new(Line::from(status_text)).alignment(Alignment::Right);

        f.render_widget(status, area);
    }

    fn render_main_status(
        &self,
        f: &mut Frame,
        area: Rect,
        session: &Option<Session>,
        project: &Option<Project>,
    ) {
        let block = ColorScheme::clean_block();

        if let Some(session) = session {
            let now = Local::now();
            let elapsed_seconds = (now.timestamp() - session.start_time.timestamp())
                - session.paused_duration.num_seconds();

            let project_name = project
                .as_ref()
                .map(|p| p.name.as_str())
                .unwrap_or("Unknown Project");

            let status_lines = vec![
                Line::from(vec![
                    Span::styled(
                        session.context.to_string(),
                        Style::default().fg(ColorScheme::CLEAN_ACCENT),
                    ),
                    Span::styled(
                        "Tracking ",
                        Style::default()
                            .fg(ColorScheme::CLEAN_BLUE)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(project_name, Style::default().fg(ColorScheme::WHITE_TEXT)),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled("L ", Style::default().fg(ColorScheme::GRAY_TEXT)),
                    Span::raw("Context: "),
                    Span::styled(
                        session.context.to_string(),
                        Style::default().fg(ColorScheme::CLEAN_ACCENT),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled("L ", Style::default().fg(ColorScheme::GRAY_TEXT)),
                    Span::raw("Duration: "),
                    Span::styled(
                        Formatter::format_duration(elapsed_seconds),
                        Style::default().fg(ColorScheme::CLEAN_GREEN),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        self.throbber.current(),
                        Style::default().fg(ColorScheme::GRAY_TEXT),
                    ),
                ]),
            ];

            let paragraph = Paragraph::new(status_lines).block(block);
            f.render_widget(paragraph, area);
        } else {
            let idle_lines = vec![
                Line::from(vec![
                    Span::styled("- ", Style::default().fg(ColorScheme::GRAY_TEXT)),
                    Span::styled("Idle", Style::default().fg(ColorScheme::GRAY_TEXT)),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled("L ", Style::default().fg(ColorScheme::GRAY_TEXT)),
                    Span::raw("Waiting for command..."),
                ]),
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled("L ", Style::default().fg(ColorScheme::GRAY_TEXT)),
                    Span::raw("Try: "),
                    Span::styled(
                        "tempo start <project>",
                        Style::default().fg(ColorScheme::CLEAN_ACCENT),
                    ),
                ]),
            ];

            let paragraph = Paragraph::new(idle_lines).block(block);
            f.render_widget(paragraph, area);
        }
    }

    fn render_metrics_area(
        &self,
        f: &mut Frame,
        area: Rect,
        daily_stats: &(i64, i64, i64),
        session_metrics: &Option<crate::utils::ipc::SessionMetrics>,
    ) {
        let (sessions_count, total_seconds, avg_seconds) = *daily_stats;

        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Daily Stats Column
        let stats_lines = vec![
            Line::from(Span::styled(
                "Daily Summary",
                Style::default()
                    .fg(ColorScheme::GRAY_TEXT)
                    .add_modifier(Modifier::UNDERLINED),
            )),
            Line::from(""),
            Line::from(vec![
                Span::raw("Sessions: "),
                Span::styled(
                    sessions_count.to_string(),
                    Style::default().fg(ColorScheme::WHITE_TEXT),
                ),
            ]),
            Line::from(vec![
                Span::raw("Total:    "),
                Span::styled(
                    Formatter::format_duration(total_seconds),
                    Style::default().fg(ColorScheme::WHITE_TEXT),
                ),
            ]),
            Line::from(vec![
                Span::raw("Average:  "),
                Span::styled(
                    Formatter::format_duration(avg_seconds),
                    Style::default().fg(ColorScheme::WHITE_TEXT),
                ),
            ]),
        ];

        f.render_widget(
            Paragraph::new(stats_lines).block(ColorScheme::clean_block()),
            chunks[0],
        );

        // Session Metrics Column
        if let Some(metrics) = session_metrics {
            let efficiency = self.calculate_efficiency_percentage(metrics);
            let activity_score = metrics.activity_score * 100.0;

            let metrics_lines = vec![
                Line::from(Span::styled(
                    "Current Session",
                    Style::default()
                        .fg(ColorScheme::GRAY_TEXT)
                        .add_modifier(Modifier::UNDERLINED),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::raw("Activity:   "),
                    Span::styled(
                        format!("{:.0}%", activity_score),
                        Style::default().fg(if activity_score > 80.0 {
                            ColorScheme::CLEAN_GREEN
                        } else {
                            ColorScheme::CLEAN_ACCENT
                        }),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("Efficiency: "),
                    Span::styled(
                        format!("{:.0}%", efficiency),
                        Style::default().fg(if efficiency > 80.0 {
                            ColorScheme::CLEAN_GREEN
                        } else {
                            ColorScheme::CLEAN_ACCENT
                        }),
                    ),
                ]),
            ];

            f.render_widget(
                Paragraph::new(metrics_lines).block(ColorScheme::clean_block()),
                chunks[1],
            );
        } else {
            let no_metrics_lines = vec![
                Line::from(Span::styled(
                    "Current Session",
                    Style::default()
                        .fg(ColorScheme::GRAY_TEXT)
                        .add_modifier(Modifier::UNDERLINED),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "No active session metrics.",
                    Style::default().fg(ColorScheme::GRAY_TEXT),
                )),
                Line::from(Span::styled(
                    "Start a session to see real-time data.",
                    Style::default().fg(ColorScheme::GRAY_TEXT),
                )),
            ];
            f.render_widget(
                Paragraph::new(no_metrics_lines).block(ColorScheme::clean_block()),
                chunks[1],
            );
        }
    }

    fn render_project_switcher(&self, f: &mut Frame, area: Rect) {
        let popup_area = self.centered_rect(60, 50, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ColorScheme::CLEAN_BLUE))
            .title(" Select Project ")
            .title_alignment(Alignment::Center)
            .style(Style::default().bg(ColorScheme::CLEAN_BG));

        f.render_widget(block.clone(), popup_area);

        let list_area = block.inner(popup_area);

        if self.available_projects.is_empty() {
            let no_projects = Paragraph::new("No projects found")
                .alignment(Alignment::Center)
                .style(Style::default().fg(ColorScheme::GRAY_TEXT));
            f.render_widget(no_projects, list_area);
        } else {
            let items: Vec<ListItem> = self
                .available_projects
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let style = if i == self.selected_project_index {
                        Style::default()
                            .fg(ColorScheme::CLEAN_BG)
                            .bg(ColorScheme::CLEAN_BLUE)
                    } else {
                        Style::default().fg(ColorScheme::WHITE_TEXT)
                    };
                    ListItem::new(format!(" {} ", p.name)).style(style)
                })
                .collect();

            let list = List::new(items);
            f.render_widget(list, list_area);
        }
    }

    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    async fn get_current_session(&mut self) -> Result<Option<Session>> {
        if !is_daemon_running() {
            return Ok(None);
        }

        let response = self
            .client
            .send_message(&IpcMessage::GetActiveSession)
            .await?;
        match response {
            IpcResponse::ActiveSession(session) => Ok(session),
            IpcResponse::Error(e) => Err(anyhow::anyhow!("Failed to get active session: {}", e)),
            _ => Ok(None),
        }
    }

    async fn get_project_by_session(&mut self, session: &Session) -> Result<Option<Project>> {
        if !is_daemon_running() {
            return Ok(None);
        }

        let response = self
            .client
            .send_message(&IpcMessage::GetProject(session.project_id))
            .await?;
        match response {
            IpcResponse::Project(project) => Ok(project),
            IpcResponse::Error(e) => Err(anyhow::anyhow!("Failed to get project: {}", e)),
            _ => Ok(None),
        }
    }

    async fn get_today_stats(&mut self) -> Result<(i64, i64, i64)> {
        // (sessions_count, total_seconds, avg_seconds)
        if !is_daemon_running() {
            return Ok((0, 0, 0));
        }

        let today = chrono::Local::now().date_naive();
        let response = self
            .client
            .send_message(&IpcMessage::GetDailyStats(today))
            .await?;
        match response {
            IpcResponse::DailyStats {
                sessions_count,
                total_seconds,
                avg_seconds,
            } => Ok((sessions_count, total_seconds, avg_seconds)),
            IpcResponse::Error(e) => Err(anyhow::anyhow!("Failed to get daily stats: {}", e)),
            _ => Ok((0, 0, 0)),
        }
    }

    async fn get_session_metrics(&mut self) -> Result<Option<crate::utils::ipc::SessionMetrics>> {
        if !is_daemon_running() {
            return Ok(None);
        }

        let response = self
            .client
            .send_message(&IpcMessage::GetSessionMetrics(0))
            .await?;
        match response {
            IpcResponse::SessionMetrics(metrics) => Ok(Some(metrics)),
            IpcResponse::Error(_) => Ok(None), // No active session
            _ => Ok(None),
        }
    }

    async fn send_activity_heartbeat(&mut self) -> Result<()> {
        if !is_daemon_running() {
            return Ok(());
        }

        let _response = self
            .client
            .send_message(&IpcMessage::ActivityHeartbeat)
            .await?;
        Ok(())
    }

    // Helper methods for project switcher navigation
    async fn toggle_project_switcher(&mut self) -> Result<()> {
        self.show_project_switcher = !self.show_project_switcher;
        if self.show_project_switcher {
            // Fetch projects when opening switcher
            self.refresh_projects().await?;
        }
        Ok(())
    }

    fn navigate_projects(&mut self, direction: i32) {
        if self.available_projects.is_empty() {
            return;
        }

        let new_index = self.selected_project_index as i32 + direction;
        if new_index >= 0 && new_index < self.available_projects.len() as i32 {
            self.selected_project_index = new_index as usize;
        }
    }

    async fn refresh_projects(&mut self) -> Result<()> {
        if !is_daemon_running() {
            return Ok(());
        }

        let response = self.client.send_message(&IpcMessage::ListProjects).await?;
        if let IpcResponse::ProjectList(projects) = response {
            self.available_projects = projects;
            self.selected_project_index = 0;
        }
        Ok(())
    }

    fn calculate_efficiency_percentage(&self, metrics: &crate::utils::ipc::SessionMetrics) -> f64 {
        if metrics.total_duration == 0 {
            return 0.0;
        }
        let active_ratio = metrics.active_duration as f64 / metrics.total_duration as f64;
        (active_ratio * 100.0).min(100.0)
    }

    async fn switch_to_selected_project(&mut self) -> Result<()> {
        if let Some(selected_project) = self.available_projects.get(self.selected_project_index) {
            // Switch to the selected project
            let project_id = selected_project.id.unwrap_or(0);
            let response = self
                .client
                .send_message(&IpcMessage::SwitchProject(project_id))
                .await?;
            match response {
                IpcResponse::Success => {
                    self.show_project_switcher = false;
                }
                IpcResponse::Error(e) => {
                    return Err(anyhow::anyhow!("Failed to switch project: {}", e))
                }
                _ => return Err(anyhow::anyhow!("Unexpected response")),
            }
        }
        Ok(())
    }

    async fn load_projects(&mut self) -> Result<Vec<Project>> {
        let db_path = get_database_path()?;
        let db = Database::new(&db_path)?;

        let projects = ProjectQueries::list_all(&db.connection, false)?; // Don't include archived
        Ok(projects)
    }
}
