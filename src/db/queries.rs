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
    
    pub fn find_by_id(conn: &Connection, project_id: i64) -> Result<Option<Project>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, path, git_hash, created_at, updated_at, is_archived, description
             FROM projects WHERE id = ?1"
        )?;
        
        let project = stmt.query_row([project_id], |row| {
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
    
    pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<Project>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, path, git_hash, created_at, updated_at, is_archived, description
             FROM projects WHERE name = ?1"
        )?;
        
        let project = stmt.query_row([name], |row| {
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
    
    pub fn archive_project(conn: &Connection, project_id: i64) -> Result<bool> {
        let mut stmt = conn.prepare(
            "UPDATE projects SET is_archived = 1, updated_at = CURRENT_TIMESTAMP WHERE id = ?1"
        )?;
        let changes = stmt.execute([project_id])?;
        Ok(changes > 0)
    }
    
    pub fn unarchive_project(conn: &Connection, project_id: i64) -> Result<bool> {
        let mut stmt = conn.prepare(
            "UPDATE projects SET is_archived = 0, updated_at = CURRENT_TIMESTAMP WHERE id = ?1"
        )?;
        let changes = stmt.execute([project_id])?;
        Ok(changes > 0)
    }
    
    pub fn update_project_path(conn: &Connection, project_id: i64, new_path: &PathBuf) -> Result<bool> {
        let mut stmt = conn.prepare(
            "UPDATE projects SET path = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2"
        )?;
        let changes = stmt.execute([new_path.to_string_lossy().to_string(), project_id.to_string()])?;
        Ok(changes > 0)
    }
    
    pub fn update_project_description(conn: &Connection, project_id: i64, description: Option<String>) -> Result<bool> {
        let mut stmt = conn.prepare(
            "UPDATE projects SET description = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2"
        )?;
        let changes = stmt.execute([description, Some(project_id.to_string())])?;
        Ok(changes > 0)
    }
    
    pub fn delete_project(conn: &Connection, project_id: i64) -> Result<bool> {
        let mut stmt = conn.prepare("DELETE FROM projects WHERE id = ?1")?;
        let changes = stmt.execute([project_id])?;
        Ok(changes > 0)
    }
    
    pub fn get_project_stats(conn: &Connection, project_id: i64) -> Result<Option<(i64, i64, i64)>> { // (total_sessions, total_time_seconds, avg_session_seconds)
        let mut stmt = conn.prepare(
            "SELECT 
                COUNT(*) as session_count,
                COALESCE(SUM(CASE 
                    WHEN end_time IS NOT NULL THEN 
                        (julianday(end_time) - julianday(start_time)) * 86400 - paused_duration
                    ELSE 0
                END), 0) as total_time,
                COALESCE(AVG(CASE 
                    WHEN end_time IS NOT NULL THEN 
                        (julianday(end_time) - julianday(start_time)) * 86400 - paused_duration
                    ELSE 0
                END), 0) as avg_time
             FROM sessions WHERE project_id = ?1 AND end_time IS NOT NULL"
        )?;
        
        let stats = stmt.query_row([project_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, f64>(1)? as i64,
                row.get::<_, f64>(2)? as i64,
            ))
        }).optional()?;
        
        Ok(stats)
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
    
    pub fn find_by_id(conn: &Connection, session_id: i64) -> Result<Option<Session>> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, start_time, end_time, context, paused_duration, notes, created_at
             FROM sessions WHERE id = ?1"
        )?;
        
        let session = stmt.query_row([session_id], |row| {
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
    
    pub fn update_session(conn: &Connection, session_id: i64, start_time: Option<chrono::DateTime<chrono::Utc>>, end_time: Option<Option<chrono::DateTime<chrono::Utc>>>, project_id: Option<i64>, notes: Option<Option<String>>) -> Result<()> {
        let mut updates = Vec::new();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        
        if let Some(st) = start_time {
            updates.push("start_time = ?");
            params.push(Box::new(st));
        }
        
        if let Some(et) = end_time {
            updates.push("end_time = ?");
            params.push(Box::new(et));
        }
        
        if let Some(pid) = project_id {
            updates.push("project_id = ?");
            params.push(Box::new(pid));
        }
        
        if let Some(n) = notes {
            updates.push("notes = ?");
            params.push(Box::new(n));
        }
        
        if updates.is_empty() {
            return Ok(());
        }
        
        params.push(Box::new(session_id));
        
        let sql = format!("UPDATE sessions SET {} WHERE id = ?", updates.join(", "));
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        stmt.execute(&param_refs[..])?;
        
        Ok(())
    }
    
    pub fn delete_session(conn: &Connection, session_id: i64) -> Result<()> {
        let mut stmt = conn.prepare("DELETE FROM sessions WHERE id = ?1")?;
        stmt.execute([session_id])?;
        Ok(())
    }
    
    pub fn list_with_filter(conn: &Connection, project_id: Option<i64>, start_date: Option<chrono::NaiveDate>, end_date: Option<chrono::NaiveDate>, limit: Option<usize>) -> Result<Vec<Session>> {
        let mut sql = "SELECT id, project_id, start_time, end_time, context, paused_duration, notes, created_at FROM sessions WHERE 1=1".to_string();
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        
        if let Some(pid) = project_id {
            sql.push_str(" AND project_id = ?");
            params.push(Box::new(pid));
        }
        
        if let Some(sd) = start_date {
            sql.push_str(" AND date(start_time) >= ?");
            params.push(Box::new(sd.format("%Y-%m-%d").to_string()));
        }
        
        if let Some(ed) = end_date {
            sql.push_str(" AND date(start_time) <= ?");
            params.push(Box::new(ed.format("%Y-%m-%d").to_string()));
        }
        
        sql.push_str(" ORDER BY start_time DESC");
        
        if let Some(lim) = limit {
            sql.push_str(" LIMIT ?");
            params.push(Box::new(lim));
        }
        
        let mut stmt = conn.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        
        let sessions = stmt.query_map(&param_refs[..], |row| {
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
    
    pub fn bulk_update_project(conn: &Connection, session_ids: &[i64], new_project_id: i64) -> Result<usize> {
        let placeholders = vec!["?"; session_ids.len()].join(",");
        let sql = format!("UPDATE sessions SET project_id = ? WHERE id IN ({})", placeholders);
        
        let mut stmt = conn.prepare(&sql)?;
        let mut params: Vec<&dyn rusqlite::ToSql> = vec![&new_project_id];
        for id in session_ids {
            params.push(id);
        }
        
        let changes = stmt.execute(&params[..])?;
        Ok(changes)
    }
    
    pub fn bulk_delete(conn: &Connection, session_ids: &[i64]) -> Result<usize> {
        let placeholders = vec!["?"; session_ids.len()].join(",");
        let sql = format!("DELETE FROM sessions WHERE id IN ({})", placeholders);
        
        let mut stmt = conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = session_ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        
        let changes = stmt.execute(&params[..])?;
        Ok(changes)
    }
    
    pub fn merge_sessions(conn: &Connection, session_ids: &[i64], target_project_id: Option<i64>, notes: Option<String>) -> Result<i64> {
        if session_ids.is_empty() {
            return Err(anyhow::anyhow!("No sessions to merge"));
        }
        
        // Get all sessions to merge
        let placeholders = vec!["?"; session_ids.len()].join(",");
        let sql = format!(
            "SELECT id, project_id, start_time, end_time, context, paused_duration, notes, created_at 
             FROM sessions WHERE id IN ({}) ORDER BY start_time", 
            placeholders
        );
        
        let mut stmt = conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::ToSql> = session_ids.iter().map(|id| id as &dyn rusqlite::ToSql).collect();
        
        let sessions: Result<Vec<Session>, _> = stmt.query_map(&params[..], |row| {
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
        })?.collect();
        
        let sessions = sessions?;
        if sessions.is_empty() {
            return Err(anyhow::anyhow!("No valid sessions found to merge"));
        }
        
        // Calculate merged session properties
        let earliest_start = sessions.iter().map(|s| s.start_time).min().unwrap();
        let latest_end = sessions.iter().filter_map(|s| s.end_time).max();
        let total_paused = sessions.iter().map(|s| s.paused_duration).fold(chrono::Duration::zero(), |acc, d| acc + d);
        let merged_project_id = target_project_id.unwrap_or(sessions[0].project_id);
        let merged_context = sessions[0].context; // Use first session's context
        
        // Create merged session
        let merged_session = Session {
            id: None,
            project_id: merged_project_id,
            start_time: earliest_start,
            end_time: latest_end,
            context: merged_context,
            paused_duration: total_paused,
            notes,
            created_at: chrono::Utc::now(),
        };
        
        // Insert merged session
        let merged_id = Self::create(conn, &merged_session)?;
        
        // Create audit records for the merge
        for session in &sessions {
            if let Some(session_id) = session.id {
                SessionEditQueries::create_edit_record(
                    conn,
                    session_id,
                    session.start_time,
                    session.end_time,
                    merged_session.start_time,
                    merged_session.end_time,
                    Some(format!("Merged into session {}", merged_id))
                )?;
            }
        }
        
        // Delete original sessions
        Self::bulk_delete(conn, session_ids)?;
        
        Ok(merged_id)
    }
    
    pub fn split_session(conn: &Connection, session_id: i64, split_times: &[chrono::DateTime<chrono::Utc>], notes_list: Option<Vec<String>>) -> Result<Vec<i64>> {
        // Get the original session
        let original_session = Self::find_by_id(conn, session_id)?
            .ok_or_else(|| anyhow::anyhow!("Session {} not found", session_id))?;
        
        if split_times.is_empty() {
            return Err(anyhow::anyhow!("No split times provided"));
        }
        
        // Validate split times are within session bounds
        for &split_time in split_times {
            if split_time <= original_session.start_time {
                return Err(anyhow::anyhow!("Split time {} is before session start", split_time));
            }
            if let Some(end_time) = original_session.end_time {
                if split_time >= end_time {
                    return Err(anyhow::anyhow!("Split time {} is after session end", split_time));
                }
            }
        }
        
        // Sort split times
        let mut sorted_splits = split_times.to_vec();
        sorted_splits.sort();
        
        let mut new_session_ids = Vec::new();
        let mut current_start = original_session.start_time;
        
        // Create sessions for each split segment
        for (i, &split_time) in sorted_splits.iter().enumerate() {
            let segment_notes = notes_list.as_ref()
                .and_then(|list| list.get(i))
                .cloned()
                .or_else(|| original_session.notes.clone());
            
            let split_session = Session {
                id: None,
                project_id: original_session.project_id,
                start_time: current_start,
                end_time: Some(split_time),
                context: original_session.context,
                paused_duration: chrono::Duration::zero(), // Reset paused duration for splits
                notes: segment_notes,
                created_at: chrono::Utc::now(),
            };
            
            let split_id = Self::create(conn, &split_session)?;
            new_session_ids.push(split_id);
            current_start = split_time;
        }
        
        // Create final segment (from last split to original end)
        let final_notes = notes_list.as_ref()
            .and_then(|list| list.get(sorted_splits.len()))
            .cloned()
            .or_else(|| original_session.notes.clone());
        
        let final_session = Session {
            id: None,
            project_id: original_session.project_id,
            start_time: current_start,
            end_time: original_session.end_time,
            context: original_session.context,
            paused_duration: chrono::Duration::zero(),
            notes: final_notes,
            created_at: chrono::Utc::now(),
        };
        
        let final_id = Self::create(conn, &final_session)?;
        new_session_ids.push(final_id);
        
        // Create audit record for the split
        SessionEditQueries::create_edit_record(
            conn,
            session_id,
            original_session.start_time,
            original_session.end_time,
            original_session.start_time,
            original_session.end_time,
            Some(format!("Split into sessions: {}", new_session_ids.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(", ")))
        )?;
        
        // Delete original session
        Self::delete_session(conn, session_id)?;
        
        Ok(new_session_ids)
    }
}

pub struct SessionEditQueries;

impl SessionEditQueries {
    pub fn create_edit_record(conn: &Connection, session_id: i64, original_start: chrono::DateTime<chrono::Utc>, original_end: Option<chrono::DateTime<chrono::Utc>>, new_start: chrono::DateTime<chrono::Utc>, new_end: Option<chrono::DateTime<chrono::Utc>>, reason: Option<String>) -> Result<i64> {
        let mut stmt = conn.prepare(
            "INSERT INTO session_edits (session_id, original_start_time, original_end_time, new_start_time, new_end_time, edit_reason)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)"
        )?;
        
        stmt.execute(params![
            session_id,
            original_start,
            original_end,
            new_start, 
            new_end,
            reason
        ])?;
        
        Ok(conn.last_insert_rowid())
    }
    
    pub fn list_session_edits(conn: &Connection, session_id: i64) -> Result<Vec<crate::models::SessionEdit>> {
        let mut stmt = conn.prepare(
            "SELECT id, session_id, original_start_time, original_end_time, new_start_time, new_end_time, edit_reason, created_at
             FROM session_edits WHERE session_id = ?1 ORDER BY created_at DESC"
        )?;
        
        let edits = stmt.query_map([session_id], |row| {
            Ok(crate::models::SessionEdit {
                id: Some(row.get(0)?),
                session_id: row.get(1)?,
                original_start_time: row.get(2)?,
                original_end_time: row.get(3)?,
                new_start_time: row.get(4)?,
                new_end_time: row.get(5)?,
                edit_reason: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        
        Ok(edits)
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
    
    pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<Tag>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, color, description, created_at FROM tags WHERE name = ?1"
        )?;
        
        let tag = stmt.query_row([name], |row| {
            Ok(Tag {
                id: Some(row.get(0)?),
                name: row.get(1)?,
                color: row.get(2)?,
                description: row.get(3)?,
                created_at: row.get(4)?,
            })
        }).optional()?;
        
        Ok(tag)
    }
    
    pub fn delete_by_name(conn: &Connection, name: &str) -> Result<bool> {
        let mut stmt = conn.prepare("DELETE FROM tags WHERE name = ?1")?;
        let changes = stmt.execute([name])?;
        Ok(changes > 0)
    }
    
    pub fn update_tag(conn: &Connection, name: &str, color: Option<String>, description: Option<String>) -> Result<bool> {
        let mut updates = Vec::new();
        let mut params: Vec<&dyn rusqlite::ToSql> = Vec::new();
        
        if let Some(c) = &color {
            updates.push("color = ?");
            params.push(c);
        }
        
        if let Some(d) = &description {
            updates.push("description = ?");
            params.push(d);
        }
        
        if updates.is_empty() {
            return Ok(false);
        }
        
        params.push(&name);
        
        let sql = format!("UPDATE tags SET {} WHERE name = ?", updates.join(", "));
        let mut stmt = conn.prepare(&sql)?;
        let changes = stmt.execute(&params[..])?;
        
        Ok(changes > 0)
    }
}