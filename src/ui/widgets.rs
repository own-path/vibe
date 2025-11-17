use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph, Table, Row, Cell, List, ListItem},
};
use chrono::{DateTime, Local};

use crate::{
    models::{Project, Session},
    ui::formatter::Formatter,
};

pub struct StatusWidget;
pub struct ProgressWidget;
pub struct SummaryWidget;

// Centralized color scheme
pub struct ColorScheme;

impl ColorScheme {
    pub fn get_context_color(context: &str) -> Color {
        match context {
            "terminal" => Color::Cyan,
            "ide" => Color::Magenta, 
            "linked" => Color::Yellow,
            "manual" => Color::Blue,
            _ => Color::White,
        }
    }

    pub fn active_status() -> Color { Color::Green }
    pub fn project_name() -> Color { Color::Yellow }
    pub fn duration() -> Color { Color::Green }
    pub fn path() -> Color { Color::Gray }
    pub fn timestamp() -> Color { Color::Gray }
    pub fn border() -> Color { Color::Cyan }
}

impl StatusWidget {
    pub fn render_status_text(project_name: &str, duration: i64, start_time: &str, context: &str) -> String {
        format!(
            "● ACTIVE | {} | Time: {} | Started: {} | Context: {}",
            project_name,
            Formatter::format_duration(duration),
            start_time,
            context
        )
    }

    pub fn render_idle_text() -> String {
        "○ IDLE | No active time tracking session | Use 'vibe session start' to begin tracking".to_string()
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
        format!("Daily Progress ({:.1}h / {:.1}h) - {}%", total_hours, target_hours, progress)
    }
}

impl SummaryWidget {
    pub fn format_project_summary(project_name: &str, total_time: i64, session_count: usize, active_count: usize) -> String {
        format!(
            "Project: {} | Total Time: {} | Sessions: {} total, {} active",
            project_name,
            Formatter::format_duration(total_time),
            session_count,
            active_count
        )
    }

    pub fn format_session_line(start_time: &DateTime<Local>, duration: i64, context: &str, is_active: bool) -> String {
        let status_char = if is_active { "●" } else { "✓" };
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