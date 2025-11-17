use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use crate::models::{Project, Session, Tag};
use std::path::PathBuf;

pub struct ProjectQueries;

impl ProjectQueries {
    pub fn create(conn: &Connection, project: &Project) -> Result<i64> {
        let mut stmt = conn.prepare(
            "INSERT INTO projects (name, path, git_hash, description, is_archived)
             VALUES (?1, ?2, ?3, ?4, ?5)"
        )?;
        
        stmt.execute(params![
            project.name,
            project.path.to_string_lossy().to_string(),
            project.git_hash,
            project.description,
            project.is_archived
        ])?;
        
        Ok(conn.last_insert_rowid())
    }
    
    pub fn find_by_path(conn: &Connection, path: &PathBuf) -> Result<Option<Project>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, path, git_hash, created_at, updated_at, is_archived, description
             FROM projects WHERE path = ?1"
        )?;
        
        let project = stmt.query_row([path.to_string_lossy().to_string()], |row| {
            Ok(Project {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                path: PathBuf::from(row.get::<_, String>(2)?),
                git_hash: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                is_archived: row.get(6)?,
                description: row.get(7)?,
            })
        }).optional()?;
        
        Ok(project)
    }
    
    pub fn list_all(conn: &Connection, include_archived: bool) -> Result<Vec<Project>> {
        let sql = if include_archived {
            "SELECT id, name, path, git_hash, created_at, updated_at, is_archived, description
             FROM projects ORDER BY name"
        } else {
            "SELECT id, name, path, git_hash, created_at, updated_at, is_archived, description
             FROM projects WHERE is_archived = 0 ORDER BY name"
        };
        
        let mut stmt = conn.prepare(sql)?;
        let projects = stmt.query_map([], |row| {
            Ok(Project {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                path: PathBuf::from(row.get::<_, String>(2)?),
                git_hash: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
                is_archived: row.get(6)?,
                description: row.get(7)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        
        Ok(projects)
    }
}

pub struct SessionQueries;

impl SessionQueries {
    pub fn create(conn: &Connection, session: &Session) -> Result<i64> {
        let mut stmt = conn.prepare(
            "INSERT INTO sessions (project_id, start_time, end_time, context, paused_duration, notes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        )?;
        
        stmt.execute(params![
            session.project_id,
            session.start_time,
            session.end_time,
            session.context.to_string(),
            session.paused_duration.num_seconds(),
            session.notes
        ])?;
        
        Ok(conn.last_insert_rowid())
    }
    
    pub fn find_active_session(conn: &Connection) -> Result<Option<Session>> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, start_time, end_time, context, paused_duration, notes, created_at
             FROM sessions WHERE end_time IS NULL LIMIT 1"
        )?;
        
        let session = stmt.query_row([], |row| {
            Ok(Session {
                id: Some(row.get(0)?),
                project_id: row.get(1)?,
                start_time: row.get(2)?,
                end_time: row.get(3)?,
                context: row.get::<_, String>(4)?.parse().unwrap(),
                paused_duration: chrono::Duration::seconds(row.get::<_, i64>(5)?),
                notes: row.get(6)?,
                created_at: row.get(7)?,
            })
        }).optional()?;
        
        Ok(session)
    }
    
    pub fn end_session(conn: &Connection, session_id: i64) -> Result<()> {
        let mut stmt = conn.prepare(
            "UPDATE sessions SET end_time = CURRENT_TIMESTAMP WHERE id = ?1"
        )?;
        
        stmt.execute([session_id])?;
        Ok(())
    }
    
    pub fn list_recent(conn: &Connection, limit: usize) -> Result<Vec<Session>> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, start_time, end_time, context, paused_duration, notes, created_at
             FROM sessions ORDER BY start_time DESC LIMIT ?1"
        )?;
        
        let sessions = stmt.query_map([limit], |row| {
            Ok(Session {
                id: Some(row.get(0)?),
                project_id: row.get(1)?,
                start_time: row.get(2)?,
                end_time: row.get(3)?,
                context: row.get::<_, String>(4)?.parse().unwrap(),
                paused_duration: chrono::Duration::seconds(row.get::<_, i64>(5)?),
                notes: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        
        Ok(sessions)
    }
}

pub struct TagQueries;

impl TagQueries {
    pub fn create(conn: &Connection, tag: &Tag) -> Result<i64> {
        let mut stmt = conn.prepare(
            "INSERT INTO tags (name, color, description) VALUES (?1, ?2, ?3)"
        )?;
        
        stmt.execute(params![
            tag.name,
            tag.color,
            tag.description
        ])?;
        
        Ok(conn.last_insert_rowid())
    }
    
    pub fn list_all(conn: &Connection) -> Result<Vec<Tag>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, color, description, created_at FROM tags ORDER BY name"
        )?;
        
        let tags = stmt.query_map([], |row| {
            Ok(Tag {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                color: row.get(2)?,
                description: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        
        Ok(tags)
    }
}