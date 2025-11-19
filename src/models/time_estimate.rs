use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeEstimate {
    pub id: Option<i64>,
    pub project_id: i64,
    pub task_name: String,
    pub estimated_hours: f64,
    pub actual_hours: Option<f64>,
    pub status: EstimateStatus,
    pub due_date: Option<NaiveDate>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EstimateStatus {
    Planned,
    InProgress,
    Completed,
    Cancelled,
}

impl std::fmt::Display for EstimateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EstimateStatus::Planned => write!(f, "planned"),
            EstimateStatus::InProgress => write!(f, "in_progress"),
            EstimateStatus::Completed => write!(f, "completed"),
            EstimateStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl TimeEstimate {
    pub fn new(project_id: i64, task_name: String, estimated_hours: f64) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            project_id,
            task_name,
            estimated_hours,
            actual_hours: None,
            status: EstimateStatus::Planned,
            due_date: None,
            completed_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_due_date(mut self, due_date: Option<NaiveDate>) -> Self {
        self.due_date = due_date;
        self
    }

    pub fn record_actual(&mut self, hours: f64) {
        self.actual_hours = Some(hours);
        self.updated_at = Utc::now();
        
        if self.status == EstimateStatus::InProgress {
            self.status = EstimateStatus::Completed;
            self.completed_at = Some(Utc::now());
        }
    }

    pub fn variance(&self) -> Option<f64> {
        self.actual_hours.map(|actual| actual - self.estimated_hours)
    }

    pub fn variance_percentage(&self) -> Option<f64> {
        self.variance().map(|v| (v / self.estimated_hours) * 100.0)
    }

    pub fn is_over_estimate(&self) -> bool {
        self.variance().map(|v| v > 0.0).unwrap_or(false)
    }

    pub fn is_under_estimate(&self) -> bool {
        self.variance().map(|v| v < 0.0).unwrap_or(false)
    }
}

