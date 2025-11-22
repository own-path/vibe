use anyhow::Result;
use chrono::{Local, Timelike};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use log::debug;
use ratatui::{
    backend::Backend,
    buffer::Buffer,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, List, ListItem, Paragraph, Row, Table, Widget},
    Frame, Terminal,
};
use std::time::{Duration, Instant};

use crate::{
    models::{Project, Session},
    ui::formatter::Formatter,
    ui::widgets::{ColorScheme, SessionStatsWidget, Spinner},
    utils::ipc::{
        get_socket_path, is_daemon_running, IpcClient, IpcMessage, IpcResponse, ProjectWithStats,
    },
};

#[derive(Clone, PartialEq)]
pub enum DashboardView {
    FocusedSession,
    Overview,
    History,
    Projects,
}

#[derive(Clone)]
pub struct SessionFilter {
    pub start_date: Option<chrono::NaiveDate>,
    pub end_date: Option<chrono::NaiveDate>,
    pub project_filter: Option<String>,
    pub duration_filter: Option<(i64, i64)>, // min, max seconds
    pub search_text: String,
}

impl Default for SessionFilter {
    fn default() -> Self {
        Self {
            start_date: None,
            end_date: None,
            project_filter: None,
            duration_filter: None,
            search_text: String::new(),
        }
    }
}

pub struct Dashboard {
    client: IpcClient,
    current_session: Option<Session>,
    current_project: Option<Project>,
    daily_stats: (i64, i64, i64),
    weekly_stats: i64,
    today_sessions: Vec<Session>,
    recent_projects: Vec<ProjectWithStats>,
    available_projects: Vec<Project>,
    selected_project_index: usize,
    show_project_switcher: bool,
    current_view: DashboardView,
    
    // History browser state
    history_sessions: Vec<Session>,
    selected_session_index: usize,
    session_filter: SessionFilter,
    filter_input_mode: bool,
    
    // Project grid state
    selected_project_row: usize,
    selected_project_col: usize,
    projects_per_row: usize,
    
    spinner: Spinner,
    last_update: Instant,
}

