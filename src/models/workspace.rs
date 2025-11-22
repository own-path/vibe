use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub path: Option<PathBuf>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Workspace {
    pub fn new(name: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            name,
            description: None,
            path: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.name.is_empty() {
            return Err(anyhow::anyhow!("Workspace name cannot be empty"));
        }
        Ok(())
    }
}
