use anyhow::Result;
use chrono::Local;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use log::debug;
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::time::{Duration, Instant};

use crate::{
    models::{Project, Session},
    ui::formatter::Formatter,
    ui::widgets::{ColorScheme, Spinner},
    utils::ipc::{get_socket_path, is_daemon_running, IpcClient, IpcMessage, IpcResponse},
};

pub struct Dashboard {
    client: IpcClient,
    current_session: Option<Session>,
    current_project: Option<Project>,
    daily_stats: (i64, i64, i64),
    available_projects: Vec<Project>,
    selected_project_index: usize,
    show_project_switcher: bool,
    spinner: Spinner,
    last_update: Instant,
}

impl Dashboard {
    pub async fn new() -> Result<Self> {
        let socket_path = get_socket_path()?;
        let client = if socket_path.exists() && is_daemon_running() {
            IpcClient::connect(&socket_path).await.unwrap_or_else(|_| IpcClient::new().unwrap())
        } else {
            IpcClient::new()?
        };
        Ok(Self {
            client,
            current_session: None,
            current_project: None,
            daily_stats: (0, 0, 0),
            available_projects: Vec::new(),
            selected_project_index: 0,
            show_project_switcher: false,
            spinner: Spinner::new(),
            last_update: Instant::now(),
        })
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Update state
            self.update_state().await?;

            terminal.draw(|f| self.render_dashboard_sync(f))?;

            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        if self.show_project_switcher {
                            self.handle_project_switcher_input(key).await?;
                        } else {
                            // Handle global exit here
                            if let KeyCode::Char('q') | KeyCode::Esc = key.code {
                                break;
                            }
                            self.handle_dashboard_input(key).await?;
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
    async fn update_state(&mut self) -> Result<()> {
        // Send activity heartbeat (throttled)
        if self.last_update.elapsed() >= Duration::from_secs(3) {
            if let Err(e) = self.send_activity_heartbeat().await {
                debug!("Heartbeat error: {}", e);
            }
            self.last_update = Instant::now();
        }

        // Tick animations
        self.spinner.next();

        // Get current status
        self.current_session = self.get_current_session().await?;

        // Clone session to avoid borrow conflict
        let session_clone = self.current_session.clone();
        if let Some(session) = session_clone {
            self.current_project = self.get_project_by_session(&session).await?;
        } else {
            self.current_project = None;
        }

        self.daily_stats = self.get_today_stats().await.unwrap_or((0, 0, 0));

        Ok(())
    }

    async fn handle_dashboard_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // 'q' and 'Esc' are handled in run()
            KeyCode::Char('p') => {
                self.refresh_projects().await?;
                self.show_project_switcher = true;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_project_switcher_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.show_project_switcher = false;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigate_projects(-1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.navigate_projects(1);
            }
            KeyCode::Enter => {
                self.switch_to_selected_project().await?;
            }
            _ => {}
        }
        Ok(())
    }

    async fn ensure_connected(&mut self) -> Result<()> {
        if !is_daemon_running() {
            return Err(anyhow::anyhow!("Daemon is not running"));
        }
        
        // Test if we have a working connection
        if self.client.stream.is_some() {
            return Ok(());
        }
        
        // Reconnect if needed
        let socket_path = get_socket_path()?;
        if socket_path.exists() {
            self.client = IpcClient::connect(&socket_path).await?;
        }
        Ok(())
    }

    fn render_dashboard_sync(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Length(1),  // Spacer
                Constraint::Length(10), // Active Session Panel
                Constraint::Length(1),  // Spacer
                Constraint::Length(3),  // Quick Stats Header
                Constraint::Length(5),  // Quick Stats Grid
                Constraint::Length(1),  // Spacer
                Constraint::Min(10),    // Recent Projects & Timeline
                Constraint::Length(1),  // Bottom bar
            ])
            .split(f.size());

        // Header
        self.render_header(f, chunks[0]);

        let daily_stats = self.get_daily_stats();
        let current_session = &self.current_session;
        let current_project = &self.current_project;

        // 1. Active Session Panel
        self.render_active_session_panel(f, chunks[2], current_session, current_project);

