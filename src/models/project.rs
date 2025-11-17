use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub id: Option<i64>,
    pub name: String,
    pub path: PathBuf,
    pub git_hash: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub is_archived: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProjectStatus {
    Active,
    Archived,
    Tracking,
    Idle,
}

impl Project {
    pub fn new(name: String, path: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            name,
            path,
            git_hash: None,
            created_at: now,
            updated_at: now,
            is_archived: false,
            description: None,
        }
    }

    pub fn with_git_hash(mut self, git_hash: Option<String>) -> Self {
        self.git_hash = git_hash;
        self
    }

    pub fn with_description(mut self, description: Option<String>) -> Self {
        self.description = description;
        self
    }

    pub fn archive(&mut self) {
        self.is_archived = true;
        self.updated_at = Utc::now();
    }

    pub fn unarchive(&mut self) {
        self.is_archived = false;
        self.updated_at = Utc::now();
    }

    pub fn update_path(&mut self, new_path: PathBuf) {
        self.path = new_path;
        self.updated_at = Utc::now();
    }

    pub fn is_git_project(&self) -> bool {
        self.path.join(".git").exists()
    }

    pub fn has_timetrack_marker(&self) -> bool {
        self.path.join(".timetrack").exists()
    }

    pub fn get_canonical_path(&self) -> anyhow::Result<PathBuf> {
        Ok(self.path.canonicalize()?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedProject {
    pub id: Option<i64>,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
    pub member_projects: Vec<Project>,
}

impl LinkedProject {
    pub fn new(name: String) -> Self {
        Self {
            id: None,
            name,
            description: None,
            created_at: Utc::now(),
            is_active: true,
            member_projects: Vec::new(),
        }
    }

    pub fn add_project(&mut self, project: Project) {
        self.member_projects.push(project);
    }

    pub fn remove_project(&mut self, project_id: i64) {
        self.member_projects.retain(|p| p.id != Some(project_id));
    }
}