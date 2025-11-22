use chrono::{DateTime, Local};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};
use std::time::{Duration, Instant};

use crate::ui::formatter::Formatter;

pub struct StatusWidget;
pub struct ProgressWidget;
pub struct SummaryWidget;

// Centralized color scheme with a Neon/Cyberpunk aesthetic
pub struct ColorScheme;

impl ColorScheme {
    // Vibrant Colors
    pub const NEON_CYAN: Color = Color::Rgb(0, 255, 255);
    pub const NEON_GREEN: Color = Color::Rgb(57, 255, 20);
    pub const NEON_PINK: Color = Color::Rgb(255, 16, 240);
    pub const NEON_PURPLE: Color = Color::Rgb(188, 19, 254);
    pub const NEON_YELLOW: Color = Color::Rgb(255, 240, 31);
    pub const DARK_BG: Color = Color::Rgb(10, 10, 15);
    pub const GRAY_TEXT: Color = Color::Rgb(160, 160, 160);
    pub const WHITE_TEXT: Color = Color::Rgb(240, 240, 240);

    // Professional clean palette
    pub const CLEAN_BG: Color = Color::Rgb(20, 20, 20);
    pub const CLEAN_ACCENT: Color = Color::Rgb(217, 119, 87); // Terracotta-ish
    pub const CLEAN_BLUE: Color = Color::Rgb(100, 150, 255);
    pub const CLEAN_GREEN: Color = Color::Rgb(100, 200, 100);
    pub const CLEAN_GOLD: Color = Color::Rgb(217, 179, 87);
    pub const CLEAN_MAGENTA: Color = Color::Rgb(188, 19, 254);

    pub fn get_context_color(context: &str) -> Color {
        match context {
            "terminal" => Self::NEON_CYAN,
            "ide" => Self::NEON_PURPLE,
            "linked" => Self::NEON_YELLOW,
            "manual" => Color::Blue,
            _ => Self::WHITE_TEXT,
        }
    }

    pub fn active_status() -> Color {
        Self::NEON_GREEN
    }
    pub fn project_name() -> Color {
        Self::NEON_YELLOW
    }
    pub fn duration() -> Color {
        Self::NEON_CYAN
    }
    pub fn path() -> Color {
        Self::GRAY_TEXT
    }
    pub fn timestamp() -> Color {
        Self::GRAY_TEXT
    }
    pub fn border() -> Color {
        Self::NEON_PURPLE
    }
    pub fn title() -> Color {
        Self::NEON_PINK
    }

    pub fn base_block() -> Block<'static> {
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Self::border()))
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(Self::DARK_BG))
    }

    pub fn clean_block() -> Block<'static> {
        Block::default()
            .borders(Borders::NONE)
            .style(Style::default().bg(Self::DARK_BG))
    }
}

pub struct Spinner {
    frames: Vec<&'static str>,
    current_frame: usize,
    last_update: Instant,
    interval: Duration,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            frames: vec!["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            current_frame: 0,
            last_update: Instant::now(),
            interval: Duration::from_millis(100),
        }
    }

    pub fn with_speed(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    pub fn next(&mut self) {
        if self.last_update.elapsed() >= self.interval {
            self.current_frame = (self.current_frame + 1) % self.frames.len();
            self.last_update = Instant::now();
        }
    }

    pub fn current(&self) -> &str {
        self.frames[self.current_frame]
    }
}

pub struct Throbber {
    frames: Vec<&'static str>,
    current: usize,
}

impl Throbber {
    pub fn new() -> Self {
        Self {
            // A horizontal throbber using ASCII
            frames: vec![
                "[=    ]", "[ =   ]", "[  =  ]", "[   = ]", "[    =]", "[   = ]", "[  =  ]",
                "[ =   ]",
            ],
            current: 0,
        }
    }

    pub fn next(&mut self) -> &'static str {
        let frame = self.frames[self.current];
        self.current = (self.current + 1) % self.frames.len();
        frame
    }

    pub fn current(&self) -> &'static str {
        self.frames[self.current]
    }
}

impl StatusWidget {
    pub fn render_status_text(
        project_name: &str,
        duration: i64,
        start_time: &str,
        context: &str,
    ) -> String {
        format!(
            "ACTIVE | {} | Time: {} | Started: {} | Context: {}",
            project_name,
            Formatter::format_duration(duration),
            start_time,
            context
        )
    }

    pub fn render_idle_text() -> String {
        "IDLE | No active time tracking session | Use 'tempo session start' to begin tracking"
            .to_string()
    }
}

