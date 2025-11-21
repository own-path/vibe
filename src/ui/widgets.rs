use chrono::{DateTime, Local};
use ratatui::{
    style::{Color, Style},
    widgets::{Block, BorderType, Borders},
};

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
    current: usize,
}

impl Spinner {
    pub fn new() -> Self {
        Self {
            // Using a simple line spinner that is ASCII safe but looks good
            frames: vec!["|", "/", "-", "\\"],
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
        context: &str,
        is_active: bool,
    ) -> String {
        let status_char = if is_active { "*" } else { "+" };
        let duration_str = if is_active {
            format!("{} (active)", Formatter::format_duration(duration))
        } else {
            Formatter::format_duration(duration)
        };

        format!(
            "{} {} | {} | {}",
            status_char,
            start_time.format("%H:%M:%S"),
            duration_str,
            context
        )
    }
}
