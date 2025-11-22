use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CalendarEvent {
    pub id: Option<i64>,
    pub external_id: Option<String>,
    pub title: String,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub project_id: Option<i64>,
    pub session_id: Option<i64>,
    pub calendar_type: CalendarType,
    pub synced_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CalendarType {
    Local,
    Google,
    Outlook,
    ICal,
}

impl std::fmt::Display for CalendarType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CalendarType::Local => write!(f, "local"),
            CalendarType::Google => write!(f, "google"),
            CalendarType::Outlook => write!(f, "outlook"),
            CalendarType::ICal => write!(f, "ical"),
        }
    }
}

impl CalendarEvent {
    pub fn new(title: String, start_time: DateTime<Utc>, end_time: DateTime<Utc>) -> Self {
        Self {
            id: None,
            external_id: None,
            title,
            start_time,
            end_time,
            project_id: None,
            session_id: None,
            calendar_type: CalendarType::Local,
            synced_at: None,
            created_at: Utc::now(),
        }
    }

    pub fn duration_hours(&self) -> f64 {
        (self.end_time - self.start_time).num_seconds() as f64 / 3600.0
    }
}