        // 2. Quick Stats
        self.render_quick_stats(f, chunks[4], chunks[5], daily_stats);

        // 3. Recent Projects & Timeline
        self.render_projects_and_timeline(f, chunks[5]);

        // 4. Bottom Bar
        self.render_bottom_bar(f, chunks[6]);

        // Project switcher overlay
        if self.show_project_switcher {
            self.render_project_switcher(f, f.size());
        }
    }

    fn render_active_session_panel(
        &self,
        f: &mut Frame,
        area: Rect,
        session: &Option<Session>,
        project: &Option<Project>,
    ) {
        let block = Block::default().style(Style::default().bg(ColorScheme::CLEAN_BG));

        f.render_widget(block, area);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // "Active Session" label
                Constraint::Length(2), // Project Name & State
                Constraint::Length(1), // Spacer
                Constraint::Length(3), // Large Timer
            ])
            .margin(1)
            .split(area);

        // Label
        f.render_widget(
            Paragraph::new("Active Session").style(
                Style::default()
                    .fg(ColorScheme::GRAY_TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
            layout[0],
        );

        if let Some(session) = session {
            let project_name = project
                .as_ref()
                .map(|p| p.name.as_str())
                .unwrap_or("Unknown Project");

            // Project Name & State
            let info_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                .split(layout[1]);

            f.render_widget(
                Paragraph::new(project_name).style(
                    Style::default()
                        .fg(ColorScheme::GRAY_TEXT)
                        .add_modifier(Modifier::BOLD),
                ),
                info_layout[0],
            );

            f.render_widget(
                Paragraph::new("State: ACTIVE")
                    .alignment(Alignment::Right)
                    .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
                info_layout[1],
            );

            // Timer
            let now = Local::now();
            let elapsed_seconds = (now.timestamp() - session.start_time.timestamp())
                - session.paused_duration.num_seconds();
            let duration_str = Formatter::format_duration(elapsed_seconds);

            f.render_widget(
                Paragraph::new(duration_str)
                    .alignment(Alignment::Center)
                    .style(
                        Style::default()
                            .fg(ColorScheme::WHITE_TEXT)
                            .add_modifier(Modifier::BOLD),
                    ),
                layout[3],
            );
        } else {
            // Idle State
            f.render_widget(
                Paragraph::new("No Active Session")
                    .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
                layout[1],
            );
            f.render_widget(
                Paragraph::new("--:--:--")
                    .alignment(Alignment::Center)
                    .style(
                        Style::default()
                            .fg(ColorScheme::GRAY_TEXT)
                            .add_modifier(Modifier::DIM),
                    ),
                layout[3],
            );
        }
    }

    fn render_quick_stats(
        &self,
        f: &mut Frame,
        header_area: Rect,
        grid_area: Rect,
        daily_stats: &(i64, i64, i64),
    ) {
        let (sessions_count, total_seconds, _avg_seconds) = *daily_stats;

        // Header
        f.render_widget(
            Paragraph::new("Quick Stats").style(
                Style::default()
                    .fg(ColorScheme::WHITE_TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
            header_area,
        );

        // Grid
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(grid_area);

        let stats = [
            ("Today", Formatter::format_duration(total_seconds)),
            ("This Week", "12h 30m".to_string()), // Placeholder for now
            ("Active", sessions_count.to_string()),
            ("Projects", self.available_projects.len().to_string()),
        ];

        for (i, (label, value)) in stats.iter().enumerate() {
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ColorScheme::GRAY_TEXT))
                .style(Style::default().bg(ColorScheme::CLEAN_BG));

            let content = Paragraph::new(vec![
                Line::from(Span::styled(
                    *label,
                    Style::default().fg(ColorScheme::GRAY_TEXT),
                )),
                Line::from(Span::styled(
                    value.as_str(),
                    Style::default()
                        .fg(ColorScheme::WHITE_TEXT)
                        .add_modifier(Modifier::BOLD),
                )),
            ])
            .block(block)
            .alignment(Alignment::Center);

            f.render_widget(content, cols[i]);
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

    fn render_projects_and_timeline(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Header
                Constraint::Min(5),    // Project List
                Constraint::Length(2), // Timeline Header
                Constraint::Length(3), // Timeline Bar
            ])
            .split(area);

        // Projects Header
        f.render_widget(
            Paragraph::new("Recent Projects").style(
                Style::default()
                    .fg(ColorScheme::WHITE_TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
            chunks[0],
        );

        // Project List (Mock data for visual alignment)
        let projects = &self.available_projects;
        let items: Vec<ListItem> = projects
            .iter()
            .take(5)
            .map(|p| {
                let content = format!(
                    "{:<20} {:<10} {:<10} {:<10}",
                    p.name,
                    "0h 00m", // Placeholder
                    "0h 00m", // Placeholder
                    "Today"   // Placeholder
                );
                ListItem::new(content).style(Style::default().fg(ColorScheme::GRAY_TEXT))
            })
            .collect();

        // Header row
        let header = Paragraph::new(format!(
            "{:<20} {:<10} {:<10} {:<10}",
            "Name", "Today", "Total", "Last Active"
        ))
        .style(
            Style::default()
                .fg(ColorScheme::GRAY_TEXT)
                .add_modifier(Modifier::UNDERLINED),
        );

        let list_area = chunks[1];
        let header_area = Rect::new(list_area.x, list_area.y, list_area.width, 1);
        let items_area = Rect::new(
            list_area.x,
            list_area.y + 1,
            list_area.width,
            list_area.height - 1,
        );

        f.render_widget(header, header_area);
        f.render_widget(List::new(items), items_area);

        // Timeline Header
        f.render_widget(
            Paragraph::new("Activity Timeline").style(
                Style::default()
                    .fg(ColorScheme::WHITE_TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
            chunks[2],
        );

        // Timeline Bar (Visual Mock)
        let bar = Block::default().style(Style::default().bg(ColorScheme::CLEAN_BLUE)); // Simple bar for now
        f.render_widget(bar, chunks[3]);

        // Timeline labels
        let labels = Paragraph::new("08:00       12:00       16:00")
            .style(Style::default().fg(ColorScheme::GRAY_TEXT));
        f.render_widget(
            labels,
            Rect::new(chunks[3].x, chunks[3].y + 1, chunks[3].width, 1),
        );
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

        self.ensure_connected().await?;

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

        self.ensure_connected().await?;

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

        self.ensure_connected().await?;

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


    async fn send_activity_heartbeat(&mut self) -> Result<()> {
        if !is_daemon_running() {
            return Ok(());
        }

        self.ensure_connected().await?;

        let _response = self
            .client
            .send_message(&IpcMessage::ActivityHeartbeat)
            .await?;
        Ok(())
    }

    // Helper methods for project switcher navigation

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

        self.ensure_connected().await?;

        let response = self.client.send_message(&IpcMessage::ListProjects).await?;
        if let IpcResponse::ProjectList(projects) = response {
            self.available_projects = projects;
            self.selected_project_index = 0;
        }
        Ok(())
    }

    async fn switch_to_selected_project(&mut self) -> Result<()> {
        if let Some(selected_project) = self.available_projects.get(self.selected_project_index) {
            let project_id = selected_project.id.unwrap_or(0);
            
            self.ensure_connected().await?;
            
            // Switch to the selected project
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

    fn render_header(&self, f: &mut Frame, area: Rect) {
        let time_str = Local::now().format("%H:%M").to_string();
        let date_str = Local::now().format("%A, %B %d").to_string();

        let header_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // Title
                Constraint::Percentage(50), // Date/Time
            ])
            .split(area);

        f.render_widget(
            Paragraph::new("TEMPO").style(
                Style::default()
                    .fg(ColorScheme::CLEAN_GOLD)
                    .add_modifier(Modifier::BOLD),
            ),
            header_layout[0],
        );

        f.render_widget(
            Paragraph::new(format!("{}  {}", date_str, time_str))
                .alignment(Alignment::Right)
                .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
            header_layout[1],
        );
    }

    fn get_daily_stats(&self) -> &(i64, i64, i64) {
        &self.daily_stats
    }
}
