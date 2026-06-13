use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
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
    OpenCode,
    Reasonix,
}

impl Tool {
    pub fn label(&self) -> &'static str {
        match self {
            Tool::ClaudeCode => "Claude Code",
            Tool::Codex => "Codex",
            Tool::OpenCode => "OpenCode",
            Tool::Reasonix => "Reasonix",
        }
    }

    pub fn all() -> &'static [Tool] {
        &[Tool::ClaudeCode, Tool::Codex, Tool::OpenCode, Tool::Reasonix]
    }
}

impl FromStr for Tool {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace(['-', '_', ' '], "") {
            s if s == "claudecode" || s == "claude" || s == "claude-code" => Ok(Tool::ClaudeCode),
            s if s == "codex" => Ok(Tool::Codex),
            s if s == "opencode" || s == "open-code" => Ok(Tool::OpenCode),
            s if s == "reasonix" => Ok(Tool::Reasonix),
            _ => Err(format!("unknown tool: {s}")),
        }
    }
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
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

    pub fn aggregate_status(&mut self) {
        self.status = self
            .sessions
            .iter()
            .map(|s| s.status)
            .max()
            .unwrap_or(Status::Idle);
    }
}
