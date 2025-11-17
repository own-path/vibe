use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tag {
    pub id: Option<i64>,
    pub name: String,
    pub color: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Tag {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name: name.trim().to_lowercase(),
            color: None,
            description: None,
            created_at: Utc::now(),
        }
    }

    pub fn with_color(mut self, color: String) -> Self {
        self.color = Some(color);
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.name.is_empty() {
            return Err(anyhow::anyhow!("Tag name cannot be empty"));
        }

        if self.name != self.name.trim().to_lowercase() {
            return Err(anyhow::anyhow!("Tag name must be lowercase and trimmed"));
        }

        if self.name.contains(char::is_whitespace) {
            return Err(anyhow::anyhow!("Tag name cannot contain whitespace"));
        }

        Ok(())
    }
}