impl ProgressWidget {
    pub fn calculate_daily_progress(completed_seconds: i64, target_hours: f64) -> u16 {
        let total_hours = completed_seconds as f64 / 3600.0;
        let progress = (total_hours / target_hours * 100.0).min(100.0) as u16;
        progress
    }

    pub fn format_progress_label(completed_seconds: i64, target_hours: f64) -> String {
        let total_hours = completed_seconds as f64 / 3600.0;
        let progress = (total_hours / target_hours * 100.0).min(100.0) as u16;
        format!(
            "Daily Progress ({:.1}h / {:.1}h) - {}%",
            total_hours, target_hours, progress
        )
    }
}

impl SummaryWidget {
    pub fn format_project_summary(
        project_name: &str,
        total_time: i64,
        session_count: usize,
        active_count: usize,
    ) -> String {
        format!(
            "Project: {} | Total Time: {} | Sessions: {} total, {} active",
            project_name,
            Formatter::format_duration(total_time),
            session_count,
            active_count
        )
    }
    pub fn format_session_line(
        start_time: &DateTime<Local>,
        duration: i64,
        project_name: &str,
        status: &str,
    ) -> String {
        format!(
            "{} - {} ({}) [{}]",
            start_time.format("%H:%M"),
            project_name,
            Formatter::format_duration(duration),
            status
        )
    }
}

#[allow(dead_code)]
pub enum StatusIndicator {
    Online,
    Offline,
    Syncing,
    Error,
    Custom(String, Color),
}

impl StatusIndicator {
    #[allow(dead_code)]
    pub fn render(&self) -> Span {
        match self {
            StatusIndicator::Online => Span::styled("●", Style::default().fg(Color::Green)),
            StatusIndicator::Offline => Span::styled("○", Style::default().fg(Color::Gray)),
            StatusIndicator::Syncing => Span::styled("⟳", Style::default().fg(Color::Blue)),
            StatusIndicator::Error => Span::styled("⚠", Style::default().fg(Color::Red)),
            StatusIndicator::Custom(symbol, color) => {
                Span::styled(symbol.clone(), Style::default().fg(*color))
            }
        }
    }
}

#[allow(dead_code)]
pub struct GradientProgressBar;

impl GradientProgressBar {
    #[allow(dead_code)]
    pub fn get_color(progress: u16) -> Color {
        match progress {
            0..=25 => Color::Red,
            26..=50 => Color::Yellow,
            51..=75 => Color::Green,
            _ => Color::Cyan,
        }
    }

    #[allow(dead_code)]
    pub fn render(progress: u16, width: u16) -> Line<'static> {
        let filled_width = (width as f64 * (progress as f64 / 100.0)).round() as u16;
        let empty_width = width.saturating_sub(filled_width);

        let color = Self::get_color(progress);

        let filled = Span::styled(
            "█".repeat(filled_width as usize),
            Style::default().fg(color),
        );
        let empty = Span::styled(
            "░".repeat(empty_width as usize),
            Style::default().fg(Color::DarkGray),
        );

        Line::from(vec![filled, empty])
    }
}

#[allow(dead_code)]
pub struct SessionStatsWidget;

impl SessionStatsWidget {
    #[allow(dead_code)]
    pub fn render(
        daily_stats: &(i64, i64, i64), // (sessions, total_time, active_time)
        weekly_total: i64,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let (daily_sessions, daily_total, _) = daily_stats;

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ColorScheme::border()))
            .title(Span::styled(
                " Session Stats ",
                Style::default().fg(ColorScheme::title()),
            ));

        let inner_area = block.inner(area);
        block.render(area, buf);

        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Daily
                Constraint::Length(1), // Weekly
                Constraint::Min(0),
            ])
            .split(inner_area);

        // Daily Stats
        let daily_text = Line::from(vec![
            Span::styled("Today: ", Style::default().fg(ColorScheme::GRAY_TEXT)),
            Span::styled(
                format!("{} sessions, ", daily_sessions),
                Style::default().fg(ColorScheme::WHITE_TEXT),
            ),
            Span::styled(
                Formatter::format_duration(*daily_total),
                Style::default().fg(ColorScheme::NEON_CYAN),
            ),
        ]);
        Paragraph::new(daily_text).render(layout[0], buf);

        // Weekly Stats
        let weekly_text = Line::from(vec![
            Span::styled("This Week: ", Style::default().fg(ColorScheme::GRAY_TEXT)),
            Span::styled(
                Formatter::format_duration(weekly_total),
                Style::default().fg(ColorScheme::NEON_PURPLE),
            ),
        ]);
        Paragraph::new(weekly_text).render(layout[1], buf);
    }
}
