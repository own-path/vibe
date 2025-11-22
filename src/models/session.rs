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
        self.paused_duration += duration;
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
        self.total_duration()
            .map(|total| total - self.paused_duration)
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
            return Err(anyhow::anyhow!(
                "Paused duration cannot exceed total duration"
            ));
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionEdit {
    pub id: Option<i64>,
    pub session_id: i64,
    pub original_start_time: DateTime<Utc>,
    pub original_end_time: Option<DateTime<Utc>>,
    pub new_start_time: DateTime<Utc>,
    pub new_end_time: Option<DateTime<Utc>>,
    pub edit_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl SessionEdit {
    pub fn new(
        session_id: i64,
        original_start_time: DateTime<Utc>,
        original_end_time: Option<DateTime<Utc>>,
        new_start_time: DateTime<Utc>,
        new_end_time: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: None,
            session_id,
            original_start_time,
            original_end_time,
            new_start_time,
            new_end_time,
            edit_reason: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_reason(mut self, reason: Option<String>) -> Self {
        self.edit_reason = reason;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_new() {
        let session = Session::new(1, SessionContext::Terminal);
        assert_eq!(session.project_id, 1);
        assert_eq!(session.context, SessionContext::Terminal);
        assert!(session.end_time.is_none());
        assert_eq!(session.paused_duration, Duration::zero());
    }

    #[test]
    fn test_session_end() {
        let mut session = Session::new(1, SessionContext::IDE);
        assert!(session.is_active());

        let result = session.end_session();
        assert!(result.is_ok());
        assert!(!session.is_active());
        assert!(session.end_time.is_some());

        // Cannot end twice
        let result = session.end_session();
        assert!(result.is_err());
    }

    #[test]
    fn test_session_duration() {
        let mut session = Session::new(1, SessionContext::Manual);
        let start = Utc::now() - Duration::hours(1);
        session.start_time = start;

        // Active duration (approx 1 hour)
        let duration = session.current_duration();
        assert!(duration >= Duration::hours(1));

        // Add pause
        session.add_pause_duration(Duration::minutes(30));
        let active = session.current_active_duration();
        // Should be approx 30 mins (1h total - 30m pause)
        assert!(active >= Duration::minutes(29) && active <= Duration::minutes(31));
    }

    #[test]
    fn test_session_validation() {
        let mut session = Session::new(1, SessionContext::Terminal);

        // Valid case
        assert!(session.validate().is_ok());

        // Invalid: End before start
        session.end_time = Some(session.start_time - Duration::seconds(1));
        assert!(session.validate().is_err());

        // Invalid: Pause > Total
        session.end_time = Some(session.start_time + Duration::minutes(10));
        session.paused_duration = Duration::minutes(20);
        assert!(session.validate().is_err());
    }
}
