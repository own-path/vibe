use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tempo_cli::models::Project;

/// Optimized project cache with efficient lookups and reduced memory usage
#[derive(Debug)]
pub struct ProjectCache {
    /// Path-based cache for fast path lookups
    by_path: HashMap<PathBuf, ProjectEntry>,
    /// ID-based cache for fast ID lookups
    by_id: HashMap<i64, PathBuf>,
}

/// Minimal project entry for caching
#[derive(Debug, Clone)]
pub struct ProjectEntry {
    pub id: i64,
    pub name: String,
    pub is_archived: bool,
    pub git_hash: Option<String>,
}

impl From<&Project> for ProjectEntry {
    fn from(project: &Project) -> Self {
        Self {
            id: project.id.unwrap_or(0),
            name: project.name.clone(),
            is_archived: project.is_archived,
            git_hash: project.git_hash.clone(),
        }
    }
}

impl ProjectCache {
    /// Create a new empty project cache
    pub fn new() -> Self {
        Self {
            by_path: HashMap::new(),
            by_id: HashMap::new(),
        }
    }

    /// Insert a project into the cache
    pub fn insert(&mut self, project: Project) {
        if let Some(id) = project.id {
            let path = project.path.clone();
            let entry = ProjectEntry::from(&project);
            
            // Store in both indices
            self.by_path.insert(path.clone(), entry);
            self.by_id.insert(id, path);
        }
    }

    /// Get a project by path
    pub fn get_by_path(&self, path: &Path) -> Option<&ProjectEntry> {
        self.by_path.get(path)
    }

    /// Get a project by ID
    pub fn get_by_id(&self, id: i64) -> Option<&ProjectEntry> {
        self.by_id.get(&id)
            .and_then(|path| self.by_path.get(path))
    }

    /// Check if a project exists by path
    pub fn contains_path(&self, path: &Path) -> bool {
        self.by_path.contains_key(path)
    }

    /// Check if a project exists by ID
    pub fn contains_id(&self, id: i64) -> bool {
        self.by_id.contains_key(&id)
    }

    /// Remove a project by path
    pub fn remove_by_path(&mut self, path: &Path) -> Option<ProjectEntry> {
        if let Some(entry) = self.by_path.remove(path) {
            self.by_id.remove(&entry.id);
            Some(entry)
        } else {
            None
        }
    }

    /// Remove a project by ID
    pub fn remove_by_id(&mut self, id: i64) -> Option<ProjectEntry> {
        if let Some(path) = self.by_id.remove(&id) {
            self.by_path.remove(&path)
        } else {
            None
        }
    }

    /// Clear all cached projects
    pub fn clear(&mut self) {
        self.by_path.clear();
        self.by_id.clear();
    }

    /// Get the number of cached projects
    pub fn len(&self) -> usize {
        self.by_path.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.by_path.is_empty()
    }

    /// Iterate over all project entries
    pub fn iter(&self) -> impl Iterator<Item = (&PathBuf, &ProjectEntry)> {
        self.by_path.iter()
    }

    /// Get all project paths
    pub fn paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.by_path.keys()
    }

    /// Update a project entry (useful for status changes)
    pub fn update_entry<F>(&mut self, path: &Path, updater: F) -> bool 
    where 
        F: FnOnce(&mut ProjectEntry),
    {
        if let Some(entry) = self.by_path.get_mut(path) {
            updater(entry);
            true
        } else {
            false
        }
    }

    /// Bulk insert projects
    pub fn insert_all(&mut self, projects: Vec<Project>) {
        for project in projects {
            self.insert(project);
        }
    }
}

impl Default for ProjectCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn create_test_project(id: i64, name: &str, path: &str) -> Project {
        Project {
            id: Some(id),
            name: name.to_string(),
            path: PathBuf::from(path),
            description: None,
            git_hash: None,
            is_archived: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_cache_insert_and_lookup() {
        let mut cache = ProjectCache::new();
        let project = create_test_project(1, "Test Project", "/test/path");
        
        cache.insert(project);
        
        assert!(cache.contains_id(1));
        assert!(cache.contains_path(&PathBuf::from("/test/path")));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_lookup_by_path() {
        let mut cache = ProjectCache::new();
        let project = create_test_project(1, "Test Project", "/test/path");
        
        cache.insert(project);
        
        let entry = cache.get_by_path(&PathBuf::from("/test/path")).unwrap();
        assert_eq!(entry.id, 1);
        assert_eq!(entry.name, "Test Project");
    }

    #[test]
    fn test_cache_lookup_by_id() {
        let mut cache = ProjectCache::new();
        let project = create_test_project(1, "Test Project", "/test/path");
        
        cache.insert(project);
        
        let entry = cache.get_by_id(1).unwrap();
        assert_eq!(entry.id, 1);
        assert_eq!(entry.name, "Test Project");
    }

    #[test]
    fn test_cache_remove() {
        let mut cache = ProjectCache::new();
        let project = create_test_project(1, "Test Project", "/test/path");
        
        cache.insert(project);
        assert_eq!(cache.len(), 1);
        
        let removed = cache.remove_by_path(&PathBuf::from("/test/path"));
        assert!(removed.is_some());
        assert_eq!(cache.len(), 0);
        assert!(!cache.contains_id(1));
    }

    #[test]
    fn test_cache_update() {
        let mut cache = ProjectCache::new();
        let project = create_test_project(1, "Test Project", "/test/path");
        
        cache.insert(project);
        
        let updated = cache.update_entry(&PathBuf::from("/test/path"), |entry| {
            entry.name = "Updated Project".to_string();
        });
        
        assert!(updated);
        
        let entry = cache.get_by_path(&PathBuf::from("/test/path")).unwrap();
        assert_eq!(entry.name, "Updated Project");
    }
}