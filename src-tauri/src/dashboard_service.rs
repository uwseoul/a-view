// Dashboard snapshot builder service
// Ported from server/dashboard-service.js

use chrono::{DateTime, Utc};

use crate::opencode_adapter::{read_opencode_sessions, DEFAULT_DB_PATH};
use crate::stall_detector::classify_status;
use crate::types::{Agent, LogEntry, Session, Snapshot, SnapshotSource, SnapshotSummary, Status};

/// Title case a status string (first letter uppercase, rest lowercase)
fn title_case_status(status: &str) -> String {
    let mut chars = status.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

/// Sort logs in reverse chronological order, returning up to 20 items
fn sort_logs(logs: Vec<LogEntry>) -> Vec<LogEntry> {
    let mut sorted = logs;
    sorted.sort_by(|a, b| {
        let a_time = DateTime::parse_from_rfc3339(&a.time)
            .map(|dt| dt.timestamp_millis())
            .unwrap_or(i64::MIN);
        let b_time = DateTime::parse_from_rfc3339(&b.time)
            .map(|dt| dt.timestamp_millis())
            .unwrap_or(i64::MIN);
        b_time.cmp(&a_time)
    });
    sorted.into_iter().take(20).collect()
}

/// Nest sessions by parent_id, building a tree structure
fn nest_sessions(flat: Vec<Session>) -> Vec<Session> {
    use std::collections::HashMap;

    // Phase 1: Separate roots and children, build id-to-session map
    let mut id_to_session: HashMap<String, Session> = HashMap::new();
    let mut root_ids: Vec<String> = Vec::new();
    let mut child_relations: Vec<(String, String)> = Vec::new(); // (parent_id, child_id)

    // Collect all ids first to avoid borrow issues
    let session_ids: Vec<String> = flat.iter().map(|s| s.id.clone()).collect();
    let id_set: std::collections::HashSet<String> = session_ids.iter().cloned().collect();

    for session in flat {
        let session_id = session.id.clone();
        let parent_id = session.parent_id.clone();

        if let Some(ref pid) = parent_id {
            if id_set.contains(pid) {
                child_relations.push((pid.clone(), session_id.clone()));
            } else {
                root_ids.push(session_id.clone());
            }
        } else {
            root_ids.push(session_id.clone());
        }
        id_to_session.insert(session_id, session);
    }

    // Phase 2: Sort children by lastActivityAt for each parent
    let mut parent_to_sorted_children: HashMap<String, Vec<String>> = HashMap::new();
    for (parent_id, child_id) in child_relations {
        parent_to_sorted_children
            .entry(parent_id)
            .or_default()
            .push(child_id);
    }

    // Sort each parent's children by lastActivityAt descending
    for children in parent_to_sorted_children.values_mut() {
        children.sort_by(|a, b| {
            let a_time = id_to_session
                .get(a)
                .and_then(|s| s.last_activity_at.as_ref())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0);
            let b_time = id_to_session
                .get(b)
                .and_then(|s| s.last_activity_at.as_ref())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0);
            b_time.cmp(&a_time)
        });
    }

    // Phase 3: Build the tree - collect children first, then assign
    // Do this in two passes to avoid borrow issues
    let mut parent_children: Vec<(String, Vec<String>)> = Vec::new();
    for (parent_id, sorted_children) in parent_to_sorted_children {
        parent_children.push((parent_id, sorted_children));
    }

    // Now assign children to parents
    for (parent_id, child_ids) in parent_children {
        let children: Vec<Session> = child_ids
            .into_iter()
            .filter_map(|cid| id_to_session.remove(&cid))
            .collect();
        if let Some(parent) = id_to_session.get_mut(&parent_id) {
            parent.children = Some(children);
        }
    }

    // Collect roots
    let roots: Vec<Session> = root_ids
        .into_iter()
        .filter_map(|rid| id_to_session.remove(&rid))
        .collect();

    roots
}

