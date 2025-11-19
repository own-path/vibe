use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClientReport {
    pub id: Option<i64>,
    pub client_name: String,
    pub project_id: Option<i64>,
    pub report_period_start: NaiveDate,
    pub report_period_end: NaiveDate,
    pub total_hours: f64,
    pub hourly_rate: Option<f64>,
    pub notes: Option<String>,
    pub status: ReportStatus,
    pub created_at: DateTime<Utc>,
    pub sent_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportStatus {
    Draft,
    Sent,
    Paid,
}

impl std::fmt::Display for ReportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportStatus::Draft => write!(f, "draft"),
            ReportStatus::Sent => write!(f, "sent"),
            ReportStatus::Paid => write!(f, "paid"),
        }
    }
}

impl ClientReport {
    pub fn new(
        client_name: String,
        report_period_start: NaiveDate,
        report_period_end: NaiveDate,
        total_hours: f64,
    ) -> Self {
        Self {
            id: None,
            client_name,
            project_id: None,
            report_period_start,
            report_period_end,
            total_hours,
            hourly_rate: None,
            notes: None,
            status: ReportStatus::Draft,
            created_at: Utc::now(),
            sent_at: None,
        }
    }

    pub fn total_amount(&self) -> Option<f64> {
        self.hourly_rate.map(|rate| rate * self.total_hours)
    }

    pub fn mark_sent(&mut self) {
        self.status = ReportStatus::Sent;
        self.sent_at = Some(Utc::now());
    }

    pub fn mark_paid(&mut self) {
        self.status = ReportStatus::Paid;
    }
}

