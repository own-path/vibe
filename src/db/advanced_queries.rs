use crate::models::{
    GitBranch, Goal, GoalStatus, InsightData, ProjectTemplate, TimeEstimate, Workspace,
};
use anyhow::Result;
use chrono::NaiveDate;
use rusqlite::{params, Connection, OptionalExtension};

pub struct GoalQueries;

impl GoalQueries {
    pub fn create(conn: &Connection, goal: &Goal) -> Result<i64> {
        let mut stmt = conn.prepare(
            "INSERT INTO goals (project_id, name, description, target_hours, start_date, end_date, current_progress, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"
        )?;

        stmt.execute(params![
            goal.project_id,
            goal.name,
            goal.description,
            goal.target_hours,
            goal.start_date,
            goal.end_date,
            goal.current_progress,
            goal.status.to_string()
        ])?;

        Ok(conn.last_insert_rowid())
    }

    pub fn find_by_id(conn: &Connection, goal_id: i64) -> Result<Option<Goal>> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, description, target_hours, start_date, end_date, current_progress, status, created_at, updated_at
             FROM goals WHERE id = ?1"
        )?;

        let goal = stmt
            .query_row([goal_id], |row| {
                Ok(Goal {
                    id: Some(row.get(0)?),
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    target_hours: row.get(4)?,
                    start_date: row.get(5)?,
                    end_date: row.get(6)?,
                    current_progress: row.get(7)?,
                    status: row
                        .get::<_, String>(8)?
                        .parse()
                        .unwrap_or(GoalStatus::Active),
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })
            .optional()?;

        Ok(goal)
    }

    pub fn list_by_project(conn: &Connection, project_id: Option<i64>) -> Result<Vec<Goal>> {
        let sql = if let Some(_pid) = project_id {
            "SELECT id, project_id, name, description, target_hours, start_date, end_date, current_progress, status, created_at, updated_at
             FROM goals WHERE project_id = ?1 ORDER BY created_at DESC"
        } else {
            "SELECT id, project_id, name, description, target_hours, start_date, end_date, current_progress, status, created_at, updated_at
             FROM goals ORDER BY created_at DESC"
        };

        let mut stmt = conn.prepare(sql)?;
        let goals = if let Some(pid) = project_id {
            stmt.query_map([pid], |row| {
                Ok(Goal {
                    id: Some(row.get(0)?),
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    target_hours: row.get(4)?,
                    start_date: row.get(5)?,
                    end_date: row.get(6)?,
                    current_progress: row.get(7)?,
                    status: row
                        .get::<_, String>(8)?
                        .parse()
                        .unwrap_or(GoalStatus::Active),
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map([], |row| {
                Ok(Goal {
                    id: Some(row.get(0)?),
                    project_id: row.get(1)?,
                    name: row.get(2)?,
                    description: row.get(3)?,
                    target_hours: row.get(4)?,
                    start_date: row.get(5)?,
                    end_date: row.get(6)?,
                    current_progress: row.get(7)?,
                    status: row
                        .get::<_, String>(8)?
                        .parse()
                        .unwrap_or(GoalStatus::Active),
                    created_at: row.get(9)?,
                    updated_at: row.get(10)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        };

        Ok(goals)
    }

    pub fn update_progress(conn: &Connection, goal_id: i64, hours: f64) -> Result<bool> {
        let mut stmt = conn.prepare(
            "UPDATE goals SET current_progress = current_progress + ?1, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?2"
        )?;
        let changes = stmt.execute(params![hours, goal_id])?;
        Ok(changes > 0)
    }
}

pub struct TemplateQueries;

impl TemplateQueries {
    pub fn create(conn: &Connection, template: &ProjectTemplate) -> Result<i64> {
        let tags_json = serde_json::to_string(&template.default_tags)?;
        let goals_json = serde_json::to_string(&template.default_goals)?;

        let mut stmt = conn.prepare(
            "INSERT INTO project_templates (name, description, default_tags, default_goals, workspace_path)
             VALUES (?1, ?2, ?3, ?4, ?5)"
        )?;

        stmt.execute(params![
            template.name,
            template.description,
            tags_json,
            goals_json,
            template
                .workspace_path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
        ])?;

        Ok(conn.last_insert_rowid())
    }

    pub fn list_all(conn: &Connection) -> Result<Vec<ProjectTemplate>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, default_tags, default_goals, workspace_path, created_at
             FROM project_templates ORDER BY name",
        )?;

        let templates = stmt
            .query_map([], |row| {
                let tags_json: String = row.get(3)?;
                let goals_json: String = row.get(4)?;

                Ok(ProjectTemplate {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    default_tags: serde_json::from_str(&tags_json).unwrap_or_default(),
                    default_goals: serde_json::from_str(&goals_json).unwrap_or_default(),
                    workspace_path: row.get::<_, Option<String>>(5)?.map(|s| s.into()),
                    created_at: row.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(templates)
    }
}

pub struct WorkspaceQueries;

impl WorkspaceQueries {
    pub fn create(conn: &Connection, workspace: &Workspace) -> Result<i64> {
        let mut stmt = conn.prepare(
            "INSERT INTO workspaces (name, description, path)
             VALUES (?1, ?2, ?3)",
        )?;

        stmt.execute(params![
            workspace.name,
            workspace.description,
            workspace
                .path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
        ])?;

        Ok(conn.last_insert_rowid())
    }

    pub fn list_all(conn: &Connection) -> Result<Vec<Workspace>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, path, created_at, updated_at
             FROM workspaces ORDER BY name",
        )?;

        let workspaces = stmt
            .query_map([], |row| {
                Ok(Workspace {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    path: row.get::<_, Option<String>>(3)?.map(|s| s.into()),
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(workspaces)
    }

    pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<Workspace>> {
        let mut stmt = conn.prepare(
            "SELECT id, name, description, path, created_at, updated_at
             FROM workspaces WHERE name = ?1",
        )?;

        let workspace = stmt
            .query_row([name], |row| {
                Ok(Workspace {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    path: row.get::<_, Option<String>>(3)?.map(|s| s.into()),
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            })
            .optional()?;

        Ok(workspace)
    }

    pub fn delete(conn: &Connection, workspace_id: i64) -> Result<bool> {
        let mut stmt = conn.prepare("DELETE FROM workspaces WHERE id = ?1")?;
        let changes = stmt.execute([workspace_id])?;
        Ok(changes > 0)
    }

    pub fn add_project(conn: &Connection, workspace_id: i64, project_id: i64) -> Result<bool> {
        let mut stmt = conn.prepare(
            "INSERT OR IGNORE INTO workspace_projects (workspace_id, project_id)
             VALUES (?1, ?2)",
        )?;
        let changes = stmt.execute(params![workspace_id, project_id])?;
        Ok(changes > 0)
    }

    pub fn remove_project(conn: &Connection, workspace_id: i64, project_id: i64) -> Result<bool> {
        let mut stmt = conn.prepare(
            "DELETE FROM workspace_projects 
             WHERE workspace_id = ?1 AND project_id = ?2",
        )?;
        let changes = stmt.execute(params![workspace_id, project_id])?;
        Ok(changes > 0)
    }

    pub fn list_projects(
        conn: &Connection,
        workspace_id: i64,
    ) -> Result<Vec<crate::models::Project>> {
        let mut stmt = conn.prepare(
            "SELECT p.id, p.name, p.path, p.git_hash, p.created_at, p.updated_at, p.is_archived, p.description
             FROM projects p 
             JOIN workspace_projects wp ON p.id = wp.project_id
             WHERE wp.workspace_id = ?1
             ORDER BY p.name"
        )?;

        let projects = stmt
            .query_map([workspace_id], |row| {
                Ok(crate::models::Project {
                    id: Some(row.get(0)?),
                    name: row.get(1)?,
                    path: row.get::<_, String>(2)?.into(),
                    git_hash: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                    is_archived: row.get(6)?,
                    description: row.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(projects)
    }
}

pub struct GitBranchQueries;

impl GitBranchQueries {
    pub fn create_or_update(conn: &Connection, branch: &GitBranch) -> Result<i64> {
        // Try to find existing branch
        let existing =
            Self::find_by_project_and_name(conn, branch.project_id, &branch.branch_name)?;

        if let Some(mut existing) = existing {
            // Update existing
            existing.update_time(branch.total_time_seconds);
            let mut stmt = conn.prepare(
                "UPDATE git_branches SET last_seen = CURRENT_TIMESTAMP, total_time_seconds = total_time_seconds + ?1
                 WHERE project_id = ?2 AND branch_name = ?3"
            )?;
            stmt.execute(params![
                branch.total_time_seconds,
                branch.project_id,
                branch.branch_name
            ])?;
            existing
                .id
                .ok_or_else(|| anyhow::anyhow!("Git branch ID missing after update"))
        } else {
            // Create new
            let mut stmt = conn.prepare(
                "INSERT INTO git_branches (project_id, branch_name, total_time_seconds)
                 VALUES (?1, ?2, ?3)",
            )?;
            stmt.execute(params![
                branch.project_id,
                branch.branch_name,
                branch.total_time_seconds
            ])?;
            Ok(conn.last_insert_rowid())
        }
    }

    pub fn find_by_project_and_name(
        conn: &Connection,
        project_id: i64,
        branch_name: &str,
    ) -> Result<Option<GitBranch>> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, branch_name, first_seen, last_seen, total_time_seconds
             FROM git_branches WHERE project_id = ?1 AND branch_name = ?2",
        )?;

        let branch = stmt
            .query_row(params![project_id, branch_name], |row| {
                Ok(GitBranch {
                    id: Some(row.get(0)?),
                    project_id: row.get(1)?,
                    branch_name: row.get(2)?,
                    first_seen: row.get(3)?,
                    last_seen: row.get(4)?,
                    total_time_seconds: row.get(5)?,
                })
            })
            .optional()?;

        Ok(branch)
    }

    pub fn list_by_project(conn: &Connection, project_id: i64) -> Result<Vec<GitBranch>> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, branch_name, first_seen, last_seen, total_time_seconds 
             FROM git_branches WHERE project_id = ?1 ORDER BY total_time_seconds DESC",
        )?;

        let branches = stmt
            .query_map([project_id], |row| {
                Ok(GitBranch {
                    id: Some(row.get(0)?),
                    project_id: row.get(1)?,
                    branch_name: row.get(2)?,
                    first_seen: row.get(3)?,
                    last_seen: row.get(4)?,
                    total_time_seconds: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(branches)
    }
}

pub struct TimeEstimateQueries;

impl TimeEstimateQueries {
    pub fn create(conn: &Connection, estimate: &TimeEstimate) -> Result<i64> {
        let mut stmt = conn.prepare(
            "INSERT INTO time_estimates (project_id, task_name, estimated_hours, actual_hours, status, due_date, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"
        )?;

        stmt.execute(params![
            estimate.project_id,
            estimate.task_name,
            estimate.estimated_hours,
            estimate.actual_hours,
            estimate.status.to_string(),
            estimate.due_date,
            estimate.completed_at
        ])?;

        Ok(conn.last_insert_rowid())
    }

    pub fn list_by_project(conn: &Connection, project_id: i64) -> Result<Vec<TimeEstimate>> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, task_name, estimated_hours, actual_hours, status, due_date, completed_at, created_at, updated_at 
             FROM time_estimates WHERE project_id = ?1 ORDER BY created_at DESC"
        )?;

        let estimates = stmt
            .query_map([project_id], |row| {
                Ok(TimeEstimate {
                    id: Some(row.get(0)?),
                    project_id: row.get(1)?,
                    task_name: row.get(2)?,
                    estimated_hours: row.get(3)?,
                    actual_hours: row.get(4)?,
                    status: match row.get::<_, String>(5)?.as_str() {
                        "planned" => crate::models::EstimateStatus::Planned,
                        "in_progress" => crate::models::EstimateStatus::InProgress,
                        "completed" => crate::models::EstimateStatus::Completed,
                        "cancelled" => crate::models::EstimateStatus::Cancelled,
                        _ => crate::models::EstimateStatus::Planned,
                    },
                    due_date: row.get(6)?,
                    completed_at: row.get(7)?,
                    created_at: row.get(8)?,
                    updated_at: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(estimates)
    }

    pub fn record_actual(conn: &Connection, estimate_id: i64, hours: f64) -> Result<bool> {
        let mut stmt = conn.prepare(
            "UPDATE time_estimates SET actual_hours = ?1, status = 'completed', completed_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP 
             WHERE id = ?2"
        )?;
        let changes = stmt.execute(params![hours, estimate_id])?;
        Ok(changes > 0)
    }
}

pub struct InsightQueries;

impl InsightQueries {
    pub fn calculate_weekly_summary(
        conn: &Connection,
        week_start: NaiveDate,
    ) -> Result<InsightData> {
        let week_end = week_start + chrono::Duration::days(6);

        let mut stmt = conn.prepare(
            "SELECT 
                COALESCE(SUM(CASE WHEN end_time IS NOT NULL THEN 
                    (julianday(end_time) - julianday(start_time)) * 86400 - COALESCE(paused_duration, 0)
                ELSE 0 END), 0) as total_seconds,
                COUNT(*) as session_count
             FROM sessions
             WHERE DATE(start_time) >= ?1 AND DATE(start_time) <= ?2 AND end_time IS NOT NULL "
        )?;

        let (total_seconds, session_count): (i64, i64) =
            stmt.query_row([week_start, week_end], |row| Ok((row.get(0)?, row.get(1)?)))?;

        let total_hours = total_seconds as f64 / 3600.0;
        let avg_session_duration = if session_count > 0 {
            total_hours / session_count as f64
        } else {
            0.0
        };

        Ok(InsightData {
            total_hours,
            sessions_count: session_count,
            avg_session_duration,
            most_active_day: None,
            most_active_time: None,
            productivity_score: None,
            project_breakdown: vec![],
            trends: vec![],
        })
    }

    pub fn calculate_monthly_summary(
        conn: &Connection,
        month_start: NaiveDate,
    ) -> Result<InsightData> {
        let month_end = month_start + chrono::Duration::days(30);

        let mut stmt = conn.prepare(
            "SELECT 
                COALESCE(SUM(CASE WHEN end_time IS NOT NULL THEN 
                    (julianday(end_time) - julianday(start_time)) * 86400 - COALESCE(paused_duration, 0)
                ELSE 0 END), 0) as total_seconds,
                COUNT(*) as session_count
             FROM sessions
             WHERE DATE(start_time) >= ?1 AND DATE(start_time) <= ?2 AND end_time IS NOT NULL "
        )?;

        let (total_seconds, session_count): (i64, i64) = stmt
            .query_row([month_start, month_end], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })?;

        let total_hours = total_seconds as f64 / 3600.0;
        let avg_session_duration = if session_count > 0 {
            total_hours / session_count as f64
        } else {
            0.0
        };

        Ok(InsightData {
            total_hours,
            sessions_count: session_count,
            avg_session_duration,
            most_active_day: None,
            most_active_time: None,
            productivity_score: None,
            project_breakdown: vec![],
            trends: vec![],
        })
    }
}
