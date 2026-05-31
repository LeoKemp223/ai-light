use crate::aggregator::StateAggregator;
use crate::types::{Status, Tool};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const POLL_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexSessionMeta {
    pub session_id: String,
    pub cwd: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodexLineEvent {
    SessionMeta(CodexSessionMeta),
    Status(Status),
    ToolCall(String),
    Ignore,
}

#[derive(Debug, Clone)]
struct WatchedRollout {
    offset: u64,
    meta: Option<CodexSessionMeta>,
    added_to_aggregator: bool,
}

pub fn start_codex_watcher(aggregator: Arc<StateAggregator>) -> io::Result<()> {
    thread::Builder::new()
        .name("ai-light-codex-watcher".to_string())
        .spawn(move || run_codex_watcher(aggregator))?;

    Ok(())
}

fn run_codex_watcher(aggregator: Arc<StateAggregator>) {
    let mut files = HashMap::new();
    let mut baseline = true;

    loop {
        let _ = poll_codex_sessions(&aggregator, &mut files, baseline);
        baseline = false;
        thread::sleep(POLL_INTERVAL);
    }
}

fn poll_codex_sessions(
    aggregator: &StateAggregator,
    files: &mut HashMap<PathBuf, WatchedRollout>,
    baseline: bool,
) -> io::Result<()> {
    let root = codex_sessions_dir();
    poll_rollout_root(aggregator, files, baseline, &root)
}

fn poll_rollout_root(
    aggregator: &StateAggregator,
    files: &mut HashMap<PathBuf, WatchedRollout>,
    baseline: bool,
    root: &Path,
) -> io::Result<()> {
    let rollouts = find_rollout_files(&root)?;
    let live_paths: HashSet<_> = rollouts.iter().cloned().collect();

    files.retain(|path, _| live_paths.contains(path));

    for path in rollouts {
        if files.contains_key(&path) {
            process_new_lines(aggregator, files, &path)?;
            continue;
        }

        let mut watched = WatchedRollout {
            offset: 0,
            meta: None,
            added_to_aggregator: false,
        };

        if baseline {
            initialize_existing_rollout(&path, &mut watched)?;
        }

        files.insert(path.clone(), watched);

        if !baseline {
            process_new_lines(aggregator, files, &path)?;
        }
    }

    Ok(())
}

fn initialize_existing_rollout(path: &Path, watched: &mut WatchedRollout) -> io::Result<()> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }

        if watched.meta.is_none() {
            if let Ok(CodexLineEvent::SessionMeta(meta)) = parse_codex_line(line.trim_end()) {
                watched.meta = Some(meta);
            }
        }
    }

    watched.offset = reader.stream_position()?;
    Ok(())
}

fn process_new_lines(
    aggregator: &StateAggregator,
    files: &mut HashMap<PathBuf, WatchedRollout>,
    path: &Path,
) -> io::Result<()> {
    let Some(watched) = files.get_mut(path) else {
        return Ok(());
    };

    let mut file = File::open(path)?;
    file.seek(SeekFrom::Start(watched.offset))?;
    let mut reader = BufReader::new(file);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader.read_line(&mut line)?;
        if bytes == 0 {
            break;
        }

        if let Ok(event) = parse_codex_line(line.trim_end()) {
            apply_codex_event(aggregator, watched, event);
        }
    }

    watched.offset = reader.stream_position()?;
    Ok(())
}

fn apply_codex_event(
    aggregator: &StateAggregator,
    watched: &mut WatchedRollout,
    event: CodexLineEvent,
) {
    match event {
        CodexLineEvent::SessionMeta(meta) => {
            if !watched.added_to_aggregator {
                aggregator.add_session(
                    meta.session_id.clone(),
                    Tool::Codex,
                    &meta.cwd,
                    Status::Idle,
                );
                watched.added_to_aggregator = true;
            }
            watched.meta = Some(meta);
        }
        CodexLineEvent::Status(status) => {
            let Some(meta) = watched.meta.clone() else {
                return;
            };

            if !watched.added_to_aggregator {
                aggregator.add_session(meta.session_id.clone(), Tool::Codex, &meta.cwd, status);
                watched.added_to_aggregator = true;
            } else {
                aggregator.update_session_status(&meta.session_id, status);
            }
        }
        CodexLineEvent::ToolCall(tool_call) => {
            if let Some(meta) = &watched.meta {
                aggregator.set_last_tool_call(&meta.session_id, tool_call);
            }
        }
        CodexLineEvent::Ignore => {}
    }
}

