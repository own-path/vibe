use chrono::{DateTime, Local, Duration};
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph, Table, Row, Cell},
};
use crate::models::{Project, Session};

pub struct Formatter;

impl Formatter {
    pub fn format_duration(seconds: i64) -> String {
        let duration = Duration::seconds(seconds);
        let hours = duration.num_hours();
        let minutes = duration.num_minutes() % 60;
        let seconds = duration.num_seconds() % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }

    pub fn format_timestamp(timestamp: &DateTime<Local>) -> String {
        timestamp.format("%Y-%m-%d %H:%M:%S").to_string()
    }

    pub fn format_time_only(timestamp: &DateTime<Local>) -> String {
        timestamp.format("%H:%M:%S").to_string()
    }

    pub fn format_date_only(timestamp: &DateTime<Local>) -> String {
        timestamp.format("%Y-%m-%d").to_string()
    }

    pub fn create_header_block(title: &str) -> Block {
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan))
    }

    pub fn create_info_block() -> Block<'static> {
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
    }

    pub fn create_success_style() -> Style {
        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
    }

    pub fn create_warning_style() -> Style {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    }

    pub fn create_error_style() -> Style {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    }

    pub fn create_highlight_style() -> Style {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    }

    pub fn format_session_status(session: &Session) -> Text {
        let status = if session.end_time.is_some() {
            Span::styled("Completed", Self::create_success_style())
        } else {
            Span::styled("Active", Self::create_success_style())
        };

        // Context-specific colors
        let context_color = match session.context.to_string().as_str() {
            "terminal" => Color::Cyan,
            "ide" => Color::Magenta,
            "linked" => Color::Yellow,
            "manual" => Color::Blue,
            _ => Color::White,
        };

        let start_time = Self::format_timestamp(&session.start_time.with_timezone(&Local));
        let duration = if let Some(_end_time) = &session.end_time {
            let active_duration = session.active_duration().unwrap_or_default();
            Self::format_duration(active_duration.num_seconds())
        } else {
            let current_active = session.current_active_duration();
            format!("{} (ongoing)", Self::format_duration(current_active.num_seconds()))
        };

        Text::from(vec![
            Line::from(vec![
                Span::raw("Status: "),
                status,
            ]),
            Line::from(vec![
                Span::raw("Started: "),
                Span::styled(start_time, Style::default().fg(Color::Gray)),
            ]),
            Line::from(vec![
                Span::raw("Duration: "),
                Span::styled(duration, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::raw("Context: "),
                Span::styled(session.context.to_string(), Style::default().fg(context_color).add_modifier(Modifier::BOLD)),
            ]),
        ])
    }

    pub fn format_project_info(project: &Project) -> Text {
        Text::from(vec![
            Line::from(vec![
                Span::raw("Name: "),
                Span::styled(project.name.clone(), Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            ]),
            Line::from(vec![
                Span::raw("Path: "),
                Span::styled(project.path.to_string_lossy().to_string(), Style::default().fg(Color::Gray)),
            ]),
            if let Some(description) = &project.description {
                Line::from(vec![
                    Span::raw("Description: "),
                    Span::styled(description.clone(), Style::default().fg(Color::White)),
                ])
            } else {
                Line::from(Span::raw(""))
            },
            Line::from(vec![
                Span::raw("Created: "),
                Span::styled(Self::format_timestamp(&project.created_at.with_timezone(&Local)), Style::default().fg(Color::Gray)),
            ]),
        ])
    }

    pub fn format_sessions_summary(sessions: &[Session]) -> String {
        if sessions.is_empty() {
            return "No sessions found".to_string();
        }

        let mut result = String::new();
        result.push_str("Sessions:\n");
        
        for session in sessions.iter().take(5) {
            let duration = if let Some(_end_time) = &session.end_time {
                let active_duration = session.active_duration().unwrap_or_default();
                Self::format_duration(active_duration.num_seconds())
            } else {
                let current_active = session.current_active_duration();
                format!("{} (active)", Self::format_duration(current_active.num_seconds()))
            };

            let status = if session.end_time.is_some() { "✓" } else { "●" };
            
            result.push_str(&format!(
                "  {} {} | {} | {} | {}\n",
                status,
                session.id.unwrap_or(0),
                Self::format_timestamp(&session.start_time.with_timezone(&Local)),
                duration,
                session.context
            ));
        }

        if sessions.len() > 5 {
            result.push_str(&format!("  ... and {} more sessions\n", sessions.len() - 5));
        }

        result
    }
}