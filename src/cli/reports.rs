use anyhow::Result;
use chrono::{DateTime, Utc, NaiveDate, TimeZone, Duration};
use crate::db::{Database, initialize_database};
use crate::utils::paths::get_data_dir;
use rusqlite::{params, Row};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeReport {
    pub period: String,
    pub total_duration: i64,
    pub entries: Vec<ReportEntry>,
    pub projects: HashMap<String, ProjectSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportEntry {
    pub date: String,
    pub project_name: String,
    pub project_path: String,
    pub context: String,
    pub duration: i64,
    pub session_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub name: String,
    pub path: String,
    pub total_duration: i64,
    pub session_count: i32,
    pub contexts: HashMap<String, i64>,
}

pub struct ReportGenerator {
    db: Database,
}

impl ReportGenerator {
    pub fn new() -> Result<Self> {
        let db_path = get_data_dir()?.join("data.db");
        let db = initialize_database(&db_path)?;
        Ok(Self { db })
    }

    pub fn generate_report(
        &self,
        project_filter: Option<String>,
        from_date: Option<String>,
        to_date: Option<String>,
        group_by: Option<String>,
    ) -> Result<TimeReport> {
        let from_date = parse_date(from_date)?;
        let to_date = parse_date(to_date)?;
        let group_by = group_by.unwrap_or_else(|| "day".to_string());

        let entries = self.fetch_report_data(project_filter, from_date, to_date, &group_by)?;
        let projects = self.summarize_by_project(&entries);
        let total_duration = entries.iter().map(|e| e.duration).sum();

        let period = format_period(from_date, to_date);

        Ok(TimeReport {
            period,
            total_duration,
            entries,
            projects,
        })
    }

    pub fn export_csv(&self, report: &TimeReport, output_path: &PathBuf) -> Result<()> {
        use std::fs::File;
        use std::io::Write;

        let mut file = File::create(output_path)?;
        
        // Write headers
        writeln!(file, "Date,Project,Context,Duration (minutes),Session Count")?;
        
        // Write data
        for entry in &report.entries {
            writeln!(
                file,
                "{},{},{},{},{}",
                entry.date,
                entry.project_name,
                entry.context,
                entry.duration / 60,
                entry.session_count
            )?;
        }

        Ok(())
    }

    pub fn export_json(&self, report: &TimeReport, output_path: &PathBuf) -> Result<()> {
        let json = serde_json::to_string_pretty(report)?;
        std::fs::write(output_path, json)?;
        Ok(())
    }

    fn fetch_report_data(
        &self,
        project_filter: Option<String>,
        from_date: Option<DateTime<Utc>>,
        to_date: Option<DateTime<Utc>>,
        group_by: &str,
    ) -> Result<Vec<ReportEntry>> {
        let group_clause = match group_by {
            "day" => "date(s.start_time)",
            "week" => "date(s.start_time, 'weekday 0', '-6 days')",
            "month" => "date(s.start_time, 'start of month')",
            "project" => "'All Time'",
            _ => "date(s.start_time)",
        };

        let mut sql = format!(
            "SELECT 
                {} as period,
                p.name as project_name,
                p.path as project_path,
                s.context,
                SUM(CASE 
                    WHEN s.end_time IS NOT NULL 
                    THEN (julianday(s.end_time) - julianday(s.start_time)) * 86400 - COALESCE(s.paused_duration, 0)
                    ELSE 0
                END) as total_duration,
                COUNT(*) as session_count
            FROM sessions s
            JOIN projects p ON s.project_id = p.id
            WHERE s.end_time IS NOT NULL",
            group_clause
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![];

        if let Some(from) = from_date {
            sql.push_str(" AND s.start_time >= ?");
            params.push(Box::new(from));
        }

        if let Some(to) = to_date {
            sql.push_str(" AND s.start_time <= ?");
            params.push(Box::new(to));
        }

        if let Some(project) = project_filter {
            sql.push_str(" AND (p.name LIKE ? OR p.path LIKE ?)");
            let pattern = format!("%{}%", project);
            params.push(Box::new(pattern.clone()));
            params.push(Box::new(pattern));
        }

        sql.push_str(" GROUP BY period, p.id, s.context ORDER BY period DESC, p.name, s.context");

        let mut stmt = self.db.connection.prepare(&sql)?;
        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
        
        let entries = stmt.query_map(param_refs.as_slice(), |row: &Row| {
            Ok(ReportEntry {
                date: row.get(0)?,
                project_name: row.get(1)?,
                project_path: row.get(2)?,
                context: row.get(3)?,
                duration: row.get::<_, f64>(4)? as i64,
                session_count: row.get(5)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }

    fn summarize_by_project(&self, entries: &[ReportEntry]) -> HashMap<String, ProjectSummary> {
        let mut projects = HashMap::new();

        for entry in entries {
            let summary = projects.entry(entry.project_name.clone()).or_insert_with(|| {
                ProjectSummary {
                    name: entry.project_name.clone(),
                    path: entry.project_path.clone(),
                    total_duration: 0,
                    session_count: 0,
                    contexts: HashMap::new(),
                }
            });

            summary.total_duration += entry.duration;
            summary.session_count += entry.session_count;
            
            *summary.contexts.entry(entry.context.clone()).or_insert(0) += entry.duration;
        }

        projects
    }
}

fn parse_date(date_str: Option<String>) -> Result<Option<DateTime<Utc>>> {
    match date_str {
        Some(date) => {
            let naive_date = NaiveDate::parse_from_str(&date, "%Y-%m-%d")
                .map_err(|_| anyhow::anyhow!("Invalid date format. Use YYYY-MM-DD"))?;
            let datetime = Utc.from_utc_datetime(&naive_date.and_hms_opt(0, 0, 0).unwrap());
            Ok(Some(datetime))
        }
        None => Ok(None),
    }
}

fn format_period(from: Option<DateTime<Utc>>, to: Option<DateTime<Utc>>) -> String {
    match (from, to) {
        (Some(from), Some(to)) => {
            format!("{} to {}", from.format("%Y-%m-%d"), to.format("%Y-%m-%d"))
        }
        (Some(from), None) => {
            format!("From {} to present", from.format("%Y-%m-%d"))
        }
        (None, Some(to)) => {
            format!("Up to {}", to.format("%Y-%m-%d"))
        }
        (None, None) => "All time".to_string(),
    }
}

pub fn print_report(report: &TimeReport) {
    println!("Time Report - {}", report.period);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    let hours = report.total_duration / 3600;
    let minutes = (report.total_duration % 3600) / 60;
    println!("⏱️  Total Time: {}h {}m", hours, minutes);
    println!();

    // Project summary
    if !report.projects.is_empty() {
        println!("📂 Projects:");
        let mut sorted_projects: Vec<_> = report.projects.values().collect();
        sorted_projects.sort_by(|a, b| b.total_duration.cmp(&a.total_duration));
        
        for project in sorted_projects {
            let hours = project.total_duration / 3600;
            let minutes = (project.total_duration % 3600) / 60;
            println!("   {} - {}h {}m ({} sessions)", project.name, hours, minutes, project.session_count);
            
            // Show context breakdown
            for (context, duration) in &project.contexts {
                let ctx_hours = duration / 3600;
                let ctx_minutes = (duration % 3600) / 60;
                println!("     {} {}h {}m", context, ctx_hours, ctx_minutes);
            }
        }
        println!();
    }

    // Daily breakdown (if there are entries)
    if !report.entries.is_empty() {
        println!("📅 Daily Breakdown:");
        let mut current_date = String::new();
        
        for entry in &report.entries {
            if entry.date != current_date {
                if !current_date.is_empty() {
                    println!();
                }
                println!("   {}", entry.date);
                current_date = entry.date.clone();
            }
            
            let hours = entry.duration / 3600;
            let minutes = (entry.duration % 3600) / 60;
            println!("     {} ({}) - {}h {}m", entry.project_name, entry.context, hours, minutes);
        }
    }

    if report.entries.is_empty() {
        println!("No time tracked in this period");
    }
}