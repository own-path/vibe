use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Goal {
    pub id: Option<i64>,
    pub project_id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub target_hours: f64,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
    pub current_progress: f64,
    pub status: GoalStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GoalStatus {
    Active,
    Completed,
    Paused,
    Cancelled,
}

impl std::fmt::Display for GoalStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GoalStatus::Active => write!(f, "active"),
            GoalStatus::Completed => write!(f, "completed"),
            GoalStatus::Paused => write!(f, "paused"),
            GoalStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

impl std::str::FromStr for GoalStatus {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "active" => Ok(GoalStatus::Active),
            "completed" => Ok(GoalStatus::Completed),
            "paused" => Ok(GoalStatus::Paused),
            "cancelled" => Ok(GoalStatus::Cancelled),
            _ => Err(anyhow::anyhow!("Invalid goal status: {}", s)),
        }
    }
}

impl Goal {
    pub fn new(name: String, target_hours: f64) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            project_id: None,
            name,
            description: None,
            target_hours,
            start_date: None,
            end_date: None,
            current_progress: 0.0,
            status: GoalStatus::Active,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_project(mut self, project_id: i64) -> Self {
        self.project_id = Some(project_id);
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_dates(mut self, start: Option<NaiveDate>, end: Option<NaiveDate>) -> Self {
        self.start_date = start;
        self.end_date = end;
        self
    }

    pub fn progress_percentage(&self) -> f64 {
        if self.target_hours == 0.0 {
            return 0.0;
        }
        (self.current_progress / self.target_hours * 100.0).min(100.0)
    }

    pub fn is_completed(&self) -> bool {
        self.status == GoalStatus::Completed || self.current_progress >= self.target_hours
    }

    pub fn remaining_hours(&self) -> f64 {
        (self.target_hours - self.current_progress).max(0.0)
    }

    pub fn update_progress(&mut self, hours: f64) {
        self.current_progress += hours;
        self.updated_at = Utc::now();
        
        if self.current_progress >= self.target_hours && self.status == GoalStatus::Active {
            self.status = GoalStatus::Completed;
        }
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.name.is_empty() {
            return Err(anyhow::anyhow!("Goal name cannot be empty"));
        }
        
        if self.target_hours <= 0.0 {
            return Err(anyhow::anyhow!("Target hours must be greater than 0"));
        }
        
        if let (Some(start), Some(end)) = (self.start_date, self.end_date) {
            if start > end {
                return Err(anyhow::anyhow!("Start date must be before end date"));
            }
        }
        
        if self.current_progress < 0.0 {
            return Err(anyhow::anyhow!("Current progress cannot be negative"));
        }
        
        Ok(())
    }
}

