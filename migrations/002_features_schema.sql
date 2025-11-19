-- Additional schema for advanced features
PRAGMA foreign_keys = ON;

-- Goals and progress tracking
CREATE TABLE IF NOT EXISTS goals (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER,
    name TEXT NOT NULL,
    description TEXT,
    target_hours REAL NOT NULL,
    start_date DATE,
    end_date DATE,
    current_progress REAL DEFAULT 0,
    status TEXT DEFAULT 'active' CHECK(status IN ('active', 'completed', 'paused', 'cancelled')),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX idx_goals_project_id ON goals(project_id);
CREATE INDEX idx_goals_status ON goals(status);

-- Project templates
CREATE TABLE IF NOT EXISTS project_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    description TEXT,
    default_tags TEXT, -- JSON array of tag names
    default_goals TEXT, -- JSON array of goal definitions
    workspace_path TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Workspaces (organizational grouping)
CREATE TABLE IF NOT EXISTS workspaces (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    description TEXT,
    path TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Workspace-project associations
CREATE TABLE IF NOT EXISTS workspace_projects (
    workspace_id INTEGER NOT NULL,
    project_id INTEGER NOT NULL,
    added_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (workspace_id, project_id),
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- Git branch tracking
CREATE TABLE IF NOT EXISTS git_branches (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    branch_name TEXT NOT NULL,
    first_seen TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    last_seen TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    total_time_seconds INTEGER DEFAULT 0,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    UNIQUE(project_id, branch_name)
);

CREATE INDEX idx_git_branches_project ON git_branches(project_id);

-- Add git_branch to sessions
ALTER TABLE sessions ADD COLUMN git_branch TEXT;

-- Time estimates vs actuals
CREATE TABLE IF NOT EXISTS time_estimates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    task_name TEXT NOT NULL,
    estimated_hours REAL NOT NULL,
    actual_hours REAL,
    status TEXT DEFAULT 'planned' CHECK(status IN ('planned', 'in_progress', 'completed', 'cancelled')),
    due_date DATE,
    completed_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX idx_time_estimates_project ON time_estimates(project_id);
CREATE INDEX idx_time_estimates_status ON time_estimates(status);

-- Automatic categorization rules
CREATE TABLE IF NOT EXISTS categorization_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    pattern TEXT NOT NULL, -- Regex pattern or path pattern
    category TEXT NOT NULL,
    tag_id INTEGER,
    priority INTEGER DEFAULT 0,
    is_active BOOLEAN DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE SET NULL
);

CREATE INDEX idx_categorization_rules_active ON categorization_rules(is_active);

-- Calendar events integration
CREATE TABLE IF NOT EXISTS calendar_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    external_id TEXT UNIQUE,
    title TEXT NOT NULL,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP NOT NULL,
    project_id INTEGER,
    session_id INTEGER,
    calendar_type TEXT DEFAULT 'local' CHECK(calendar_type IN ('local', 'google', 'outlook', 'ical')),
    synced_at TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE SET NULL
);

CREATE INDEX idx_calendar_events_time ON calendar_events(start_time, end_time);
CREATE INDEX idx_calendar_events_project ON calendar_events(project_id);

-- Issue tracker connections
CREATE TABLE IF NOT EXISTS issue_trackers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    tracker_type TEXT NOT NULL CHECK(tracker_type IN ('github', 'gitlab', 'jira', 'linear', 'asana', 'trello')),
    api_url TEXT,
    api_key TEXT, -- Encrypted
    workspace_id TEXT,
    project_id TEXT,
    is_active BOOLEAN DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Issue-session associations
CREATE TABLE IF NOT EXISTS issue_sessions (
    issue_id TEXT NOT NULL,
    issue_tracker_id INTEGER NOT NULL,
    session_id INTEGER NOT NULL,
    linked_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (issue_id, issue_tracker_id, session_id),
    FOREIGN KEY (issue_tracker_id) REFERENCES issue_trackers(id) ON DELETE CASCADE,
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_issue_sessions_session ON issue_sessions(session_id);

-- Shared projects (team collaboration)
CREATE TABLE IF NOT EXISTS shared_projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    shared_with TEXT NOT NULL, -- User identifier or email
    permission_level TEXT DEFAULT 'viewer' CHECK(permission_level IN ('viewer', 'contributor', 'admin')),
    shared_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    UNIQUE(project_id, shared_with)
);

CREATE INDEX idx_shared_projects_project ON shared_projects(project_id);

-- Client reporting
CREATE TABLE IF NOT EXISTS client_reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    client_name TEXT NOT NULL,
    project_id INTEGER,
    report_period_start DATE NOT NULL,
    report_period_end DATE NOT NULL,
    total_hours REAL NOT NULL,
    hourly_rate REAL,
    notes TEXT,
    status TEXT DEFAULT 'draft' CHECK(status IN ('draft', 'sent', 'paid')),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    sent_at TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL
);

CREATE INDEX idx_client_reports_client ON client_reports(client_name);
CREATE INDEX idx_client_reports_period ON client_reports(report_period_start, report_period_end);

-- Productivity insights cache
CREATE TABLE IF NOT EXISTS productivity_insights (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER,
    insight_type TEXT NOT NULL CHECK(insight_type IN ('daily', 'weekly', 'monthly', 'project_summary')),
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    data TEXT NOT NULL, -- JSON data
    calculated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    UNIQUE(project_id, insight_type, period_start, period_end)
);

CREATE INDEX idx_productivity_insights_type ON productivity_insights(insight_type, period_start);

-- Team productivity insights
CREATE TABLE IF NOT EXISTS team_insights (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    workspace_id INTEGER,
    team_member TEXT NOT NULL,
    period_start DATE NOT NULL,
    period_end DATE NOT NULL,
    total_hours REAL NOT NULL,
    project_breakdown TEXT, -- JSON
    productivity_score REAL,
    calculated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (workspace_id) REFERENCES workspaces(id) ON DELETE CASCADE
);

CREATE INDEX idx_team_insights_member ON team_insights(team_member, period_start);

-- IDE plugin connections
CREATE TABLE IF NOT EXISTS ide_connections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ide_type TEXT NOT NULL CHECK(ide_type IN ('vscode', 'jetbrains', 'vim', 'emacs', 'sublime')),
    connection_token TEXT UNIQUE NOT NULL,
    last_activity TIMESTAMP,
    is_active BOOLEAN DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_ide_connections_token ON ide_connections(connection_token);
CREATE INDEX idx_ide_connections_active ON ide_connections(is_active);

