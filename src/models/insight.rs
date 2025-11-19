use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProductivityInsight {
    pub id: Option<i64>,
    pub project_id: Option<i64>,
    pub insight_type: InsightType,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub data: InsightData,
    pub calculated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InsightType {
    Daily,
    Weekly,
    Monthly,
    ProjectSummary,
}

impl std::fmt::Display for InsightType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InsightType::Daily => write!(f, "daily"),
            InsightType::Weekly => write!(f, "weekly"),
            InsightType::Monthly => write!(f, "monthly"),
            InsightType::ProjectSummary => write!(f, "project_summary"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsightData {
    pub total_hours: f64,
    pub sessions_count: i64,
    pub avg_session_duration: f64,
    pub most_active_day: Option<String>,
    pub most_active_time: Option<String>,
    pub productivity_score: Option<f64>,
    pub project_breakdown: Vec<ProjectBreakdown>,
    pub trends: Vec<TrendPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectBreakdown {
    pub project_id: i64,
    pub project_name: String,
    pub hours: f64,
    pub percentage: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrendPoint {
    pub date: NaiveDate,
    pub hours: f64,
    pub sessions: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamInsight {
    pub id: Option<i64>,
    pub workspace_id: Option<i64>,
    pub team_member: String,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub total_hours: f64,
    pub project_breakdown: Vec<ProjectBreakdown>,
    pub productivity_score: Option<f64>,
    pub calculated_at: DateTime<Utc>,
}

impl ProductivityInsight {
    pub fn new(
        insight_type: InsightType,
        period_start: NaiveDate,
        period_end: NaiveDate,
        data: InsightData,
    ) -> Self {
        Self {
            id: None,
            project_id: None,
            insight_type,
            period_start,
            period_end,
            data,
            calculated_at: Utc::now(),
        }
    }
}

