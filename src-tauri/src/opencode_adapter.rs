// SQLite adapter for reading OpenCode session data
// Ported from server/opencode-adapter.js

use rusqlite::{Connection, Result};

use crate::types::{
    Agent, LogEntry, Session, Snapshot, SnapshotSource, SnapshotSummary, Status, StatusCounts,
};

// =============================================================================
// Constants
// =============================================================================

pub const DEFAULT_DB_PATH: &str = "~/.local/share/opencode/opencode.db";
pub const DEFAULT_TRANSCRIPTS_DIR: &str = "~/.claude/transcripts";

pub const SYSTEM_AGENT_NAMES: [&str; 2] = ["compaction", "session"];

// =============================================================================
// Helper Functions
// =============================================================================

/// Safe JSON parsing helper - returns None on parse failure
pub fn safe_json_parse(value: &str) -> Option<serde_json::Value> {
    match serde_json::from_str(value) {
        Ok(v) => Some(v),
        Err(_) => None,
    }
}

/// Convert milliseconds to ISO timestamp string
pub fn as_iso_from_ms(ms: i64) -> Option<String> {
    if ms <= 0 {
        return None;
    }
    // Convert milliseconds to seconds and nanoseconds
    let secs = ms / 1000;
    let nsecs = ((ms % 1000) * 1_000_000) as u32;
    let datetime = chrono::DateTime::from_timestamp(secs, nsecs)?;
    Some(datetime.to_rfc3339())
}

/// Format milliseconds as human-readable duration (e.g., "1h 30m", "5m 30s")
pub fn format_duration_from_ms(ms: i64) -> Option<String> {
    if ms <= 0 {
        return None;
    }

    let hours = ms / 3_600_000;
    let minutes = (ms % 3_600_000) / 60_000;
    let seconds = (ms % 60_000) / 1000;

    if hours > 0 {
        Some(format!("{}h {}m", hours, minutes))
    } else if minutes > 0 {
        Some(format!("{}m {}s", minutes, seconds))
    } else if seconds > 0 {
        Some(format!("{}s", seconds))
    } else {
        None
    }
}

/// Get first non-empty string from multiple options
pub fn first_non_empty<'a>(values: impl IntoIterator<Item = Option<&'a str>>) -> Option<&'a str> {
    for value in values {
        if let Some(v) = value {
            if !v.trim().is_empty() {
                return Some(v.trim());
            }
        }
    }
    None
}

/// Pick agent name from message JSON
fn pick_agent_name(message_json: &serde_json::Value) -> String {
    let empty_map = serde_json::Map::new();
    let obj = message_json.as_object().unwrap_or(&empty_map);

    first_non_empty([
        obj.get("agent").and_then(|v| v.as_str()),
        obj.get("mode").and_then(|v| v.as_str()),
        obj.get("model")
            .and_then(|v| v.get("modelID"))
            .and_then(|v| v.as_str()),
    ])
    .unwrap_or("session")
    .to_string()
}

