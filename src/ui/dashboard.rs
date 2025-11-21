use anyhow::Result;
use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use log::debug;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Gauge, List, ListItem, Paragraph, Wrap},
    Frame, Terminal,
};
use std::time::Duration;

use crate::{
    db::queries::ProjectQueries,
    db::{get_database_path, Database},
    models::{Project, Session},
    ui::formatter::Formatter,
    ui::widgets::{ColorScheme, Spinner},
    utils::ipc::{get_socket_path, is_daemon_running, IpcClient, IpcMessage, IpcResponse},
};

pub struct Dashboard {
    client: IpcClient,
    show_project_switcher: bool,
    available_projects: Vec<Project>,
    selected_project_index: usize,
    spinner: Spinner,
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

            // Tick spinner animation
            self.spinner.next();

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
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Title
                Constraint::Length(10), // Enhanced session info with progress
                Constraint::Length(6),  // Project info
                Constraint::Length(8),  // Real-time metrics with visuals
                Constraint::Min(0),     // Statistics with charts
                Constraint::Length(3),  // Help
            ])
            .split(f.size());

        // Title
        let spinner_char = self.spinner.current();
        let title_text = format!(" {} Tempo - Time Tracking Dashboard ", spinner_char);
        let title = Paragraph::new(title_text)
            .style(
                Style::default()
                    .fg(ColorScheme::NEON_PINK)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(ColorScheme::base_block());
        f.render_widget(title, chunks[0]);

        // Current session info
        self.render_session_info(f, chunks[1], current_session);

        // Project info
        self.render_project_info(f, chunks[2], current_project);

        // Real-time metrics
        self.render_session_metrics(f, chunks[3], session_metrics);

        // Statistics
        self.render_statistics_sync(f, chunks[4], daily_stats);

        // Help
        self.render_help(f, chunks[5]);

        // Project switcher overlay
        if self.show_project_switcher {
            self.render_project_switcher(f, f.size());
        }
    }

    fn render_session_info(&self, f: &mut Frame, area: Rect, session: &Option<Session>) {
        let block = ColorScheme::base_block().title(Span::styled(
            " Current Session ",
            Style::default().fg(ColorScheme::title()),
        ));

        if let Some(session) = session {
            let now = Local::now();
            let elapsed_seconds = (now.timestamp() - session.start_time.timestamp())
                - session.paused_duration.num_seconds();

            // Split the area for text and progress bar
            let session_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(6), // Session info
                    Constraint::Length(2), // Progress bar
                ])
                .split(area);

            // Session information
            let status_text = vec![
                Line::from(vec![
                    Span::raw("Status: "),
                    Span::styled(
                        "● ACTIVE",
                        Style::default()
                            .fg(ColorScheme::NEON_GREEN)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("Started: "),
                    Span::styled(
                        Formatter::format_timestamp(&session.start_time.with_timezone(&Local)),
                        Style::default().fg(ColorScheme::WHITE_TEXT),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("Elapsed: "),
                    Span::styled(
                        Formatter::format_duration(elapsed_seconds),
                        Style::default()
                            .fg(ColorScheme::NEON_CYAN)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("Context: "),
                    Span::styled(
                        session.context.to_string(),
                        Style::default().fg(ColorScheme::NEON_YELLOW),
                    ),
                ]),
            ];

            let session_block = ColorScheme::base_block().title(Span::styled(
                " Current Session ",
                Style::default().fg(ColorScheme::title()),
            ));

            let paragraph = Paragraph::new(status_text)
                .block(session_block)
                .wrap(Wrap { trim: true });
            f.render_widget(paragraph, session_chunks[0]);

            // Visual progress bar for session duration
            let progress_ratio = self.calculate_session_progress(elapsed_seconds);
            let progress_bar = Gauge::default()
                .block(ColorScheme::base_block().title(Span::styled(
                    " Session Progress ",
                    Style::default().fg(ColorScheme::title()),
                )))
                .gauge_style(
                    Style::default()
                        .fg(ColorScheme::NEON_GREEN)
                        .bg(Color::Black),
                )
                .percent((progress_ratio * 100.0) as u16)
                .label(format!(
                    "{} / target: 2h",
                    Formatter::format_duration(elapsed_seconds)
                ));
            f.render_widget(progress_bar, session_chunks[1]);
        } else {
            let no_session_text = vec![
                Line::from(Span::styled(
                    "No active session",
                    Style::default().fg(ColorScheme::GRAY_TEXT),
                )),
                Line::from(Span::raw("")),
                Line::from(Span::raw("Use 'tempo start' to begin tracking time")),
                Line::from(Span::raw("")),
                Line::from(Span::styled(
                    "🎯 Set your focus and track your productivity",
                    Style::default().fg(ColorScheme::NEON_CYAN),
                )),
            ];

            let paragraph = Paragraph::new(no_session_text)
                .block(block)
                .wrap(Wrap { trim: true });
            f.render_widget(paragraph, area);
        }
    }

    fn render_project_info(&self, f: &mut Frame, area: Rect, project: &Option<Project>) {
        let block = ColorScheme::base_block().title(Span::styled(
            " Current Project ",
            Style::default().fg(ColorScheme::title()),
        ));

        if let Some(project) = project {
            let project_text = vec![
                Line::from(vec![
                    Span::raw("Name: "),
                    Span::styled(
                        &project.name,
                        Style::default()
                            .fg(ColorScheme::NEON_YELLOW)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("Path: "),
                    Span::styled(
                        project.path.to_string_lossy().to_string(),
                        Style::default().fg(ColorScheme::GRAY_TEXT),
                    ),
                ]),
            ];

            let paragraph = Paragraph::new(project_text)
                .block(block)
                .wrap(Wrap { trim: true });
            f.render_widget(paragraph, area);
        } else {
            let no_project_text = vec![Line::from(Span::styled(
                "No active project",
                Style::default().fg(ColorScheme::GRAY_TEXT),
            ))];

            let paragraph = Paragraph::new(no_project_text)
                .block(block)
                .wrap(Wrap { trim: true });
            f.render_widget(paragraph, area);
        }
    }

    fn render_statistics_sync(&self, f: &mut Frame, area: Rect, daily_stats: &(i64, i64, i64)) {
        let (sessions_count, total_seconds, avg_seconds) = *daily_stats;

        if sessions_count > 0 {
            // Split area for text and visual chart
            let stats_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50), // Stats text
                    Constraint::Percentage(50), // Visual chart
                ])
                .split(area);

            // Statistics text
            let stats_text = vec![
                Line::from(vec![
                    Span::raw("📊 Sessions: "),
                    Span::styled(
                        sessions_count.to_string(),
                        Style::default()
                            .fg(ColorScheme::NEON_PURPLE)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("⏱️  Total time: "),
                    Span::styled(
                        Formatter::format_duration(total_seconds),
                        Style::default().fg(ColorScheme::NEON_GREEN),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("📈 Avg session: "),
                    Span::styled(
                        Formatter::format_duration(avg_seconds),
                        Style::default().fg(ColorScheme::NEON_CYAN),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("🎯 Target: "),
                    Span::styled(
                        format!(
                            "{:.0}% complete",
                            (total_seconds as f64 / (8.0 * 3600.0)) * 100.0
                        ),
                        if total_seconds > 4 * 3600 {
                            Style::default().fg(ColorScheme::NEON_GREEN)
                        } else {
                            Style::default().fg(ColorScheme::NEON_YELLOW)
                        },
                    ),
                ]),
            ];

            let text_block = ColorScheme::base_block().title(Span::styled(
                " Today's Summary ",
                Style::default().fg(ColorScheme::title()),
            ));

            let paragraph = Paragraph::new(stats_text)
                .block(text_block)
                .wrap(Wrap { trim: true });
            f.render_widget(paragraph, stats_chunks[0]);

            // Visual progress bar for daily goal (8 hours)
            let daily_goal_seconds = 8 * 3600; // 8 hours
            let progress_percentage =
                ((total_seconds as f64 / daily_goal_seconds as f64) * 100.0).min(100.0);

            let goal_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Daily goal progress
                    Constraint::Min(0),    // Activity sparkline (placeholder)
                ])
                .split(stats_chunks[1]);

            let daily_progress = Gauge::default()
                .block(ColorScheme::base_block().title(Span::styled(
                    " Daily Goal (8h) ",
                    Style::default().fg(ColorScheme::title()),
                )))
                .gauge_style(Style::default().fg(if progress_percentage >= 100.0 {
                    ColorScheme::NEON_GREEN
                } else if progress_percentage >= 50.0 {
                    ColorScheme::NEON_YELLOW
                } else {
                    ColorScheme::NEON_PINK
                }))
                .percent(progress_percentage as u16)
                .label(format!("{:.1}%", progress_percentage));
            f.render_widget(daily_progress, goal_chunks[0]);

            // Placeholder for activity sparkline or mini-chart
            let activity_placeholder = Paragraph::new(vec![
                Line::from(Span::styled(
                    "📈 Activity Timeline",
                    Style::default().fg(ColorScheme::NEON_CYAN),
                )),
                Line::from(Span::raw(" ▂▃▅▇█▇▅▃▂  (simulated)")),
            ])
            .block(ColorScheme::base_block().title(Span::styled(
                " Activity Pattern ",
                Style::default().fg(ColorScheme::title()),
            )))
            .alignment(Alignment::Center);
            f.render_widget(activity_placeholder, goal_chunks[1]);
        } else {
            let no_stats_text = vec![
                Line::from(Span::styled(
                    "📊 No sessions today",
                    Style::default().fg(ColorScheme::GRAY_TEXT),
                )),
                Line::from(Span::raw("")),
                Line::from(Span::raw("🚀 Start your first session to see:")),
                Line::from(Span::raw("  • Session count and timing")),
                Line::from(Span::raw("  • Daily goal progress")),
                Line::from(Span::raw("  • Activity patterns")),
                Line::from(Span::raw("  • Productivity insights")),
            ];

            let block = ColorScheme::base_block().title(Span::styled(
                " Today's Summary ",
                Style::default().fg(ColorScheme::title()),
            ));

            let paragraph = Paragraph::new(no_stats_text)
                .block(block)
                .wrap(Wrap { trim: true });
            f.render_widget(paragraph, area);
        }
    }

    fn render_session_metrics(
        &self,
        f: &mut Frame,
        area: Rect,
        metrics: &Option<crate::utils::ipc::SessionMetrics>,
    ) {
        if let Some(metrics) = metrics {
            // Split area for metrics text and visual indicators
            let metrics_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(60), // Metrics text
                    Constraint::Percentage(40), // Visual indicators
                ])
                .split(area);

            // Metrics text
            let activity_color = match metrics.activity_score {
                s if s > 0.7 => ColorScheme::NEON_GREEN,
                s if s > 0.3 => ColorScheme::NEON_YELLOW,
                _ => ColorScheme::NEON_PINK,
            };

            let activity_indicator = match metrics.activity_score {
                s if s > 0.8 => "🔥 Very Active",
                s if s > 0.6 => "⚡ Active",
                s if s > 0.3 => "⏳ Moderate",
                _ => "😴 Low Activity",
            };

            let metrics_text = vec![
                Line::from(vec![
                    Span::raw("Activity: "),
                    Span::styled(activity_indicator, Style::default().fg(activity_color)),
                ]),
                Line::from(vec![
                    Span::raw("Score: "),
                    Span::styled(
                        format!("{:.1}%", metrics.activity_score * 100.0),
                        Style::default().fg(activity_color),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("Active: "),
                    Span::styled(
                        Formatter::format_duration(metrics.active_duration),
                        Style::default().fg(ColorScheme::NEON_CYAN),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("Paused: "),
                    Span::styled(
                        Formatter::format_duration(metrics.paused_duration),
                        Style::default().fg(ColorScheme::GRAY_TEXT),
                    ),
                ]),
                Line::from(vec![
                    Span::raw("Efficiency: "),
                    Span::styled(
                        format!("{:.0}%", self.calculate_efficiency_percentage(metrics)),
                        Style::default().fg(
                            if self.calculate_efficiency_percentage(metrics) > 70.0 {
                                ColorScheme::NEON_GREEN
                            } else {
                                ColorScheme::NEON_YELLOW
                            },
                        ),
                    ),
                ]),
            ];

            let text_block = ColorScheme::base_block().title(Span::styled(
                " Real-time Metrics ",
                Style::default().fg(ColorScheme::title()),
            ));

            let paragraph = Paragraph::new(metrics_text)
                .block(text_block)
                .wrap(Wrap { trim: true });
            f.render_widget(paragraph, metrics_chunks[0]);

            // Visual activity indicator
            let activity_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3), // Activity gauge
                    Constraint::Length(3), // Efficiency gauge
                ])
                .split(metrics_chunks[1]);

            // Activity level gauge
            let activity_gauge = Gauge::default()
                .block(ColorScheme::base_block().title(Span::styled(
                    " Activity ",
                    Style::default().fg(ColorScheme::title()),
                )))
                .gauge_style(Style::default().fg(activity_color))
                .percent((metrics.activity_score * 100.0) as u16)
                .label(format!("{:.0}%", metrics.activity_score * 100.0));
            f.render_widget(activity_gauge, activity_chunks[0]);

            // Efficiency gauge
            let efficiency = self.calculate_efficiency_percentage(metrics);
            let efficiency_color = if efficiency > 80.0 {
                ColorScheme::NEON_GREEN
            } else if efficiency > 60.0 {
                ColorScheme::NEON_YELLOW
            } else {
                ColorScheme::NEON_PINK
            };

            let efficiency_gauge = Gauge::default()
                .block(ColorScheme::base_block().title(Span::styled(
                    " Efficiency ",
                    Style::default().fg(ColorScheme::title()),
                )))
                .gauge_style(Style::default().fg(efficiency_color))
                .percent(efficiency as u16)
                .label(format!("{:.0}%", efficiency));
            f.render_widget(efficiency_gauge, activity_chunks[1]);
        } else {
            let no_metrics_block = ColorScheme::base_block().title(Span::styled(
                " Real-time Metrics ",
                Style::default().fg(ColorScheme::title()),
            ));

            let no_metrics_text = vec![
                Line::from(Span::styled(
                    "No active session",
                    Style::default().fg(ColorScheme::GRAY_TEXT),
                )),
                Line::from(Span::raw("")),
                Line::from(Span::raw("Start tracking to see:")),
                Line::from(Span::raw("• Activity indicators")),
                Line::from(Span::raw("• Efficiency metrics")),
                Line::from(Span::raw("• Visual progress")),
            ];

            let paragraph = Paragraph::new(no_metrics_text)
                .block(no_metrics_block)
                .wrap(Wrap { trim: true });
            f.render_widget(paragraph, area);
        }
    }

    fn render_help(&self, f: &mut Frame, area: Rect) {
        let help_text = if self.show_project_switcher {
            "Project Switcher: ↑/↓ Navigate | Enter - Select | P/Esc - Close"
        } else {
            "Press 'q' or 'Esc' to quit | 'p' for project switcher | Updates every 100ms"
        };

        let help_paragraph = Paragraph::new(help_text)
            .style(Style::default().fg(ColorScheme::GRAY_TEXT))
            .alignment(Alignment::Center)
            .block(ColorScheme::base_block());
        f.render_widget(help_paragraph, area);
    }

    fn render_project_switcher(&self, f: &mut Frame, area: Rect) {
        // Create a centered popup
        let popup_area = self.centered_rect(60, 70, area);

        // Clear the background
        let background = ColorScheme::base_block()
            .title(Span::styled(
                " 🔄 Project Switcher ",
                Style::default().fg(ColorScheme::title()),
            ))
            .title_alignment(Alignment::Center);
        f.render_widget(background, popup_area);

        // Create the project list
        let projects_area = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .split(popup_area)[0];

        if self.available_projects.is_empty() {
            let no_projects = Paragraph::new(
                "No projects found\n\nCreate a project first using:\ntempo init <project-name>",
            )
            .style(Style::default().fg(ColorScheme::NEON_YELLOW))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
            f.render_widget(no_projects, projects_area);
        } else {
            let project_items: Vec<ListItem> = self
                .available_projects
                .iter()
                .enumerate()
                .map(|(i, project)| {
                    let style = if i == self.selected_project_index {
                        Style::default()
                            .fg(Color::Black)
                            .bg(ColorScheme::NEON_CYAN)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(ColorScheme::WHITE_TEXT)
                    };

                    let content = vec![
                        Line::from(vec![Span::styled(format!("{}", project.name), style)]),
                        Line::from(vec![Span::styled(
                            format!("  📁 {}", project.path.to_string_lossy()),
                            Style::default().fg(if i == self.selected_project_index {
                                Color::Black
                            } else {
                                ColorScheme::GRAY_TEXT
                            }),
                        )]),
                    ];

                    ListItem::new(content).style(style)
                })
                .collect();

            let projects_list =
                List::new(project_items).style(Style::default().fg(ColorScheme::WHITE_TEXT));
            f.render_widget(projects_list, projects_area);
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

    fn calculate_session_progress(&self, elapsed_seconds: i64) -> f64 {
        // Progress towards 2-hour session target
        let target_seconds = 2 * 3600; // 2 hours
        (elapsed_seconds as f64 / target_seconds as f64).min(1.0)
    }

    fn calculate_efficiency_percentage(&self, metrics: &crate::utils::ipc::SessionMetrics) -> f64 {
        if metrics.total_duration == 0 {
            return 0.0;
        }

        let efficiency = (metrics.active_duration as f64 / metrics.total_duration as f64) * 100.0;
        efficiency.min(100.0)
    }

    async fn toggle_project_switcher(&mut self) -> Result<()> {
        if self.show_project_switcher {
            self.show_project_switcher = false;
        } else {
            // Load available projects
            self.available_projects = self.load_projects().await?;
            self.selected_project_index = 0;
            self.show_project_switcher = true;
        }
        Ok(())
    }

    fn navigate_projects(&mut self, direction: i32) {
        if !self.available_projects.is_empty() {
            let current = self.selected_project_index as i32;
            let new_index = (current + direction)
                .max(0)
                .min(self.available_projects.len() as i32 - 1);
            self.selected_project_index = new_index as usize;
        }
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