/// Build a child session (simplified version for nested children)
fn build_child_session(child: &Session, now: DateTime<Utc>) -> Session {
    let agents: Vec<Agent> = child
        .agents
        .iter()
        .map(|agent| {
            let status = classify_status(
                Some(&agent.status_label.to_lowercase()),
                agent.tools.iter().any(|t| t.contains("running")),
                agent.last_activity_at.as_deref(),
                now,
            );

            let recent_logs: Vec<LogEntry> = sort_logs(agent.recent_logs.clone());
            let latest_log = recent_logs.first();
            let task = if !agent.task.is_empty() {
                agent.task.clone()
            } else if let Some(log) = latest_log {
                log.message.clone()
            } else {
                "활동 요약 없음".to_string()
            };

            let duration_sec = agent
                .started_at
                .as_ref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| {
                    let now_ms = now.timestamp_millis();
                    let dt_ms = dt.timestamp_millis();
                    if now_ms > dt_ms {
                        ((now_ms - dt_ms) / 1000) as i64
                    } else {
                        0
                    }
                });

            let last_event_age_sec = agent
                .last_activity_at
                .as_ref()
                .and_then(|s| {
                    DateTime::parse_from_rfc3339(s)
                        .ok()
                        .map(|dt| ((now.timestamp_millis() - dt.timestamp_millis()) / 1000) as i64)
                })
                .filter(|&v| v >= 0);

            Agent {
                id: agent.id.clone(),
                name: agent.name.clone(),
                model: agent.model.clone(),
                status: status.clone(),
                status_label: title_case_status(&format!("{:?}", status).to_lowercase()),
                task,
                started_at: agent.started_at.clone(),
                last_activity_at: agent.last_activity_at.clone(),
                duration_sec,
                last_event_age_sec,
                is_stalled: status == Status::Stalled,
                tools: agent.tools.iter().take(4).cloned().collect(),
                recent_logs,
            }
        })
        .collect();

    // Compute status counts from reclassified agents
    let mut child_status_counts = crate::types::StatusCounts::default();
    for agent in &agents {
        match agent.status {
            Status::Running => child_status_counts.running += 1,
            Status::Delayed => child_status_counts.delayed += 1,
            Status::Stalled => child_status_counts.stalled += 1,
            Status::Failed => child_status_counts.failed += 1,
            Status::Completed => child_status_counts.completed += 1,
        }
    }

    Session {
        id: child.id.clone(),
        name: child.name.clone(),
        directory: child.directory.clone(),
        parent_id: child.parent_id.clone(),
        started_at: child.started_at.clone(),
        last_activity_at: child.last_activity_at.clone(),
        duration_sec: child.duration_sec,
        stalled_agent_count: agents.iter().filter(|a| a.is_stalled).count() as i64,
        status_counts: child_status_counts,
        agents,
        children: child.children.clone(),
    }
}

