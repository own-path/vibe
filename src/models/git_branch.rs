use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GitBranch {
    pub id: Option<i64>,
    pub project_id: i64,
    pub branch_name: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub total_time_seconds: i64,
}

impl GitBranch {
    pub fn new(project_id: i64, branch_name: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            project_id,
            branch_name,
            first_seen: now,
            last_seen: now,
            total_time_seconds: 0,
        }
    }

    pub fn update_time(&mut self, seconds: i64) {
        self.total_time_seconds += seconds;
        self.last_seen = Utc::now();
    }

    pub fn total_hours(&self) -> f64 {
        self.total_time_seconds as f64 / 3600.0
    }
}

