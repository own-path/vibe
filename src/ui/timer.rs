use anyhow::Result;
use chrono::{DateTime, Duration, Utc};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    backend::Backend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Gauge, Paragraph, Wrap},
    Frame, Terminal,
};
use std::time::Duration as StdDuration;

use crate::{
    ui::{
        formatter::Formatter,
        widgets::{ColorScheme, Throbber},
    },
    utils::ipc::IpcClient,
};

pub struct InteractiveTimer {
    client: IpcClient,
    start_time: Option<DateTime<Utc>>,
    paused_at: Option<DateTime<Utc>>,
    total_paused: Duration,
    target_duration: i64, // in seconds
    show_milestones: bool,
    throbber: Throbber,
}

impl InteractiveTimer {
    pub async fn new() -> Result<Self> {
        let socket_path = crate::utils::ipc::get_socket_path()?;
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
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Title (compact)
                Constraint::Length(5), // Timer display (reduced from 8)
                Constraint::Length(4), // Progress bar (reduced from 6)
                Constraint::Length(5), // Milestones (reduced from 6)
                Constraint::Min(0),    // Controls (flexible)
            ])
            .split(f.size());

        // Title
        // Title
        let title = Paragraph::new("Interactive Timer")
            .style(
                Style::default()
                    .fg(ColorScheme::CLEAN_ACCENT)
                    .add_modifier(Modifier::BOLD),
            )
            .alignment(Alignment::Center)
            .block(ColorScheme::clean_block());
        f.render_widget(title, chunks[0]);

        // Timer display
        self.render_timer_display(f, chunks[1]);

        // Progress bar
        self.render_progress_bar(f, chunks[2]);

        // Milestones
        if self.show_milestones {
            self.render_milestones(f, chunks[3]);
        }

        // Controls
        self.render_controls(f, chunks[4]);
    }

    fn render_timer_display(&self, f: &mut Frame, area: Rect) {
        let elapsed = self.get_elapsed_time();
        let is_running = self.start_time.is_some() && self.paused_at.is_none();

        let time_display = Formatter::format_duration(elapsed);
        let status = if is_running {
            "RUNNING"
        } else if self.start_time.is_some() {
            "PAUSED"
        } else {
            "STOPPED"
        };
        let status_color = if is_running {
            Color::Green
        } else if self.start_time.is_some() {
            Color::Yellow
        } else {
            Color::Red
        };

        let timer_text = vec![
            Line::from(Span::styled(
                time_display,
                Style::default()
                    .fg(ColorScheme::CLEAN_BLUE)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::raw("")),
            Line::from(vec![
                Span::raw("Status: "),
                Span::styled(
                    status,
                    Style::default()
                        .fg(status_color)
                        .add_modifier(Modifier::BOLD),
                ),
                if is_running {
                    Span::styled(
                        format!("  {}", self.throbber.current()),
                        Style::default().fg(ColorScheme::CLEAN_ACCENT),
                    )
                } else {
                    Span::raw("")
                },
            ]),
            Line::from(vec![
                Span::raw("Target: "),
                Span::styled(
                    Formatter::format_duration(self.target_duration),
                    Style::default().fg(ColorScheme::WHITE_TEXT),
                ),
            ]),
        ];

        let timer_block = ColorScheme::clean_block()
            .title("Timer")
            .style(Style::default().fg(ColorScheme::WHITE_TEXT));

        let paragraph = Paragraph::new(timer_text)
            .block(timer_block)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }

    fn render_progress_bar(&self, f: &mut Frame, area: Rect) {
        let elapsed = self.get_elapsed_time();
        let progress = if self.target_duration > 0 {
            ((elapsed as f64 / self.target_duration as f64) * 100.0).min(100.0)
        } else {
            0.0
        };

        let progress_color = if progress >= 100.0 {
            ColorScheme::CLEAN_GREEN
        } else if progress >= 75.0 {
            Color::Yellow
        } else {
            ColorScheme::CLEAN_ACCENT
        };

        let progress_bar = Gauge::default()
            .block(
                ColorScheme::clean_block()
                    .title("Progress to Target")
                    .style(Style::default().fg(ColorScheme::WHITE_TEXT)),
            )
            .gauge_style(Style::default().fg(progress_color))
            .percent(progress as u16)
            .label(format!(
                "{:.1}% ({}/{})",
                progress,
                Formatter::format_duration(elapsed),
                Formatter::format_duration(self.target_duration)
            ));

        f.render_widget(progress_bar, area);
    }

    fn render_milestones(&self, f: &mut Frame, area: Rect) {
        let elapsed = self.get_elapsed_time();
        let milestones = vec![
            (5 * 60, "5 min warm-up"),
            (15 * 60, "15 min focus"),
            (25 * 60, "Pomodoro complete"),
            (45 * 60, "45 min deep work"),
            (60 * 60, "1 hour marathon"),
        ];

        let mut milestone_lines = vec![];
        for (duration, name) in milestones {
            let achieved = elapsed >= duration;
            let icon = if achieved { "[x]" } else { "[ ]" };
            let style = if achieved {
                Style::default().fg(ColorScheme::CLEAN_GREEN)
            } else {
                Style::default().fg(ColorScheme::GRAY_TEXT)
            };

            milestone_lines.push(Line::from(vec![Span::styled(
                format!("{} {}", icon, name),
                style,
            )]));
        }

        let milestones_block = ColorScheme::clean_block()
            .title("Milestones")
            .style(Style::default().fg(ColorScheme::WHITE_TEXT));

        let paragraph = Paragraph::new(milestone_lines)
            .block(milestones_block)
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
    }

    fn render_controls(&self, f: &mut Frame, area: Rect) {
        let controls_text = vec![
            Line::from(Span::styled(
                "Controls:",
                Style::default()
                    .fg(ColorScheme::CLEAN_ACCENT)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::raw("Space - Start/Pause timer")),
            Line::from(Span::raw("R - Reset timer")),
            Line::from(Span::raw("S - Set target duration")),
            Line::from(Span::raw("M - Toggle milestones")),
            Line::from(Span::raw("Q/Esc - Quit")),
        ];

        let controls_block = ColorScheme::clean_block()
            .title("Controls")
            .style(Style::default().fg(ColorScheme::WHITE_TEXT));

        let paragraph = Paragraph::new(controls_text)
            .block(controls_block)
            .wrap(Wrap { trim: true });
        f.render_widget(paragraph, area);
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
