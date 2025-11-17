use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionContext {
    Terminal,
    IDE,
    Linked,
    Manual,
}

impl std::fmt::Display for SessionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionContext::Terminal => write!(f, "terminal"),
            SessionContext::IDE => write!(f, "ide"),
            SessionContext::Linked => write!(f, "linked"),
            SessionContext::Manual => write!(f, "manual"),
        }
    }
}

impl std::str::FromStr for SessionContext {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "terminal" => Ok(SessionContext::Terminal),
            "ide" => Ok(SessionContext::IDE),
            "linked" => Ok(SessionContext::Linked),
            "manual" => Ok(SessionContext::Manual),
            _ => Err(anyhow::anyhow!("Invalid session context: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    pub id: Option<i64>,
    pub project_id: i64,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub context: SessionContext,
    pub paused_duration: Duration,
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Session {
    pub fn new(project_id: i64, context: SessionContext) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            project_id,
            start_time: now,
            end_time: None,
            context,
            paused_duration: Duration::zero(),
            notes: None,
            created_at: now,
        }
    }

    pub fn with_start_time(mut self, start_time: DateTime<Utc>) -> Self {
        self.start_time = start_time;
        self
    }

    pub fn with_notes(mut self, notes: Option<String>) -> Self {
        self.notes = notes;
        self
    }

    pub fn end_session(&mut self) -> anyhow::Result<()> {
        if self.end_time.is_some() {
            return Err(anyhow::anyhow!("Session is already ended"));
        }
        
        self.end_time = Some(Utc::now());
        Ok(())
    }

    pub fn add_pause_duration(&mut self, duration: Duration) {
        self.paused_duration = self.paused_duration + duration;
    }

    pub fn is_active(&self) -> bool {
        self.end_time.is_none()
    }

    pub fn status(&self) -> SessionStatus {
        if self.end_time.is_some() {
            SessionStatus::Completed
        } else {
            SessionStatus::Active
        }
    }

    pub fn total_duration(&self) -> Option<Duration> {
        self.end_time.map(|end| end - self.start_time)
    }

    pub fn active_duration(&self) -> Option<Duration> {
        self.total_duration().map(|total| total - self.paused_duration)
    }

    pub fn current_duration(&self) -> Duration {
        let end_time = self.end_time.unwrap_or_else(Utc::now);
        end_time - self.start_time
    }

    pub fn current_active_duration(&self) -> Duration {
        self.current_duration() - self.paused_duration
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(end_time) = self.end_time {
            if end_time <= self.start_time {
                return Err(anyhow::anyhow!("End time must be after start time"));
            }
        }

        if self.paused_duration < Duration::zero() {
            return Err(anyhow::anyhow!("Paused duration cannot be negative"));
        }

        let total = self.current_duration();
        if self.paused_duration > total {
            return Err(anyhow::anyhow!("Paused duration cannot exceed total duration"));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEdit {
    pub id: Option<i64>,
    pub session_id: i64,
    pub field_name: String,
    pub old_value: Option<String>,
    pub new_value: String,
    pub edit_reason: Option<String>,
    pub edited_at: DateTime<Utc>,
}

impl SessionEdit {
    pub fn new(
        session_id: i64,
        field_name: String,
        old_value: Option<String>,
        new_value: String,
    ) -> Self {
        Self {
            id: None,
            session_id,
            field_name,
            old_value,
            new_value,
            edit_reason: None,
            edited_at: Utc::now(),
        }
    }

    pub fn with_reason(mut self, reason: String) -> Self {
        self.edit_reason = Some(reason);
        self
    }
}