pub fn parse_codex_line(line: &str) -> serde_json::Result<CodexLineEvent> {
    let line = line.trim_start_matches('\u{feff}');

    if line.trim().is_empty() {
        return Ok(CodexLineEvent::Ignore);
    }

    let value: Value = serde_json::from_str(line)?;
    let line_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let payload = value.get("payload").unwrap_or(&Value::Null);

    match line_type {
        "session_meta" => {
            let session_id = payload
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let cwd = payload
                .get("cwd")
                .and_then(Value::as_str)
                .map(PathBuf::from)
                .or_else(|| std::env::current_dir().ok())
                .unwrap_or_else(|| PathBuf::from("."));

            Ok(CodexLineEvent::SessionMeta(CodexSessionMeta {
                session_id,
                cwd,
            }))
        }
        "event_msg" => {
            let event_type = payload
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default();

            Ok(match event_type {
                "task_started" | "agent_message" => CodexLineEvent::Status(Status::Working),
                "task_complete" => CodexLineEvent::Status(Status::Done),
                "error" | "stream_error" | "turn_aborted" => CodexLineEvent::Status(Status::Error),
                _ => CodexLineEvent::Ignore,
            })
        }
        "response_item" => {
            let payload_type = payload
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or_default();

            if payload_type == "function_call" {
                let tool_call = payload
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("tool")
                    .to_string();
                Ok(CodexLineEvent::ToolCall(tool_call))
            } else {
                Ok(CodexLineEvent::Ignore)
            }
        }
        _ => Ok(CodexLineEvent::Ignore),
    }
}

fn find_rollout_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if !root.exists() {
        return Ok(files);
    }

    collect_rollout_files(root, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_rollout_files(dir: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;

        if file_type.is_dir() {
            collect_rollout_files(&path, files)?;
        } else if file_type.is_file() && is_rollout_file(&path) {
            files.push(path);
        }
    }

    Ok(())
}

fn is_rollout_file(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    file_name.starts_with("rollout-") && file_name.ends_with(".jsonl")
}

