-- Initial TimeTrack database schema
-- Version: 001
-- Description: Core tables for projects, sessions, tags, and linked projects

PRAGMA foreign_keys = ON;

-- Schema version tracking
CREATE TABLE schema_version (
    version INTEGER PRIMARY KEY,
    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- INSERT INTO schema_version (version) VALUES (1);

-- Projects table - tracks individual projects
CREATE TABLE projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    path TEXT UNIQUE NOT NULL,
    git_hash TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_archived BOOLEAN DEFAULT 0,
    description TEXT,
    
    CHECK (length(name) > 0),
    CHECK (length(path) > 0)
);

CREATE INDEX idx_projects_path ON projects(path);
CREATE INDEX idx_projects_archived ON projects(is_archived);
CREATE INDEX idx_projects_git_hash ON projects(git_hash) WHERE git_hash IS NOT NULL;

-- Sessions table - tracks time spent on projects
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id INTEGER NOT NULL,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP,
    context TEXT NOT NULL CHECK(context IN ('terminal', 'ide', 'linked', 'manual')),
    paused_duration INTEGER DEFAULT 0,
    active_duration INTEGER GENERATED ALWAYS AS (
        CASE 
            WHEN end_time IS NOT NULL 
            THEN (julianday(end_time) - julianday(start_time)) * 86400 - COALESCE(paused_duration, 0)
            ELSE NULL
        END
    ) STORED,
    notes TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    CHECK (start_time < end_time OR end_time IS NULL),
    CHECK (paused_duration >= 0),
    CHECK (length(context) > 0)
);

CREATE INDEX idx_sessions_project_id ON sessions(project_id);
CREATE INDEX idx_sessions_start_time ON sessions(start_time);
CREATE INDEX idx_sessions_context ON sessions(context);
CREATE INDEX idx_sessions_active ON sessions(end_time) WHERE end_time IS NULL;

-- Tags table for organizing projects
CREATE TABLE tags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    color TEXT,
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    CHECK (length(name) > 0),
    CHECK (name = trim(lower(name)))
);

-- Many-to-many relationship between projects and tags
CREATE TABLE project_tags (
    project_id INTEGER NOT NULL,
    tag_id INTEGER NOT NULL,
    assigned_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    PRIMARY KEY (project_id, tag_id),
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
    FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
);

-- Linked projects for multi-project tracking
CREATE TABLE linked_projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT UNIQUE NOT NULL,
    description TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    is_active BOOLEAN DEFAULT 1,
    
    CHECK (length(name) > 0)
);

-- Members of linked project groups
CREATE TABLE linked_project_members (
    linked_project_id INTEGER NOT NULL,
    project_id INTEGER NOT NULL,
    added_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    PRIMARY KEY (linked_project_id, project_id),
    FOREIGN KEY (linked_project_id) REFERENCES linked_projects(id) ON DELETE CASCADE,
    FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE
);

-- Session edits for audit trail
CREATE TABLE session_edits (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    field_name TEXT NOT NULL,
    old_value TEXT,
    new_value TEXT,
    edit_reason TEXT,
    edited_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE,
    CHECK (field_name IN ('start_time', 'end_time', 'project_id', 'context', 'notes', 'paused_duration'))
);

CREATE INDEX idx_session_edits_session_id ON session_edits(session_id);
CREATE INDEX idx_session_edits_edited_at ON session_edits(edited_at);

-- Configuration storage
CREATE TABLE config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Default configuration will be inserted by application

-- Views for common queries
CREATE VIEW active_sessions AS
SELECT 
    s.*,
    p.name as project_name,
    p.path as project_path
FROM sessions s
JOIN projects p ON s.project_id = p.id
WHERE s.end_time IS NULL;

CREATE VIEW daily_summary AS
SELECT 
    date(s.start_time) as date,
    p.name as project_name,
    s.context,
    COUNT(*) as session_count,
    SUM(s.active_duration) as total_duration
FROM sessions s
JOIN projects p ON s.project_id = p.id
WHERE s.end_time IS NOT NULL
GROUP BY date(s.start_time), p.id, s.context
ORDER BY date DESC;

-- Triggers for maintaining updated_at timestamps
CREATE TRIGGER update_projects_timestamp 
    AFTER UPDATE ON projects
BEGIN
    UPDATE projects SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;

-- Trigger to prevent overlapping sessions
CREATE TRIGGER prevent_overlapping_sessions
    BEFORE INSERT ON sessions
BEGIN
    SELECT CASE
        WHEN EXISTS (
            SELECT 1 FROM sessions 
            WHERE project_id = NEW.project_id 
            AND end_time IS NULL 
            AND id != NEW.id
        )
        THEN RAISE(ABORT, 'Project already has an active session')
    END;
END;