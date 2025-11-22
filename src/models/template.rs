use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectTemplate {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub default_tags: Vec<String>,
    pub default_goals: Vec<TemplateGoal>,
    pub workspace_path: Option<PathBuf>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TemplateGoal {
    pub name: String,
    pub target_hours: f64,
    pub description: Option<String>,
}

impl ProjectTemplate {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            description: None,
            default_tags: Vec::new(),
            default_goals: Vec::new(),
            workspace_path: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.default_tags = tags;
        self
    }

    pub fn with_goals(mut self, goals: Vec<TemplateGoal>) -> Self {
        self.default_goals = goals;
        self
    }

    pub fn with_workspace_path(mut self, path: PathBuf) -> Self {
        self.workspace_path = Some(path);
        self
    }
}
