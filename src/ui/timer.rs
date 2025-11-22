use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame, Terminal,
};
use std::time::Duration as StdDuration;

use crate::ui::{
    formatter::Formatter,
    widgets::{ColorScheme, Throbber},
};

pub struct InteractiveTimer {
    start_time: Option<DateTime<Utc>>,
    paused_at: Option<DateTime<Utc>>,
    total_paused: Duration,
    target_duration: i64, // in seconds
    show_milestones: bool,
    throbber: Throbber,
}

impl InteractiveTimer {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            start_time: None,
            paused_at: None,
            total_paused: Duration::zero(),
            target_duration: 25 * 60, // Default 25 minutes (Pomodoro)
            show_milestones: true,
            throbber: Throbber::new(),
        })
    }

    pub async fn run<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<()> {
        loop {
            // Update timer state
            self.update_timer_state().await?;

            terminal.draw(|f| {
                self.render_timer(f);
            })?;

            // Handle input
            if event::poll(StdDuration::from_millis(100))? {
                match event::read()? {
                    Event::Key(key) if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => break,
                        KeyCode::Char(' ') => self.toggle_timer().await?,
                        KeyCode::Char('r') => self.reset_timer().await?,
                        KeyCode::Char('s') => self.set_target().await?,
                        KeyCode::Char('m') => self.show_milestones = !self.show_milestones,
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn render_timer(&self, f: &mut Frame) {
        // Focused Mode Layout:
        // Centered box with:
        // 1. Project Context (Top)
        // 2. Large Timer (Center)
        // 3. Metadata & Progress (Bottom)

        let area = f.size();
        let vertical_center = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(area);

        let horizontal_center = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(20),
                Constraint::Percentage(60),
                Constraint::Percentage(20),
            ])
            .split(vertical_center[1]);

        let main_area = horizontal_center[1];

        // Main Block with subtle border
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(ColorScheme::GRAY_TEXT))
            .style(Style::default().bg(ColorScheme::CLEAN_BG));

        f.render_widget(block.clone(), main_area);

        let inner_area = block.inner(main_area);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(2), // Project Context
                Constraint::Length(1), // Spacer
                Constraint::Length(3), // Large Timer
                Constraint::Length(1), // Spacer
                Constraint::Length(1), // Progress Indicator
                Constraint::Length(1), // Spacer
                Constraint::Min(1),    // Metadata
            ])
            .margin(2)
            .split(inner_area);

        // 1. Project Context
        self.render_project_context(f, chunks[0]);

        // 2. Large Timer
        self.render_large_timer(f, chunks[2]);

        // 3. Progress Indicator
        self.render_progress_indicator(f, chunks[4]);

        // 4. Metadata
        self.render_metadata(f, chunks[6]);
    }

    fn render_project_context(&self, f: &mut Frame, area: Rect) {
        // Placeholder for project info - ideally fetched from state
        let project_name = "Current Project";
        let description = "Deep Work Session";

        let text = vec![
            Line::from(Span::styled(
                project_name,
                Style::default()
                    .fg(ColorScheme::CLEAN_ACCENT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                description,
                Style::default().fg(ColorScheme::GRAY_TEXT),
            )),
        ];

        f.render_widget(Paragraph::new(text).alignment(Alignment::Center), area);
    }

    fn render_large_timer(&self, f: &mut Frame, area: Rect) {
        let elapsed = self.get_elapsed_time();
        let time_str = Formatter::format_duration_clock(elapsed);

        // In a real terminal, "large text" is hard without ASCII art libraries.
        // We'll use bold and bright colors for now.
        let text = Paragraph::new(time_str)
            .style(
                Style::default()
                    .fg(ColorScheme::WHITE_TEXT)
                    .add_modifier(Modifier::BOLD),
            ) // .add_modifier(Modifier::ITALIC) ?
            .alignment(Alignment::Center);

        f.render_widget(text, area);
    }

    fn render_progress_indicator(&self, f: &mut Frame, area: Rect) {
        let elapsed = self.get_elapsed_time();
        let progress = if self.target_duration > 0 {
            ((elapsed as f64 / self.target_duration as f64) * 100.0).min(100.0)
        } else {
            0.0
        };

        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(ColorScheme::CLEAN_BLUE))
            .percent(progress as u16)
            .label(""); // Minimalist, no label inside

        f.render_widget(gauge, area);
    }

    fn render_metadata(&self, f: &mut Frame, area: Rect) {
        let start_time_str = if let Some(start) = self.start_time {
            start.format("%H:%M").to_string()
        } else {
            "--:--".to_string()
        };

        let meta_text = vec![
            Line::from(vec![
                Span::raw("Started: "),
                Span::styled(start_time_str, Style::default().fg(ColorScheme::WHITE_TEXT)),
                Span::raw(" • "),
                Span::raw("Target: "),
                Span::styled(
                    Formatter::format_duration(self.target_duration),
                    Style::default().fg(ColorScheme::WHITE_TEXT),
                ),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                "[Space] Pause  [R] Reset  [Q] Quit",
                Style::default().fg(ColorScheme::GRAY_TEXT),
            )),
        ];

        f.render_widget(Paragraph::new(meta_text).alignment(Alignment::Center), area);
    }

    async fn update_timer_state(&mut self) -> Result<()> {
        // This would sync with the actual session state from the daemon
        // For now, we'll keep local state
        if self.start_time.is_some() && self.paused_at.is_none() {
            self.throbber.next();
        }
        Ok(())
    }

    async fn toggle_timer(&mut self) -> Result<()> {
        if self.start_time.is_none() {
            // Start timer
            self.start_time = Some(Utc::now());
            self.paused_at = None;
        } else if self.paused_at.is_some() {
            // Resume timer
            if let Some(paused_at) = self.paused_at {
                self.total_paused += Utc::now() - paused_at;
            }
            self.paused_at = None;
        } else {
            // Pause timer
            self.paused_at = Some(Utc::now());
        }
        Ok(())
    }

    async fn reset_timer(&mut self) -> Result<()> {
        self.start_time = None;
        self.paused_at = None;
        self.total_paused = chrono::Duration::zero();
        Ok(())
    }

    async fn set_target(&mut self) -> Result<()> {
        // In a full implementation, this would show an input dialog
        // For now, cycle through common durations
        self.target_duration = match self.target_duration {
            1500 => 1800, // 25min -> 30min
            1800 => 2700, // 30min -> 45min
            2700 => 3600, // 45min -> 1hour
            3600 => 5400, // 1hour -> 1.5hour
            5400 => 7200, // 1.5hour -> 2hour
            _ => 1500,    // Default back to 25min (Pomodoro)
        };
        Ok(())
    }

    fn get_elapsed_time(&self) -> i64 {
        if let Some(start) = self.start_time {
            let end_time = if let Some(paused) = self.paused_at {
                paused
            } else {
                Utc::now()
            };

            (end_time - start - self.total_paused).num_seconds().max(0)
        } else {
            0
        }
    }
}