/// Pick model identifier from message JSON
fn pick_model(message_json: &serde_json::Value) -> String {
    let empty_map = serde_json::Map::new();
    let obj = message_json.as_object().unwrap_or(&empty_map);

    // Try providerID/modelID format first
    if let (Some(provider), Some(model)) = (
        obj.get("model")
            .and_then(|v| v.get("providerID"))
            .and_then(|v| v.as_str()),
        obj.get("model")
            .and_then(|v| v.get("modelID"))
            .and_then(|v| v.as_str()),
    ) {
        return format!("{}/{}", provider, model);
    }

    // Try flat providerID/modelID
    if let (Some(provider), Some(model)) = (
        obj.get("providerID").and_then(|v| v.as_str()),
        obj.get("modelID").and_then(|v| v.as_str()),
    ) {
        return format!("{}/{}", provider, model);
    }

    // Fallback to modelID or "unknown"
    obj.get("modelID")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Summarize a part JSON into a display message
fn summarize_part(part_json: &serde_json::Value) -> Option<String> {
    let obj = part_json.as_object()?;
    let part_type = obj.get("type")?.as_str()?;

    match part_type {
        "text" => {
            first_non_empty([obj.get("text").and_then(|v| v.as_str())]).map(|s| s.to_string())
        }
        "tool" => {
            let tool =
                first_non_empty([obj.get("tool").and_then(|v| v.as_str())]).unwrap_or("tool");
            let status = obj
                .get("state")
                .and_then(|v| v.get("status"))
                .and_then(|v| v.as_str());
            let input = obj
                .get("state")
                .and_then(|v| v.get("input"))
                .and_then(|v| v.get("command"))
                .and_then(|v| v.as_str());

            if let (Some(s), Some(i)) = (status, input) {
                Some(format!("{} · {} · [{}]", tool, s, i))
            } else if let Some(s) = status {
                Some(format!("{} · {}", tool, s))
            } else if let Some(i) = input {
                Some(format!("[{}]", i))
            } else {
                Some(tool.to_string())
            }
        }
        "step-start" => Some("작업 단계 시작".to_string()),
        "step-finish" => Some("작업 단계 완료".to_string()),
        "reasoning" => Some("Reasoning update".to_string()),
        _ => first_non_empty([
            obj.get("text").and_then(|v| v.as_str()),
            obj.get("tool").and_then(|v| v.as_str()),
        ])
        .map(|s| s.to_string()),
    }
}

/// Derive log level from part and message JSON
fn derive_log_level(part_json: &serde_json::Value, message_json: &serde_json::Value) -> String {
    let empty_map = serde_json::Map::new();
    let part_obj = part_json.as_object().unwrap_or(&empty_map);
    let msg_obj = message_json.as_object().unwrap_or(&empty_map);

    // Check explicit status first
    if let Some(status) = part_obj
        .get("state")
        .and_then(|v| v.get("status"))
        .and_then(|v| v.as_str())
    {
        match status {
            "failed" | "error" => return "error".to_string(),
            "completed" | "success" | "done" => return "success".to_string(),
            "pending" => return "warn".to_string(),
            "running" => {}
            _ => {}
        }
    }

    // Check part type
    if part_obj.get("type").and_then(|v| v.as_str()) == Some("tool") {
        return "tool".to_string();
    }

    // Default based on message role
    if msg_obj.get("role").and_then(|v| v.as_str()) == Some("assistant") {
        "info".to_string()
    } else {
        "info".to_string()
    }
}

// =============================================================================
// SQLite Query Execution
// =============================================================================

// NOTE: execute_query function was removed - use direct query methods instead

/// Simple base64 encoding for blob data
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as usize;
        let b1 = chunk.get(1).copied().unwrap_or(0) as usize;
        let b2 = chunk.get(2).copied().unwrap_or(0) as usize;

        result.push(ALPHABET[b0 >> 2] as char);
        result.push(ALPHABET[((b0 & 0x03) << 4) | (b1 >> 4)] as char);

        if chunk.len() > 1 {
            result.push(ALPHABET[((b1 & 0x0f) << 2) | (b2 >> 6)] as char);
        } else {
            result.push('=');
        }

        if chunk.len() > 2 {
            result.push(ALPHABET[b2 & 0x3f] as char);
        } else {
            result.push('=');
        }
    }

    result
}

// =============================================================================
// Raw Data Reading Functions
// =============================================================================

