use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Status {
    Idle = 0,
    Done = 1,
    Working = 2,
    Error = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tool {
    ClaudeCode,
    Codex,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionRef {
    pub session_id: String,
    pub tool: Tool,
    pub status: Status,
    #[serde(skip)]
    pub started_at: Instant,
}

#[derive(Debug, Clone, Serialize)]
pub struct LightState {
    pub project_id: String,
    pub project_label: String,
    pub status: Status,
    pub sessions: Vec<SessionRef>,
    #[serde(skip)]
    pub last_event_at: Instant,
    pub last_tool_call: Option<String>,
}

impl LightState {
    pub fn new(project_id: String, project_label: String) -> Self {
        Self {
            project_id,
            project_label,
            status: Status::Idle,
            sessions: Vec::new(),
            last_event_at: Instant::now(),
            last_tool_call: None,
        }
    }

    /// Aggregate status from all sessions (max by severity)
    pub fn aggregate_status(&mut self) {
        self.status = self
            .sessions
            .iter()
            .map(|s| s.status)
            .max()
            .unwrap_or(Status::Idle);
    }
}
