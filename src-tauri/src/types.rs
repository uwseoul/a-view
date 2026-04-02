// Shared types for Tauri Commands and Dashboard Service

use serde::{Deserialize, Serialize};

/// Session status enumeration
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Running,
    Delayed,
    Stalled,
    Failed,
    Completed,
}

/// Agent object from sessions
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Agent {
    pub id: String,
    pub name: String,
    pub model: String,
    pub status: Status,
    pub status_label: String,
    pub task: String,
    pub started_at: Option<String>,
    pub last_activity_at: Option<String>,
    pub duration_sec: Option<i64>,
    pub last_event_age_sec: Option<i64>,
    pub is_stalled: bool,
    pub tools: Vec<String>,
    pub recent_logs: Vec<LogEntry>,
}

/// Log entry from messages
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct LogEntry {
    pub time: String,
    pub level: String,
    pub message: String,
}

/// Session object with nested agents
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Session {
    pub id: String,
    pub name: String,
    pub directory: String,
    pub parent_id: Option<String>,
    pub started_at: Option<String>,
    pub last_activity_at: Option<String>,
    pub duration_sec: Option<i64>,
    pub stalled_agent_count: i64,
    pub status_counts: StatusCounts,
    pub agents: Vec<Agent>,
    pub children: Option<Vec<Session>>,
}

/// Status counts for sessions/agents
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub struct StatusCounts {
    #[serde(default)]
    pub running: i64,
    #[serde(default)]
    pub delayed: i64,
    #[serde(default)]
    pub stalled: i64,
    #[serde(default)]
    pub completed: i64,
    #[serde(default)]
    pub failed: i64,
}

/// Snapshot source metadata
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct SnapshotSource {
    pub db_path: String,
    pub mode: String,
    pub refresh_interval_sec: u64,
    pub stalled_threshold_sec: u64,
}

/// Complete dashboard snapshot
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct Snapshot {
    pub generated_at: String,
    pub source: SnapshotSource,
    pub summary: SnapshotSummary,
    pub sessions: Vec<Session>,
}

/// Snapshot summary statistics
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct SnapshotSummary {
    pub running_agents: i64,
    pub suspected_stalled: i64,
    pub total_sessions: i64,
}

// =============================================================================
// Raw database types (for SQLite adapter)
// =============================================================================

/// Raw session row from database
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct RawSession {
    pub id: String,
    pub title: String,
    pub directory: String,
    pub parent_id: Option<String>,
    pub time_created: Option<String>,
    pub time_updated: Option<String>,
}

/// Raw message row from database
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct RawMessage {
    pub id: String,
    pub session_id: String,
    pub time_created: Option<String>,
    pub time_updated: Option<String>,
    pub data: String,
}

/// Raw part row from database
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct RawPart {
    pub id: String,
    pub message_id: String,
    pub session_id: String,
    pub time_created: Option<String>,
    pub time_updated: Option<String>,
    pub data: String,
}

/// Raw todo row from database
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub struct RawTodo {
    pub content: String,
    pub status: String,
    pub priority: Option<String>,
    pub position: i64,
    pub time_created: Option<String>,
    pub time_updated: Option<String>,
}