/// Read raw session data from database
pub fn read_raw_sessions(
    limit: i64,
    db_path: Option<&str>,
) -> Result<Vec<crate::types::RawSession>, String> {
    let db_path = db_path.unwrap_or(DEFAULT_DB_PATH);

    // Expand tilde to home directory
    let db_path = if db_path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            db_path.replacen("~", &home, 1)
        } else if let Ok(home) = std::env::var("USERPROFILE") {
            db_path.replacen("~", &home, 1)
        } else {
            db_path.to_string()
        }
    } else {
        db_path.to_string()
    };

    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    let query = r#"
        SELECT id, title, directory, parent_id, time_created, time_updated
        FROM session
        WHERE time_archived IS NULL
        ORDER BY time_updated DESC
        LIMIT ?1
    "#;

    let mut stmt = conn
        .prepare(query)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let sessions = stmt
        .query_map([limit], |row| {
            Ok(crate::types::RawSession {
                id: row.get(0)?,
                title: row.get(1)?,
                directory: row.get(2)?,
                parent_id: row.get(3)?,
                time_created: row.get(4)?,
                time_updated: row.get(5)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(sessions)
}

/// Read messages for a specific session
pub fn read_raw_messages(
    session_id: &str,
    limit: i64,
    db_path: Option<&str>,
) -> Result<Vec<crate::types::RawMessage>, String> {
    let db_path = db_path.unwrap_or(DEFAULT_DB_PATH);

    let db_path = if db_path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            db_path.replacen("~", &home, 1)
        } else if let Ok(home) = std::env::var("USERPROFILE") {
            db_path.replacen("~", &home, 1)
        } else {
            db_path.to_string()
        }
    } else {
        db_path.to_string()
    };

    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    let query = r#"
        SELECT id, session_id, time_created, time_updated, data
        FROM message
        WHERE session_id = ?1
        ORDER BY time_updated DESC
        LIMIT ?2
    "#;

    let mut stmt = conn
        .prepare(query)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let messages = stmt
        .query_map([session_id, &limit.to_string()], |row| {
            Ok(crate::types::RawMessage {
                id: row.get(0)?,
                session_id: row.get(1)?,
                time_created: row.get(2)?,
                time_updated: row.get(3)?,
                data: row.get(4)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(messages)
}

/// Read parts for a specific session
pub fn read_raw_parts(
    session_id: &str,
    limit: i64,
    db_path: Option<&str>,
) -> Result<Vec<crate::types::RawPart>, String> {
    let db_path = db_path.unwrap_or(DEFAULT_DB_PATH);

    let db_path = if db_path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            db_path.replacen("~", &home, 1)
        } else if let Ok(home) = std::env::var("USERPROFILE") {
            db_path.replacen("~", &home, 1)
        } else {
            db_path.to_string()
        }
    } else {
        db_path.to_string()
    };

    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    let query = r#"
        SELECT id, message_id, session_id, time_created, time_updated, data
        FROM part
        WHERE session_id = ?1
        ORDER BY time_updated DESC
        LIMIT ?2
    "#;

    let mut stmt = conn
        .prepare(query)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let parts = stmt
        .query_map([session_id, &limit.to_string()], |row| {
            Ok(crate::types::RawPart {
                id: row.get(0)?,
                message_id: row.get(1)?,
                session_id: row.get(2)?,
                time_created: row.get(3)?,
                time_updated: row.get(4)?,
                data: row.get(5)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(parts)
}

/// Read todos for a specific session
pub fn read_raw_todos(
    session_id: &str,
    db_path: Option<&str>,
) -> Result<Vec<crate::types::RawTodo>, String> {
    let db_path = db_path.unwrap_or(DEFAULT_DB_PATH);

    let db_path = if db_path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            db_path.replacen("~", &home, 1)
        } else if let Ok(home) = std::env::var("USERPROFILE") {
            db_path.replacen("~", &home, 1)
        } else {
            db_path.to_string()
        }
    } else {
        db_path.to_string()
    };

    let conn = Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

    let query = r#"
        SELECT content, status, priority, position, time_created, time_updated
        FROM todo
        WHERE session_id = ?1
        ORDER BY position ASC
    "#;

    let mut stmt = conn
        .prepare(query)
        .map_err(|e| format!("Failed to prepare query: {}", e))?;

    let todos = stmt
        .query_map([session_id], |row| {
            Ok(crate::types::RawTodo {
                content: row.get(0)?,
                status: row.get(1)?,
                priority: row.get(2)?,
                position: row.get(3)?,
                time_created: row.get(4)?,
                time_updated: row.get(5)?,
            })
        })
        .map_err(|e| format!("Query failed: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    Ok(todos)
}

// =============================================================================
// Session Normalization
// =============================================================================

/// Normalize a raw session into a full Session object with agents
pub fn normalize_raw_session(
    raw: &crate::types::RawSession,
    messages: &[crate::types::RawMessage],
    parts: &[crate::types::RawPart],
    todos: &[crate::types::RawTodo],
) -> Session {
    let mut agent_buckets: std::collections::HashMap<String, Agent> =
        std::collections::HashMap::new();

    // Track the latest status per agent from parts
    let mut agent_latest_status: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();

    // Build message map for part-to-agent resolution (reused later too)
    let message_data_map: std::collections::HashMap<&str, &str> =
        messages.iter().map(|m| (m.id.as_str(), m.data.as_str())).collect();

    // Pre-scan parts to determine agent status from their latest tool state
    for part_row in parts {
        let part_json = safe_json_parse(&part_row.data)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        let msg_data = message_data_map.get(part_row.message_id.as_str()).copied();
        let message_json = msg_data
            .and_then(|data| safe_json_parse(data))
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        let agent_name = pick_agent_name(&message_json);
        let agent_id = format!("{}:{}", raw.id, agent_name);

        if let Some(status) = part_json
            .get("state")
            .and_then(|v| v.get("status"))
            .and_then(|v| v.as_str())
        {
            // Parts sorted DESC — first match = newest status, keep it
            agent_latest_status.entry(agent_id).or_insert(status.to_string());
        }
    }

    // Process messages to build agents
    let mut sorted_messages = messages.to_vec();
    sorted_messages.sort_by(|a, b| {
        let a_time = a.time_created.unwrap_or(0);
        let b_time = b.time_created.unwrap_or(0);
        a_time.cmp(&b_time)
    });

    for msg_row in &sorted_messages {
        let message_json = safe_json_parse(&msg_row.data)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        let agent_name = pick_agent_name(&message_json);
        let model = pick_model(&message_json);
        let created_at = msg_row.time_created.and_then(|ms| as_iso_from_ms(ms));

        let agent_id = format!("{}:{}", raw.id, agent_name);

        // Determine initial status from pre-scanned latest part status
        let (_initial_status, initial_label) = agent_latest_status
            .get(&agent_id)
            .map(|s| {
                let label = match s.as_str() {
                    "completed" | "success" | "done" => ("Completed", "Completed"),
                    "failed" | "error" => ("Failed", "Failed"),
                    "running" | "pending" => ("Running", "Running"),
                    _ => ("Running", "Running"),
                };
                (label.0.to_string(), label.1.to_string())
            })
            .unwrap_or_else(|| ("unknown".to_string(), "unknown".to_string()));

        // Ensure agent exists with id set
        if !agent_buckets.contains_key(&agent_id) {
            agent_buckets.insert(
                agent_id.clone(),
                Agent {
                    id: agent_id.clone(),
                    name: agent_name.clone(),
                    model: model.clone(),
                    status: Status::Running,
                    status_label: initial_label,
                    task: String::new(),
                    started_at: created_at.clone(),
                    last_activity_at: None,
                    duration_sec: None,
                    last_event_age_sec: None,
                    is_stalled: false,
                    tools: Vec::new(),
                    recent_logs: Vec::new(),
                },
            );
        }

        let agent = agent_buckets.get_mut(&agent_id).unwrap();

        if let Some(updated_at) = msg_row
            .time_updated
            .as_ref()
            .copied()
            .and_then(|ms| as_iso_from_ms(ms))
        {
            if agent.last_activity_at.is_none()
                || updated_at > agent.last_activity_at.clone().unwrap_or_default()
            {
                agent.last_activity_at = Some(updated_at);
            }
        }

        // Extract task from user messages
        if agent.task.is_empty() {
            if let Some(role) = message_json.get("role").and_then(|v| v.as_str()) {
                if role == "user" {
                    if let Some(content) = message_json.get("content").and_then(|v| v.as_str()) {
                        agent.task = content.chars().take(120).collect();
                    } else if let Some(summary) =
                        message_json.get("summary").and_then(|v| v.as_str())
                    {
                        agent.task = summary.chars().take(120).collect();
                    }
                }
            }
        }

        // Fallback task extraction
        if agent.task.is_empty() {
            if let Some(summary_text) = message_json
                .get("summary")
                .and_then(|v| v.get("text"))
                .and_then(|v| v.as_str())
            {
                agent.task = summary_text.chars().take(120).collect();
            } else if let Some(title) = message_json.get("title").and_then(|v| v.as_str()) {
                agent.task = title.chars().take(120).collect();
            }
        }
    }

    // Process parts to enrich agents
    for part_row in parts {
        let part_json = safe_json_parse(&part_row.data)
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
        let message_json = message_data_map
            .get(part_row.message_id.as_str())
            .and_then(|data| safe_json_parse(data))
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let agent_name = pick_agent_name(&message_json);
        let model = pick_model(&message_json);
        let created_at = part_row.time_created.and_then(|ms| as_iso_from_ms(ms));

        let agent_id = format!("{}:{}", raw.id, agent_name);

        // Determine initial status from pre-scanned latest part status
        let (_initial_status, initial_label) = agent_latest_status
            .get(&agent_id)
            .map(|s| {
                let label = match s.as_str() {
                    "completed" | "success" | "done" => ("Completed", "Completed"),
                    "failed" | "error" => ("Failed", "Failed"),
                    "running" | "pending" => ("Running", "Running"),
                    _ => ("Running", "Running"),
                };
                (label.0.to_string(), label.1.to_string())
            })
            .unwrap_or_else(|| ("unknown".to_string(), "unknown".to_string()));

        // Ensure agent exists
        if !agent_buckets.contains_key(&agent_id) {
            agent_buckets.insert(
                agent_id.clone(),
                Agent {
                    id: agent_id.clone(),
                    name: agent_name.clone(),
                    model: model.clone(),
                    status: Status::Running,
                    status_label: initial_label,
                    task: String::new(),
                    started_at: created_at,
                    last_activity_at: None,
                    duration_sec: None,
                    last_event_age_sec: None,
                    is_stalled: false,
                    tools: Vec::new(),
                    recent_logs: Vec::new(),
                },
            );
        }

        let agent = agent_buckets.get_mut(&agent_id).unwrap();

        if let Some(updated_at) = part_row
            .time_updated
            .as_ref()
            .copied()
            .and_then(|ms| as_iso_from_ms(ms))
        {
            if agent.last_activity_at.is_none()
                || updated_at > agent.last_activity_at.clone().unwrap_or_default()
            {
                agent.last_activity_at = Some(updated_at);
            }
        }

        // Track tools
        if let Some(tool_name) = part_json.get("tool").and_then(|v| v.as_str()) {
            if !agent.tools.contains(&tool_name.to_string()) {
                agent.tools.push(tool_name.to_string());
            }
        }

        // Check running tool status
        if let Some(status) = part_json
            .get("state")
            .and_then(|v| v.get("status"))
            .and_then(|v| v.as_str())
        {
            if status == "running" {
                // Update task with running command
                if let Some(cmd) = part_json
                    .get("state")
                    .and_then(|v| v.get("input"))
                    .and_then(|v| v.get("command"))
                    .and_then(|v| v.as_str())
                {
                    if let Some(tool) = part_json.get("tool").and_then(|v| v.as_str()) {
                        agent.task = format!("[{}] {}", tool, cmd);
                    }
                }
            }
        }

        // Add log entry
        if let Some(log_message) = summarize_part(&part_json) {
            let updated_at = part_row
                .time_updated
                .as_ref()
                .copied()
                .and_then(|ms| as_iso_from_ms(ms))
                .unwrap_or_default();

            agent.recent_logs.push(LogEntry {
                time: updated_at,
                level: derive_log_level(&part_json, &message_json),
                message: log_message.clone(),
            });

            // Use first non-reasoning log as task fallback
            if agent.task.is_empty()
                && part_json.get("type").and_then(|v| v.as_str()) != Some("reasoning")
            {
                agent.task = log_message;
            }
        }
    }

    // Filter out system agents if we have more than one agent
    let mut agents: Vec<Agent> = if agent_buckets.len() > 1 {
        agent_buckets
            .into_values()
            .filter(|a| !SYSTEM_AGENT_NAMES.contains(&a.name.as_str()))
            .collect()
    } else {
        agent_buckets.into_values().collect()
    };

    // Ensure at least one agent exists
    if agents.is_empty() {
        let started_at = raw.time_created.and_then(|ms| as_iso_from_ms(ms));
        let last_activity_at = raw.time_updated.and_then(|ms| as_iso_from_ms(ms));

        let todo_summary = if !todos.is_empty() {
            let completed = todos.iter().filter(|t| t.status == "completed").count();
            Some(format!("Todo {}/{}", completed, todos.len()))
        } else {
            None
        };

        agents.push(Agent {
            id: format!("{}:session", raw.id),
            name: "session".to_string(),
            model: "unknown".to_string(),
            status: Status::Running,
            status_label: "Running".to_string(),
            task: todo_summary.unwrap_or_else(|| "세션 활동 없음".to_string()),
            started_at,
            last_activity_at,
            duration_sec: None,
            last_event_age_sec: None,
            is_stalled: false,
            tools: Vec::new(),
            recent_logs: Vec::new(),
        });
    }

    // Calculate duration
    let duration_sec = if let (Some(start), Some(end)) = (raw.time_created, raw.time_updated) {
        Some((end - start) / 1000)
    } else {
        None
    };

    // Build status counts from agents
    let mut status_counts = StatusCounts::default();
    for agent in &agents {
        match agent.status {
            Status::Running => status_counts.running += 1,
            Status::Delayed => status_counts.delayed += 1,
            Status::Stalled => status_counts.stalled += 1,
            Status::Failed => status_counts.failed += 1,
            Status::Completed => status_counts.completed += 1,
        }
    }

    let stalled_agent_count = agents.iter().filter(|a| a.is_stalled).count() as i64;

    Session {
        id: raw.id.clone(),
        name: raw.title.clone(),
        directory: raw.directory.clone(),
        parent_id: raw.parent_id.clone(),
        started_at: raw
            .time_created
            .as_ref()
            .copied()
            .and_then(|ms| as_iso_from_ms(ms)),
        last_activity_at: raw
            .time_updated
            .as_ref()
            .copied()
            .and_then(|ms| as_iso_from_ms(ms)),
        duration_sec,
        stalled_agent_count,
        status_counts,
        agents,
        children: None,
    }
}

/// Read and normalize all sessions
pub fn read_opencode_sessions(limit: i64, db_path: Option<&str>) -> Result<Vec<Session>, String> {
    let raw_sessions = read_raw_sessions(limit, db_path)?;

    let mut sessions = Vec::new();
    for raw in &raw_sessions {
        let messages = read_raw_messages(&raw.id, 80, db_path)?;
        let parts = read_raw_parts(&raw.id, 160, db_path)?;
        let todos = read_raw_todos(&raw.id, db_path)?;

        let session = normalize_raw_session(raw, &messages, &parts, &todos);
        sessions.push(session);
    }

    Ok(sessions)
}

// =============================================================================
// Tauri Commands
// =============================================================================

/// Tauri command to read all sessions from the database
#[tauri::command]
pub fn read_sessions(limit: i64) -> Result<Snapshot, String> {
    let sessions = read_opencode_sessions(limit, None)?;

    let running_agents = sessions
        .iter()
        .flat_map(|s| &s.agents)
        .filter(|a| a.status == Status::Running)
        .count() as i64;

    let suspected_stalled = sessions
        .iter()
        .flat_map(|s| &s.agents)
        .filter(|a| a.status == Status::Stalled)
        .count() as i64;

    Ok(Snapshot {
        generated_at: chrono::Utc::now().to_rfc3339(),
        source: SnapshotSource {
            db_path: DEFAULT_DB_PATH.to_string(),
            mode: "sqlite".to_string(),
            refresh_interval_sec: 300,
            stalled_threshold_sec: 45,
        },
        summary: SnapshotSummary {
            running_agents,
            suspected_stalled,
            total_sessions: sessions.len() as i64,
        },
        sessions,
    })
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_json_parse() {
        assert!(safe_json_parse(r#"{"key": "value"}"#).is_some());
        assert!(safe_json_parse("invalid").is_none());
        assert!(safe_json_parse("").is_some()); // Empty object is valid JSON
    }

    #[test]
    fn test_format_duration_from_ms() {
        assert_eq!(
            format_duration_from_ms(3_600_000),
            Some("1h 0m".to_string())
        ); // 1 hour
        assert_eq!(
            format_duration_from_ms(3_600_000 + 1_800_000),
            Some("1h 30m".to_string())
        ); // 1.5 hours
        assert_eq!(format_duration_from_ms(60_000), Some("1m 0s".to_string())); // 1 minute
        assert_eq!(format_duration_from_ms(90_000), Some("1m 30s".to_string())); // 1.5 minutes
        assert_eq!(format_duration_from_ms(30_000), Some("0m 30s".to_string())); // 30 seconds
        assert_eq!(format_duration_from_ms(0), None);
        assert!(format_duration_from_ms(-1000).is_none());
    }

    #[test]
    fn test_first_non_empty() {
        assert_eq!(
            first_non_empty([None, Some(""), Some("value")]),
            Some("value")
        );
        assert_eq!(first_non_empty([None, None]), None);
        assert_eq!(first_non_empty([Some("  "), Some("value")]), Some("value"));
    }

    #[test]
    fn test_pick_agent_name() {
        let json: serde_json::Value = serde_json::json!({
            "agent": "test-agent"
        });
        assert_eq!(pick_agent_name(&json), "test-agent");

        let json2: serde_json::Value = serde_json::json!({
            "mode": "test-mode"
        });
        assert_eq!(pick_agent_name(&json2), "test-mode");

        let json3: serde_json::Value = serde_json::json!({});
        assert_eq!(pick_agent_name(&json3), "session");
    }

    #[test]
    fn test_pick_model() {
        let json: serde_json::Value = serde_json::json!({
            "model": {
                "providerID": "openai",
                "modelID": "gpt-4"
            }
        });
        assert_eq!(pick_model(&json), "openai/gpt-4");

        let json2: serde_json::Value = serde_json::json!({
            "providerID": "anthropic",
            "modelID": "claude-3"
        });
        assert_eq!(pick_model(&json2), "anthropic/claude-3");

        let json3: serde_json::Value = serde_json::json!({
            "modelID": "o3-mini"
        });
        assert_eq!(pick_model(&json3), "o3-mini");

        let json4: serde_json::Value = serde_json::json!({});
        assert_eq!(pick_model(&json4), "unknown");
    }
}