/// Build dashboard snapshot
pub fn build_snapshot(
    now: DateTime<Utc>,
    limit: i64,
    db_path: Option<&str>,
) -> Result<Snapshot, String> {
    let flat_sessions = read_opencode_sessions(limit, db_path)?;

    let sessions: Vec<Session> = nest_sessions(flat_sessions)
        .into_iter()
        .map(|session| {
            let agents: Vec<Agent> = session
                .agents
                .iter()
                .map(|agent| {
                    let status = classify_status(
                        Some(&agent.status_label.to_lowercase()),
                        agent.tools.iter().any(|t| t.contains("running")),
                        agent.last_activity_at.as_deref(),
                        now,
                    );

                    let recent_logs: Vec<LogEntry> = sort_logs(agent.recent_logs.clone());
                    let latest_log = recent_logs.first();
                    let task = if !agent.task.is_empty() {
                        agent.task.clone()
                    } else if let Some(log) = latest_log {
                        log.message.clone()
                    } else {
                        "활동 요약 없음".to_string()
                    };

                    let duration_sec = agent
                        .started_at
                        .as_ref()
                        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                        .map(|dt| {
                            let now_ms = now.timestamp_millis();
                            let dt_ms = dt.timestamp_millis();
                            if now_ms > dt_ms {
                                ((now_ms - dt_ms) / 1000) as i64
                            } else {
                                0
                            }
                        });

                    let last_event_age_sec = agent
                        .last_activity_at
                        .as_ref()
                        .and_then(|s| {
                            DateTime::parse_from_rfc3339(s).ok().map(|dt| {
                                ((now.timestamp_millis() - dt.timestamp_millis()) / 1000) as i64
                            })
                        })
                        .filter(|&v| v >= 0);

                    Agent {
                        id: agent.id.clone(),
                        name: agent.name.clone(),
                        model: agent.model.clone(),
                        status: status.clone(),
                        status_label: title_case_status(&format!("{:?}", status).to_lowercase()),
                        task,
                        started_at: agent.started_at.clone(),
                        last_activity_at: agent.last_activity_at.clone(),
                        duration_sec,
                        last_event_age_sec,
                        is_stalled: status == Status::Stalled,
                        tools: agent.tools.iter().take(4).cloned().collect(),
                        recent_logs,
                    }
                })
                .collect();

            // Sort agents by status priority: stalled > delayed > running > failed > completed
            let status_order: std::collections::HashMap<String, i32> = [
                ("stalled".to_string(), 0),
                ("delayed".to_string(), 1),
                ("running".to_string(), 2),
                ("failed".to_string(), 3),
                ("completed".to_string(), 4),
            ]
            .into_iter()
            .collect();

            let mut sorted_agents = agents.clone();
            sorted_agents.sort_by(|a, b| {
                let a_order = status_order
                    .get(&format!("{:?}", a.status).to_lowercase())
                    .copied()
                    .unwrap_or(99);
                let b_order = status_order
                    .get(&format!("{:?}", b.status).to_lowercase())
                    .copied()
                    .unwrap_or(99);
                a_order.cmp(&b_order)
            });

            // Compute status counts
            let mut status_counts = crate::types::StatusCounts::default();
            for agent in &sorted_agents {
                match agent.status {
                    Status::Running => status_counts.running += 1,
                    Status::Delayed => status_counts.delayed += 1,
                    Status::Stalled => status_counts.stalled += 1,
                    Status::Failed => status_counts.failed += 1,
                    Status::Completed => status_counts.completed += 1,
                }
            }

            let children: Vec<Session> = session
                .children
                .as_ref()
                .map(|c| c.iter().map(|ch| build_child_session(ch, now)).collect())
                .unwrap_or_default();

            // Duration from session startedAt to now
            let duration_sec = session
                .started_at
                .as_ref()
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| {
                    let now_ms = now.timestamp_millis();
                    let dt_ms = dt.timestamp_millis();
                    if now_ms > dt_ms {
                        ((now_ms - dt_ms) / 1000) as i64
                    } else {
                        0
                    }
                });

            Session {
                id: session.id.clone(),
                name: session.name.clone(),
                directory: session.directory.clone(),
                parent_id: session.parent_id.clone(),
                started_at: session.started_at.clone(),
                last_activity_at: session.last_activity_at.clone(),
                duration_sec,
                stalled_agent_count: status_counts.stalled,
                status_counts,
                agents: sorted_agents,
                children: if children.is_empty() {
                    None
                } else {
                    Some(children)
                },
            }
        })
        .collect();

    // Compute summary — include children sessions' agent counts
    let summary = sessions.iter().fold(
        SnapshotSummary {
            running_agents: 0,
            suspected_stalled: 0,
            total_sessions: 0,
        },
        |mut acc, session| {
            acc.running_agents += session.status_counts.running;
            acc.suspected_stalled += session.status_counts.stalled;
            acc.total_sessions += 1;
            // Include children sessions
            if let Some(ref children) = session.children {
                for child in children {
                    acc.running_agents += child.status_counts.running;
                    acc.suspected_stalled += child.status_counts.stalled;
                    acc.total_sessions += 1;
                }
            }
            acc
        },
    );

    Ok(Snapshot {
        generated_at: now.to_rfc3339(),
        source: SnapshotSource {
            db_path: DEFAULT_DB_PATH.to_string(),
            mode: "sqlite".to_string(),
            refresh_interval_sec: 5,
            stalled_threshold_sec: 45,
        },
        summary,
        sessions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_title_case_status() {
        assert_eq!(title_case_status("running"), "Running");
        assert_eq!(title_case_status("stalled"), "Stalled");
        assert_eq!(title_case_status("completed"), "Completed");
        assert_eq!(title_case_status(""), "");
    }

    #[test]
    fn test_sort_logs() {
        let logs = vec![
            LogEntry {
                time: "2026-04-01T10:00:00Z".to_string(),
                level: "info".to_string(),
                message: "first".to_string(),
            },
            LogEntry {
                time: "2026-04-01T12:00:00Z".to_string(),
                level: "info".to_string(),
                message: "third".to_string(),
            },
            LogEntry {
                time: "2026-04-01T11:00:00Z".to_string(),
                level: "info".to_string(),
                message: "second".to_string(),
            },
        ];

        let sorted = sort_logs(logs);
        assert_eq!(sorted.len(), 3);
        assert_eq!(sorted[0].message, "third"); // most recent first
        assert_eq!(sorted[1].message, "second");
        assert_eq!(sorted[2].message, "first");
    }

    #[test]
    fn test_sort_logs_limit() {
        let logs: Vec<LogEntry> = (0..25)
            .map(|i| LogEntry {
                time: format!("2026-04-01T{:02}:00:00Z", i),
                level: "info".to_string(),
                message: format!("log{}", i),
            })
            .collect();

        let sorted = sort_logs(logs);
        assert_eq!(sorted.len(), 20); // limited to 20
        assert_eq!(sorted[0].message, "log24"); // most recent first
    }
}