fn codex_sessions_dir() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".codex")
        .join("sessions")
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_codex_status_events() {
        assert_eq!(
            parse_codex_line(r#"{"type":"event_msg","payload":{"type":"task_started"}}"#).unwrap(),
            CodexLineEvent::Status(Status::Working)
        );
        assert_eq!(
            parse_codex_line(r#"{"type":"event_msg","payload":{"type":"task_complete"}}"#).unwrap(),
            CodexLineEvent::Status(Status::Done)
        );
        assert_eq!(
            parse_codex_line(r#"{"type":"event_msg","payload":{"type":"stream_error"}}"#).unwrap(),
            CodexLineEvent::Status(Status::Error)
        );
    }

    #[test]
    fn parses_codex_session_meta_and_tool_call() {
        assert_eq!(
            parse_codex_line(
                r#"{"type":"session_meta","payload":{"id":"s1","cwd":"N:\\AI\\ai_light"}}"#,
            )
            .unwrap(),
            CodexLineEvent::SessionMeta(CodexSessionMeta {
                session_id: "s1".to_string(),
                cwd: PathBuf::from(r"N:\AI\ai_light"),
            })
        );

        assert_eq!(
            parse_codex_line(
                r#"{"type":"response_item","payload":{"type":"function_call","name":"shell_command"}}"#,
            )
            .unwrap(),
            CodexLineEvent::ToolCall("shell_command".to_string())
        );
    }

    #[test]
    fn polling_new_rollout_drives_codex_light() {
        let root = std::env::temp_dir().join(unique_name("ai-light-codex-root"));
        let project = std::env::temp_dir().join(unique_name("ai-light-codex-project"));
        let day = root.join("2026").join("05").join("31");
        fs::create_dir_all(&day).unwrap();
        fs::create_dir_all(&project).unwrap();

        let rollout = day.join("rollout-2026-05-31T00-00-00-s1.jsonl");
        fs::write(
            &rollout,
            format!(
                "{}\n{}\n",
                json_line(
                    "session_meta",
                    &format!(r#"{{"id":"s1","cwd":"{}"}}"#, json_path(&project))
                ),
                json_line("event_msg", r#"{"type":"task_started"}"#)
            ),
        )
        .unwrap();

        let aggregator = StateAggregator::new();
        let mut files = HashMap::new();
        poll_rollout_root(&aggregator, &mut files, false, &root).unwrap();

        let lights = aggregator.get_lights();
        assert_eq!(lights.len(), 1);
        assert_eq!(lights[0].status, Status::Working);
        assert_eq!(lights[0].sessions[0].tool, Tool::Codex);

        fs::write(
            &rollout,
            format!(
                "{}\n{}\n{}\n{}\n",
                json_line(
                    "session_meta",
                    &format!(r#"{{"id":"s1","cwd":"{}"}}"#, json_path(&project))
                ),
                json_line("event_msg", r#"{"type":"task_started"}"#),
                json_line(
                    "response_item",
                    r#"{"type":"function_call","name":"apply_patch"}"#
                ),
                json_line("event_msg", r#"{"type":"task_complete"}"#)
            ),
        )
        .unwrap();

        poll_rollout_root(&aggregator, &mut files, false, &root).unwrap();
        let lights = aggregator.get_lights();
        assert_eq!(lights[0].status, Status::Done);
        assert_eq!(lights[0].last_tool_call.as_deref(), Some("apply_patch"));

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(project);
    }

    #[test]
    fn baseline_existing_rollout_does_not_replay_history() {
        let root = std::env::temp_dir().join(unique_name("ai-light-codex-root"));
        let project = std::env::temp_dir().join(unique_name("ai-light-codex-project"));
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&project).unwrap();

        let rollout = root.join("rollout-2026-05-31T00-00-00-s1.jsonl");
        fs::write(
            &rollout,
            format!(
                "{}\n{}\n",
                json_line(
                    "session_meta",
                    &format!(r#"{{"id":"s1","cwd":"{}"}}"#, json_path(&project))
                ),
                json_line("event_msg", r#"{"type":"task_complete"}"#)
            ),
        )
        .unwrap();

        let aggregator = StateAggregator::new();
        let mut files = HashMap::new();
        poll_rollout_root(&aggregator, &mut files, true, &root).unwrap();

        assert!(aggregator.get_lights().is_empty());

        fs::write(
            &rollout,
            format!(
                "{}\n{}\n{}\n",
                json_line(
                    "session_meta",
                    &format!(r#"{{"id":"s1","cwd":"{}"}}"#, json_path(&project))
                ),
                json_line("event_msg", r#"{"type":"task_complete"}"#),
                json_line("event_msg", r#"{"type":"task_started"}"#)
            ),
        )
        .unwrap();

        poll_rollout_root(&aggregator, &mut files, false, &root).unwrap();

        let lights = aggregator.get_lights();
        assert_eq!(lights.len(), 1);
        assert_eq!(lights[0].status, Status::Working);

        let _ = fs::remove_dir_all(root);
        let _ = fs::remove_dir_all(project);
    }

    fn json_line(line_type: &str, payload: &str) -> String {
        format!(r#"{{"type":"{line_type}","payload":{payload}}}"#)
    }

    fn json_path(path: &Path) -> String {
        path.to_string_lossy().replace('\\', "\\\\")
    }

    fn unique_name(prefix: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        format!("{prefix}-{nanos}")
    }
}