impl Dashboard {
    pub async fn new() -> Result<Self> {
        let socket_path = get_socket_path()?;
        let client = if socket_path.exists() && is_daemon_running() {
            IpcClient::connect(&socket_path)
                .await
                .unwrap_or_else(|_| IpcClient::new().unwrap())
        } else {
            IpcClient::new()?
        };
        Ok(Self {
            client,
            current_session: None,
            current_project: None,
            daily_stats: (0, 0, 0),
            weekly_stats: 0,
            today_sessions: Vec::new(),
            recent_projects: Vec::new(),
            available_projects: Vec::new(),
            selected_project_index: 0,
            show_project_switcher: false,
            current_view: DashboardView::FocusedSession,
            
            // Initialize history browser state
            history_sessions: Vec::new(),
            selected_session_index: 0,
            session_filter: SessionFilter::default(),
            filter_input_mode: false,
            
            // Initialize project grid state
            selected_project_row: 0,
            selected_project_col: 0,
            projects_per_row: 3,
            
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
                            match key.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Esc => {
                                    if self.current_view == DashboardView::FocusedSession {
                                        self.current_view = DashboardView::Overview;
                                    } else {
                                        break;
                                    }
                                }
                                _ => self.handle_dashboard_input(key).await?,
                            }
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
        self.weekly_stats = self.get_weekly_stats().await.unwrap_or(0);
        self.today_sessions = self.get_today_sessions().await.unwrap_or_default();
        self.recent_projects = self.get_recent_projects().await.unwrap_or_default();
        
        // Update history sessions if in history view
        if self.current_view == DashboardView::History {
            self.history_sessions = self.get_history_sessions().await.unwrap_or_default();
        }
        
        // Update project list if in project view
        if self.current_view == DashboardView::Projects && self.available_projects.is_empty() {
            if let Err(_) = self.refresh_projects().await {
                // Ignore errors and use empty list
            }
        }

        Ok(())
    }

    async fn get_weekly_stats(&mut self) -> Result<i64> {
        match self.client.send_message(&IpcMessage::GetWeeklyStats).await {
            Ok(IpcResponse::WeeklyStats { total_seconds }) => Ok(total_seconds),
            Ok(response) => {
                debug!("Unexpected response for GetWeeklyStats: {:?}", response);
                Err(anyhow::anyhow!("Unexpected response"))
            }
            Err(e) => {
                debug!("Failed to receive GetWeeklyStats response: {}", e);
                Err(anyhow::anyhow!("Failed to receive response"))
            }
        }
    }

    async fn get_recent_projects(&mut self) -> Result<Vec<ProjectWithStats>> {
        match self
            .client
            .send_message(&IpcMessage::GetRecentProjects)
            .await
        {
            Ok(IpcResponse::RecentProjects(projects)) => Ok(projects),
            Ok(response) => {
                debug!("Unexpected response for GetRecentProjects: {:?}", response);
                Err(anyhow::anyhow!("Unexpected response"))
            }
            Err(e) => {
                debug!("Failed to receive GetRecentProjects response: {}", e);
                Err(anyhow::anyhow!("Failed to receive response"))
            }
        }
    }

    async fn handle_dashboard_input(&mut self, key: KeyEvent) -> Result<()> {
        // Handle view-specific inputs first
        match self.current_view {
            DashboardView::History => {
                return self.handle_history_input(key).await;
            }
            DashboardView::Projects => {
                return self.handle_project_grid_input(key).await;
            }
            _ => {}
        }

        // Handle global navigation
        match key.code {
            // View navigation
            KeyCode::Char('1') => self.current_view = DashboardView::FocusedSession,
            KeyCode::Char('2') => self.current_view = DashboardView::Overview,
            KeyCode::Char('3') => self.current_view = DashboardView::History,
            KeyCode::Char('4') => self.current_view = DashboardView::Projects,
            KeyCode::Char('f') => self.current_view = DashboardView::FocusedSession,
            KeyCode::Tab => {
                self.current_view = match self.current_view {
                    DashboardView::FocusedSession => DashboardView::Overview,
                    DashboardView::Overview => DashboardView::History,
                    DashboardView::History => DashboardView::Projects,
                    DashboardView::Projects => DashboardView::FocusedSession,
                };
            }
            // Project switcher (only in certain views)
            KeyCode::Char('p') if self.current_view != DashboardView::Projects => {
                self.refresh_projects().await?;
                self.show_project_switcher = true;
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_history_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Navigation in session list
            KeyCode::Up | KeyCode::Char('k') => {
                if !self.history_sessions.is_empty() && self.selected_session_index > 0 {
                    self.selected_session_index -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_session_index < self.history_sessions.len().saturating_sub(1) {
                    self.selected_session_index += 1;
                }
            }
            // Search mode
            KeyCode::Char('/') => {
                self.filter_input_mode = true;
            }
            KeyCode::Enter if self.filter_input_mode => {
                self.filter_input_mode = false;
                self.history_sessions = self.get_history_sessions().await.unwrap_or_default();
            }
            // Character input in search mode
            KeyCode::Char(c) if self.filter_input_mode => {
                self.session_filter.search_text.push(c);
            }
            KeyCode::Backspace if self.filter_input_mode => {
                self.session_filter.search_text.pop();
            }
            KeyCode::Esc if self.filter_input_mode => {
                self.filter_input_mode = false;
                self.session_filter.search_text.clear();
            }
            _ => {}
        }
        Ok(())
    }

    async fn handle_project_grid_input(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Grid navigation
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_project_row > 0 {
                    self.selected_project_row -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let total_projects = self.available_projects.len();
                let total_rows = (total_projects + self.projects_per_row - 1) / self.projects_per_row;
                if self.selected_project_row < total_rows.saturating_sub(1) {
                    // Only move down if there's a project on the next row
                    let next_row_first_index = (self.selected_project_row + 1) * self.projects_per_row;
                    if next_row_first_index < total_projects {
                        self.selected_project_row += 1;
                    }
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if self.selected_project_col > 0 {
                    self.selected_project_col -= 1;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                let row_start = self.selected_project_row * self.projects_per_row;
                let row_end = (row_start + self.projects_per_row).min(self.available_projects.len());
                let max_col = (row_end - row_start).saturating_sub(1);
                if self.selected_project_col < max_col {
                    self.selected_project_col += 1;
                }
            }
            // Project selection
            KeyCode::Enter => {
                self.switch_to_grid_selected_project().await?;
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

    async fn switch_to_grid_selected_project(&mut self) -> Result<()> {
        let selected_index = self.selected_project_row * self.projects_per_row + self.selected_project_col;
        if let Some(selected_project) = self.available_projects.get(selected_index) {
            let project_id = selected_project.id.unwrap_or(0);

            self.ensure_connected().await?;

            // Switch to the selected project
            let response = self
                .client
                .send_message(&IpcMessage::SwitchProject(project_id))
                .await?;
            match response {
                IpcResponse::Success => {
                    // Switch to focused view after selection
                    self.current_view = DashboardView::FocusedSession;
                }
                IpcResponse::Error(e) => {
                    return Err(anyhow::anyhow!("Failed to switch project: {}", e))
                }
                _ => return Err(anyhow::anyhow!("Unexpected response")),
            }
        }
        Ok(())
    }

    fn render_keyboard_hints(&self, area: Rect, buf: &mut Buffer) {
        let hints = match self.current_view {
            DashboardView::FocusedSession => vec![
                ("Esc", "Exit Focus"),
                ("Tab", "Next View"),
                ("p", "Projects"),
            ],
            DashboardView::History => vec![
                ("↑/↓", "Navigate"),
                ("/", "Search"),
                ("Tab", "Next View"),
                ("q", "Quit"),
            ],
            DashboardView::Projects => vec![
                ("↑/↓/←/→", "Navigate"),
                ("Enter", "Select"),
                ("Tab", "Next View"),
                ("q", "Quit"),
            ],
            _ => vec![
                ("q", "Quit"),
                ("f", "Focus"),
                ("Tab", "Next View"), 
                ("1-4", "View"),
                ("p", "Projects"),
            ],
        };

        let spans: Vec<Span> = hints
            .iter()
            .flat_map(|(key, desc)| {
                vec![
                    Span::styled(
                        format!(" {} ", key),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("{} ", desc), Style::default().fg(Color::DarkGray)),
                ]
            })
            .collect();

        let line = Line::from(spans);
        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray));
        Paragraph::new(line).block(block).render(area, buf);
    }

    fn render_dashboard_sync(&mut self, f: &mut Frame) {
        match self.current_view {
            DashboardView::FocusedSession => self.render_focused_session_view(f),
            DashboardView::Overview => self.render_overview_dashboard(f),
            DashboardView::History => self.render_history_browser(f),
            DashboardView::Projects => self.render_project_grid(f),
        }

        // Project switcher overlay (available on most views)
        if self.show_project_switcher {
            self.render_project_switcher(f, f.size());
        }
    }

    fn render_focused_session_view(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header with ESC hint
                Constraint::Length(2),  // Spacer
                Constraint::Length(6),  // Project info box
                Constraint::Length(2),  // Spacer
                Constraint::Length(8),  // Large timer box
                Constraint::Length(2),  // Spacer
                Constraint::Length(8),  // Session details
                Constraint::Min(0),     // Bottom spacer
                Constraint::Length(1),  // Footer
            ])
            .split(f.size());

        // Top header with ESC hint
        let header_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(100)])
            .split(chunks[0]);

        f.render_widget(
            Paragraph::new("Press ESC to exit focused mode.")
                .alignment(Alignment::Center)
                .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
            header_layout[0],
        );

        if let (Some(session), Some(project)) = (&self.current_session, &self.current_project) {
            // Project info box
            let project_area = self.centered_rect(60, 20, chunks[2]);
            let project_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ColorScheme::GRAY_TEXT))
                .style(Style::default().bg(ColorScheme::CLEAN_BG));

            let project_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .margin(1)
                .split(project_area);

            f.render_widget(project_block, project_area);

            // Project name
            f.render_widget(
                Paragraph::new(project.name.clone())
                    .alignment(Alignment::Center)
                    .style(
                        Style::default()
                            .fg(ColorScheme::WHITE_TEXT)
                            .add_modifier(Modifier::BOLD),
                    ),
                project_layout[0],
            );

            // Project description or refactor info
            let default_description = "Refactor authentication module".to_string();
            let description = project
                .description
                .as_ref()
                .unwrap_or(&default_description);
            f.render_widget(
                Paragraph::new(description.clone())
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
                project_layout[1],
            );

            // Large timer box
            let timer_area = self.centered_rect(40, 20, chunks[4]);
            let timer_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ColorScheme::CLEAN_GREEN))
                .style(Style::default().bg(Color::Black));

            let timer_inner = timer_block.inner(timer_area);
            f.render_widget(timer_block, timer_area);

            // Calculate and display large timer
            let now = Local::now();
            let elapsed_seconds = (now.timestamp() - session.start_time.timestamp())
                - session.paused_duration.num_seconds();
            let duration_str = Formatter::format_duration_clock(elapsed_seconds);

            f.render_widget(
                Paragraph::new(duration_str)
                    .alignment(Alignment::Center)
                    .style(
                        Style::default()
                            .fg(ColorScheme::CLEAN_GREEN)
                            .add_modifier(Modifier::BOLD),
                    ),
                timer_inner,
            );

            // Session details box
            let details_area = self.centered_rect(60, 25, chunks[6]);
            let details_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ColorScheme::GRAY_TEXT))
                .style(Style::default().bg(ColorScheme::CLEAN_BG));

            let details_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // Start time
                    Constraint::Length(2), // Session type
                    Constraint::Length(2), // Tags
                ])
                .margin(1)
                .split(details_area);

            f.render_widget(details_block, details_area);

            // Start time
            let start_time_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(details_layout[0]);

            f.render_widget(
                Paragraph::new("Start Time")
                    .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
                start_time_layout[0],
            );
            f.render_widget(
                Paragraph::new(session.start_time.with_timezone(&Local).format("%H:%M").to_string())
                    .alignment(Alignment::Right)
                    .style(Style::default().fg(ColorScheme::WHITE_TEXT)),
                start_time_layout[1],
            );

            // Session type
            let session_type_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(details_layout[1]);

            f.render_widget(
                Paragraph::new("Session Type")
                    .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
                session_type_layout[0],
            );
            f.render_widget(
                Paragraph::new("Deep Work")
                    .alignment(Alignment::Right)
                    .style(Style::default().fg(ColorScheme::WHITE_TEXT)),
                session_type_layout[1],
            );

            // Tags
            let tags_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
                .split(details_layout[2]);

            f.render_widget(
                Paragraph::new("Tags")
                    .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
                tags_layout[0],
            );

            // Create tag spans
            let tag_spans = vec![
                Span::styled(
                    " Backend ",
                    Style::default()
                        .fg(ColorScheme::CLEAN_BG)
                        .bg(ColorScheme::GRAY_TEXT),
                ),
                Span::raw(" "),
                Span::styled(
                    " Refactor ",
                    Style::default()
                        .fg(ColorScheme::CLEAN_BG)
                        .bg(ColorScheme::GRAY_TEXT),
                ),
                Span::raw(" "),
                Span::styled(
                    " Security ",
                    Style::default()
                        .fg(ColorScheme::CLEAN_BG)
                        .bg(ColorScheme::GRAY_TEXT),
                ),
            ];

            f.render_widget(
                Paragraph::new(Line::from(tag_spans))
                    .alignment(Alignment::Right),
                tags_layout[1],
            );

        } else {
            // No active session - show idle state
            let idle_area = self.centered_rect(50, 20, chunks[4]);
            let idle_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ColorScheme::GRAY_TEXT))
                .style(Style::default().bg(ColorScheme::CLEAN_BG));

            f.render_widget(idle_block.clone(), idle_area);

            let idle_inner = idle_block.inner(idle_area);
            f.render_widget(
                Paragraph::new("No Active Session\n\nPress 's' to start tracking")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
                idle_inner,
            );
        }
    }

    fn render_overview_dashboard(&mut self, f: &mut Frame) {
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
        SessionStatsWidget::render(daily_stats, self.weekly_stats, chunks[5], f.buffer_mut());
        self.render_quick_stats(f, chunks[4], chunks[5], daily_stats);

        // 3. Recent Projects & Timeline
        self.render_projects_and_timeline(f, chunks[5]);

        // 4. Bottom Bar
        self.render_keyboard_hints(chunks[8], f.buffer_mut());

        self.render_bottom_bar(f, chunks[6]);
    }

    fn render_history_browser(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(f.size());

        let left_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Length(8),  // Filters
                Constraint::Min(10),    // Session list
            ])
            .split(chunks[0]);

        let right_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(60), // Session details
                Constraint::Length(4),      // Action buttons
                Constraint::Min(0),         // Summary
            ])
            .split(chunks[1]);

        // Header
        f.render_widget(
            Paragraph::new("Tempo TUI :: History Browser")
                .style(Style::default().fg(ColorScheme::CLEAN_BLUE).add_modifier(Modifier::BOLD))
                .block(
                    Block::default()
                        .borders(Borders::BOTTOM)
                        .border_style(Style::default().fg(ColorScheme::GRAY_TEXT)),
                ),
            left_chunks[0],
        );

        // Filters panel
        self.render_history_filters(f, left_chunks[1]);

        // Session list
        self.render_session_list(f, left_chunks[2]);

        // Session details
        self.render_session_details(f, right_chunks[0]);

        // Action buttons
        self.render_session_actions(f, right_chunks[1]);

        // Summary
        self.render_history_summary(f, right_chunks[2]);
    }

    fn render_history_filters(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Filters ")
            .border_style(Style::default().fg(ColorScheme::GRAY_TEXT));

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        let filter_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Date filters
                Constraint::Length(1), // Project filter
                Constraint::Length(1), // Duration filter
                Constraint::Length(1), // Search
            ])
            .split(inner_area);

        // Date range
        let date_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(35), Constraint::Percentage(35)])
            .split(filter_chunks[0]);

        f.render_widget(
            Paragraph::new("Start Date\nEnd Date")
                .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
            date_layout[0],
        );
        f.render_widget(
            Paragraph::new("2023-10-01\n2023-10-31")
                .style(Style::default().fg(ColorScheme::WHITE_TEXT)),
            date_layout[1],
        );
        f.render_widget(
            Paragraph::new("Project\nDuration Filter")
                .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
            date_layout[2],
        );

        // Project filter
        let project_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(15), Constraint::Min(0)])
            .split(filter_chunks[1]);

        f.render_widget(
            Paragraph::new("Project")
                .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
            project_layout[0],
        );
        f.render_widget(
            Paragraph::new("Filter by project ▼")
                .style(Style::default().fg(ColorScheme::WHITE_TEXT)),
            project_layout[1],
        );

        // Duration filter
        let duration_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(15), Constraint::Min(0)])
            .split(filter_chunks[2]);

        f.render_widget(
            Paragraph::new("Duration Filter")
                .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
            duration_layout[0],
        );
        f.render_widget(
            Paragraph::new(">1h, <30m")
                .style(Style::default().fg(ColorScheme::WHITE_TEXT)),
            duration_layout[1],
        );

        // Free-text search
        let search_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(15), Constraint::Min(0)])
            .split(filter_chunks[3]);

        f.render_widget(
            Paragraph::new("Free-text Search")
                .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
            search_layout[0],
        );

        let search_style = if self.filter_input_mode {
            Style::default().fg(ColorScheme::CLEAN_BLUE)
        } else {
            Style::default().fg(ColorScheme::WHITE_TEXT)
        };

        let search_text = if self.session_filter.search_text.is_empty() {
            "Search session notes and context..."
        } else {
            &self.session_filter.search_text
        };

        f.render_widget(
            Paragraph::new(search_text)
                .style(search_style),
            search_layout[1],
        );
    }

    fn render_session_list(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ColorScheme::GRAY_TEXT));

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        // Header row
        let header_row = Row::new(vec![
            Cell::from("DATE").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("PROJECT").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("DURATION").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("START").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("END").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("STATUS").style(Style::default().add_modifier(Modifier::BOLD)),
        ])
        .style(Style::default().fg(ColorScheme::GRAY_TEXT))
        .bottom_margin(1);

        // Create sample session rows
        let rows: Vec<Row> = self.history_sessions
            .iter()
            .enumerate()
            .map(|(i, session)| {
                let is_selected = i == self.selected_session_index;
                let style = if is_selected {
                    Style::default().bg(ColorScheme::CLEAN_BLUE).fg(Color::Black)
                } else {
                    Style::default().fg(ColorScheme::WHITE_TEXT)
                };

                let status = if session.end_time.is_some() {
                    "[✓] Completed"
                } else {
                    "[▶] Running"
                };

                let start_time = session.start_time.with_timezone(&Local).format("%H:%M").to_string();
                let end_time = if let Some(end) = session.end_time {
                    end.with_timezone(&Local).format("%H:%M").to_string()
                } else {
                    "--:--".to_string()
                };

                let duration = if let Some(_) = session.end_time {
                    let duration_secs = (session.start_time.timestamp() - session.start_time.timestamp()).abs();
                    Formatter::format_duration(duration_secs)
                } else {
                    "0h 0m".to_string()
                };

                Row::new(vec![
                    Cell::from(session.start_time.with_timezone(&Local).format("%Y-%m-%d").to_string()),
                    Cell::from("Project Phoenix"), // TODO: Get actual project name
                    Cell::from(duration),
                    Cell::from(start_time),
                    Cell::from(end_time),
                    Cell::from(status),
                ])
                .style(style)
            })
            .collect();

        if rows.is_empty() {
            // Show sample data
            let sample_rows = vec![
                Row::new(vec![
                    Cell::from("2023-10-26"),
                    Cell::from("Project Phoenix"),
                    Cell::from("2h 15m"),
                    Cell::from("09:03"),
                    Cell::from("11:18"),
                    Cell::from("[✓] Completed"),
                ]).style(Style::default().bg(ColorScheme::CLEAN_BLUE).fg(Color::Black)),
                Row::new(vec![
                    Cell::from("2023-10-26"),
                    Cell::from("Internal Tools"),
                    Cell::from("0h 45m"),
                    Cell::from("11:30"),
                    Cell::from("12:15"),
                    Cell::from("[✓] Completed"),
                ]),
                Row::new(vec![
                    Cell::from("2023-10-25"),
                    Cell::from("Project Phoenix"),
                    Cell::from("4h 05m"),
                    Cell::from("13:00"),
                    Cell::from("17:05"),
                    Cell::from("[✓] Completed"),
                ]),
                Row::new(vec![
                    Cell::from("2023-10-25"),
                    Cell::from("Client Support"),
                    Cell::from("1h 00m"),
                    Cell::from("10:00"),
                    Cell::from("11:00"),
                    Cell::from("[✓] Completed"),
                ]),
                Row::new(vec![
                    Cell::from("2023-10-24"),
                    Cell::from("Project Phoenix"),
                    Cell::from("8h 00m"),
                    Cell::from("09:00"),
                    Cell::from("17:00"),
                    Cell::from("[✓] Completed"),
                ]),
                Row::new(vec![
                    Cell::from("2023-10-27"),
                    Cell::from("Project Nova"),
                    Cell::from("0h 22m"),
                    Cell::from("14:00"),
                    Cell::from("--:--"),
                    Cell::from("[▶] Running"),
                ]),
            ];

            let table = Table::new(sample_rows)
                .header(header_row)
                .widths(&[
                    Constraint::Length(12),
                    Constraint::Min(15),
                    Constraint::Length(10),
                    Constraint::Length(8),
                    Constraint::Length(8),
                    Constraint::Min(12),
                ]);

            f.render_widget(table, inner_area);
        }
    }

    fn render_session_details(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Session Details ")
            .border_style(Style::default().fg(ColorScheme::GRAY_TEXT));

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        let details_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Session notes
                Constraint::Length(2), // Tags
                Constraint::Length(3), // Context
            ])
            .split(inner_area);

        // Session notes
        f.render_widget(
            Paragraph::new("SESSION NOTES\n\nWorked on the new authentication flow.\nImplemented JWT token refresh logic and fixed\nthe caching issue on the user profile page.\nReady for QA review.")
                .style(Style::default().fg(ColorScheme::WHITE_TEXT))
                .wrap(ratatui::widgets::Wrap { trim: true }),
            details_chunks[0],
        );

        // Tags
        let tag_spans = vec![
            Span::styled(
                " #backend ",
                Style::default()
                    .fg(Color::Black)
                    .bg(ColorScheme::CLEAN_BLUE),
            ),
            Span::raw(" "),
            Span::styled(
                " #auth ",
                Style::default()
                    .fg(Color::Black)
                    .bg(ColorScheme::CLEAN_BLUE),
            ),
            Span::raw(" "),
            Span::styled(
                " #bugfix ",
                Style::default()
                    .fg(Color::Black)
                    .bg(ColorScheme::CLEAN_BLUE),
            ),
        ];

        f.render_widget(
            Paragraph::new(vec![
                Line::from("TAGS"),
                Line::from(tag_spans),
            ])
            .style(Style::default().fg(ColorScheme::WHITE_TEXT)),
            details_chunks[1],
        );

        // Context
        let context_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(details_chunks[2]);

        f.render_widget(
            Paragraph::new("CONTEXT")
                .style(Style::default().fg(ColorScheme::WHITE_TEXT)),
            context_chunks[0],
        );

        let context_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(context_chunks[1]);

        f.render_widget(
            Paragraph::new("Git\nBranch:\nIssue ID:\nCommit:")
                .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
            context_layout[0],
        );
        f.render_widget(
            Paragraph::new("feature/PHX-123-auth\nPHX-123\na1b2c3d")
                .style(Style::default().fg(ColorScheme::WHITE_TEXT)),
            context_layout[1],
        );
    }

    fn render_session_actions(&self, f: &mut Frame, area: Rect) {
        let button_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(area);

        let buttons = [
            ("[ Edit ]", ColorScheme::GRAY_TEXT),
            ("[ Duplicate ]", ColorScheme::GRAY_TEXT),
            ("[ Delete ]", Color::Red),
            ("", ColorScheme::GRAY_TEXT),
        ];

        for (i, (text, color)) in buttons.iter().enumerate() {
            if !text.is_empty() {
                f.render_widget(
                    Paragraph::new(*text)
                        .alignment(Alignment::Center)
                        .style(Style::default().fg(*color)),
                    button_layout[i],
                );
            }
        }
    }

    fn render_history_summary(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Summary ")
            .border_style(Style::default().fg(ColorScheme::GRAY_TEXT));

        let inner_area = block.inner(area);
        f.render_widget(block, area);

        f.render_widget(
            Paragraph::new("Showing 7 of 128 sessions. Total Duration: 17h 40m")
                .style(Style::default().fg(ColorScheme::WHITE_TEXT))
                .alignment(Alignment::Center),
            inner_area,
        );
    }

    fn render_project_grid(&mut self, f: &mut Frame) {
        let area = f.size();
        
        let main_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Header
                Constraint::Min(10),    // Project grid
                Constraint::Length(3),  // Stats summary
                Constraint::Length(1),  // Bottom hints
            ])
            .split(area);

        // Header
        f.render_widget(
            Paragraph::new("Project Dashboard")
                .style(Style::default().fg(ColorScheme::CLEAN_BLUE).add_modifier(Modifier::BOLD))
                .block(
                    Block::default()
                        .borders(Borders::BOTTOM)
                        .border_style(Style::default().fg(ColorScheme::GRAY_TEXT)),
                ),
            main_layout[0],
        );

        // Project grid area
        self.render_project_cards(f, main_layout[1]);

        // Stats summary
        self.render_project_stats_summary(f, main_layout[2]);

        // Bottom hints
        let hints = vec![
            ("↑/↓/←/→", "Navigate"),
            ("Enter", "Select"),
            ("Tab", "Next View"),
            ("q", "Quit"),
        ];

        let spans: Vec<Span> = hints
            .iter()
            .flat_map(|(key, desc)| {
                vec![
                    Span::styled(
                        format!(" {} ", key),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(format!("{} ", desc), Style::default().fg(Color::DarkGray)),
                ]
            })
            .collect();

        let line = Line::from(spans);
        let block = Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(Color::DarkGray));
        Paragraph::new(line).block(block).render(main_layout[3], f.buffer_mut());
    }

    fn render_project_cards(&mut self, f: &mut Frame, area: Rect) {
        if self.available_projects.is_empty() {
            // Show empty state
            let empty_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ColorScheme::GRAY_TEXT))
                .title(" No Projects Found ");
            
            let empty_area = self.centered_rect(50, 30, area);
            f.render_widget(empty_block.clone(), empty_area);
            
            let inner = empty_block.inner(empty_area);
            f.render_widget(
                Paragraph::new("No projects available.\n\nStart a session to create a project.")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(ColorScheme::GRAY_TEXT)),
                inner,
            );
            return;
        }

        // Calculate grid layout
        let margin = 2;
        let card_height = 8;
        let card_spacing = 1;
        
        // Calculate how many rows we can fit
        let available_height = area.height.saturating_sub(margin * 2);
        let total_rows = (self.available_projects.len() + self.projects_per_row - 1) / self.projects_per_row;
        let visible_rows = (available_height / (card_height + card_spacing)).min(total_rows as u16) as usize;
        
        // Render visible rows
        for row in 0..visible_rows {
            let y_offset = margin + row as u16 * (card_height + card_spacing);
            
            // Create horizontal layout for this row
            let row_area = Rect::new(area.x, area.y + y_offset, area.width, card_height);
            let card_constraints = vec![Constraint::Percentage(100 / self.projects_per_row as u16); self.projects_per_row];
            let row_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(card_constraints)
                .margin(1)
                .split(row_area);
            
            // Render cards in this row
            for col in 0..self.projects_per_row {
                let project_index = row * self.projects_per_row + col;
                if project_index >= self.available_projects.len() {
                    break;
                }
                
                let is_selected = row == self.selected_project_row && col == self.selected_project_col;
                self.render_project_card(f, row_layout[col], project_index, is_selected);
            }
        }
    }

    fn render_project_card(&self, f: &mut Frame, area: Rect, project_index: usize, is_selected: bool) {
        if let Some(project) = self.available_projects.get(project_index) {
            // Card styling based on selection
            let border_style = if is_selected {
                Style::default().fg(ColorScheme::CLEAN_BLUE)
            } else {
                Style::default().fg(ColorScheme::GRAY_TEXT)
            };
            
            let bg_color = if is_selected {
                ColorScheme::CLEAN_BG
            } else {
                Color::Black
            };

            let card_block = Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(Style::default().bg(bg_color));

            f.render_widget(card_block.clone(), area);

            let inner_area = card_block.inner(area);
            let card_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Project name
                    Constraint::Length(1), // Path
                    Constraint::Length(1), // Spacer
                    Constraint::Length(2), // Stats
                    Constraint::Length(1), // Status
                ])
                .split(inner_area);

            // Project name
            let name = if project.name.len() > 20 {
                format!("{}...", &project.name[..17])
            } else {
                project.name.clone()
            };
            
            f.render_widget(
                Paragraph::new(name)
                    .style(Style::default().fg(ColorScheme::WHITE_TEXT).add_modifier(Modifier::BOLD))
                    .alignment(Alignment::Center),
                card_layout[0],
            );

            // Path (shortened)
            let path_str = project.path.to_string_lossy();
            let short_path = if path_str.len() > 25 {
                format!("...{}", &path_str[path_str.len()-22..])
            } else {
                path_str.to_string()
            };
            
            f.render_widget(
                Paragraph::new(short_path)
                    .style(Style::default().fg(ColorScheme::GRAY_TEXT))
                    .alignment(Alignment::Center),
                card_layout[1],
            );

            // Stats placeholder - in real implementation, we'd fetch these
            let stats_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(card_layout[3]);

            f.render_widget(
                Paragraph::new("Sessions\n42")
                    .style(Style::default().fg(ColorScheme::WHITE_TEXT))
                    .alignment(Alignment::Center),
                stats_layout[0],
            );

            f.render_widget(
                Paragraph::new("Time\n24h 15m")
                    .style(Style::default().fg(ColorScheme::WHITE_TEXT))
                    .alignment(Alignment::Center),
                stats_layout[1],
            );

            // Status
            let status = if project.is_archived {
                (" Archived ", Color::Red)
            } else {
                (" Active ", ColorScheme::CLEAN_GREEN)
            };

            f.render_widget(
                Paragraph::new(status.0)
                    .style(Style::default().fg(status.1))
                    .alignment(Alignment::Center),
                card_layout[4],
            );
        }
    }

    fn render_project_stats_summary(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Summary ")
            .border_style(Style::default().fg(ColorScheme::GRAY_TEXT));

        f.render_widget(block.clone(), area);

        let inner = block.inner(area);
        let stats_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ])
            .split(inner);

        let total_projects = self.available_projects.len();
        let active_projects = self.available_projects.iter().filter(|p| !p.is_archived).count();
        let archived_projects = total_projects - active_projects;

        let stats = [
            ("Total Projects", total_projects.to_string()),
            ("Active", active_projects.to_string()),
            ("Archived", archived_projects.to_string()),
            ("Selected", format!("{}/{}", 
                self.selected_project_row * self.projects_per_row + self.selected_project_col + 1, 
                total_projects)),
        ];

        for (i, (label, value)) in stats.iter().enumerate() {
            let content = Paragraph::new(vec![
                Line::from(Span::styled(*label, Style::default().fg(ColorScheme::GRAY_TEXT))),
                Line::from(Span::styled(
                    value.as_str(),
                    Style::default().fg(ColorScheme::WHITE_TEXT).add_modifier(Modifier::BOLD),
                )),
            ])
            .alignment(Alignment::Center);

            f.render_widget(content, stats_layout[i]);
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
            let duration_str = Formatter::format_duration_clock(elapsed_seconds);

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
            ("This Week", Formatter::format_duration(self.weekly_stats)),
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

        let list_area = chunks[1];
        let items_area = Rect::new(list_area.x, list_area.y, list_area.width, list_area.height);

        // Recent Projects Table
        let header = Row::new(vec![
            Cell::from("Project").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Today").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Total").style(Style::default().add_modifier(Modifier::BOLD)),
            Cell::from("Last Active").style(Style::default().add_modifier(Modifier::BOLD)),
        ])
        .style(Style::default().fg(ColorScheme::CLEAN_BLUE))
        .bottom_margin(1);

        let items: Vec<Row> = self
            .recent_projects
            .iter()
            .map(|p| {
                let last_active = if let Some(date) = p.last_active {
                    let now = chrono::Utc::now();
                    let diff = now - date;
                    if diff.num_days() > 0 {
                        format!("{}d ago", diff.num_days())
                    } else if diff.num_hours() > 0 {
                        format!("{}h ago", diff.num_hours())
                    } else {
                        format!("{}m ago", diff.num_minutes())
                    }
                } else {
                    "-".to_string()
                };

                Row::new(vec![
                    Cell::from(p.project.name.clone()),
                    Cell::from(Formatter::format_duration(p.today_seconds)),
                    Cell::from(Formatter::format_duration(p.total_seconds)),
                    Cell::from(last_active),
                ])
            })
            .collect();

        let table = Table::new(items)
            .header(header)
            .block(Block::default().borders(Borders::NONE))
            .widths(&[
                Constraint::Percentage(40),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
                Constraint::Percentage(20),
            ]);

        f.render_widget(table, items_area);

        // Timeline Header
        f.render_widget(
            Paragraph::new("Activity Timeline").style(
                Style::default()
                    .fg(ColorScheme::WHITE_TEXT)
                    .add_modifier(Modifier::BOLD),
            ),
            chunks[2],
        );

        // Timeline Bar
        let timeline_area = chunks[3];
        let bar_area = Rect::new(timeline_area.x, timeline_area.y, timeline_area.width, 1);

        // Background
        f.render_widget(
            Block::default().style(Style::default().bg(ColorScheme::GRAY_TEXT)),
            bar_area,
        );

        // Sessions
        let width = bar_area.width as f64;
        for session in &self.today_sessions {
            let start = session.start_time.with_timezone(&Local).time();
            let start_seconds = start.num_seconds_from_midnight() as f64;

            let duration = if let Some(end) = session.end_time {
                (end - session.start_time).num_seconds() as f64
            } else {
                (Local::now().signed_duration_since(session.start_time.with_timezone(&Local)))
                    .num_seconds() as f64
            };

            // Subtract paused duration
            let duration = duration - session.paused_duration.num_seconds() as f64;

            let x_offset = (start_seconds / 86400.0) * width;
            let bar_width = (duration / 86400.0) * width;

            if bar_width > 0.0 {
                let x_pos = bar_area.x + x_offset as u16;
                // Ensure we don't draw outside bounds or wrap
                if x_pos < bar_area.x + bar_area.width {
                    let w = (bar_width.max(1.0) as u16).min(bar_area.width - (x_pos - bar_area.x));
                    let session_rect = Rect::new(x_pos, bar_area.y, w, 1);
                    f.render_widget(
                        Block::default().style(Style::default().bg(ColorScheme::CLEAN_BLUE)),
                        session_rect,
                    );
                }
            }
        }

        // Labels
        let label_y = timeline_area.y + 1;
        f.render_widget(
            Paragraph::new("00:00").style(Style::default().fg(ColorScheme::GRAY_TEXT)),
            Rect::new(timeline_area.x, label_y, 5, 1),
        );
        f.render_widget(
            Paragraph::new("12:00")
                .style(Style::default().fg(ColorScheme::GRAY_TEXT))
                .alignment(Alignment::Center),
            Rect::new(timeline_area.x, label_y, timeline_area.width, 1),
        );
        f.render_widget(
            Paragraph::new("24:00")
                .style(Style::default().fg(ColorScheme::GRAY_TEXT))
                .alignment(Alignment::Right),
            Rect::new(timeline_area.x, label_y, timeline_area.width, 1),
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

    async fn get_today_sessions(&mut self) -> Result<Vec<Session>> {
        if !is_daemon_running() {
            return Ok(Vec::new());
        }

        self.ensure_connected().await?;

        let today = chrono::Local::now().date_naive();
        let response = self
            .client
            .send_message(&IpcMessage::GetSessionsForDate(today))
            .await?;
        match response {
            IpcResponse::SessionList(sessions) => Ok(sessions),
            IpcResponse::Error(e) => Err(anyhow::anyhow!("Failed to get sessions: {}", e)),
            _ => Ok(Vec::new()),
        }
    }

    async fn get_history_sessions(&mut self) -> Result<Vec<Session>> {
        if !is_daemon_running() {
            return Ok(Vec::new());
        }

        self.ensure_connected().await?;

        // For now, get a date range of the last 30 days
        let end_date = chrono::Local::now().date_naive();
        let _start_date = end_date - chrono::Duration::days(30);
        
        // Get sessions for the date range (simplified - in a real implementation, 
        // this would use a new IPC message like GetSessionsInRange)
        let mut all_sessions = Vec::new();
        for days_ago in 0..30 {
            let date = end_date - chrono::Duration::days(days_ago);
            if let Ok(IpcResponse::SessionList(sessions)) = self
                .client
                .send_message(&IpcMessage::GetSessionsForDate(date))
                .await
            {
                all_sessions.extend(sessions);
            }
        }

        // Apply filters
        let filtered_sessions: Vec<Session> = all_sessions
            .into_iter()
            .filter(|session| {
                // Apply search filter if set
                if !self.session_filter.search_text.is_empty() {
                    if let Some(notes) = &session.notes {
                        if !notes.to_lowercase().contains(&self.session_filter.search_text.to_lowercase()) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }
                true
            })
            .collect();

        Ok(filtered_sessions)
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
