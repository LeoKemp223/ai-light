use ai_light::types::{LightState, SessionRef, Status, Tool};
use std::time::Instant;

#[test]
fn test_status_ordering() {
    assert!(Status::Error > Status::Working);
    assert!(Status::Working > Status::Done);
    assert!(Status::Done > Status::Idle);
}

#[test]
fn test_status_max() {
    let statuses = vec![Status::Idle, Status::Working, Status::Done];
    assert_eq!(statuses.iter().max(), Some(&Status::Working));

    let with_error = vec![Status::Working, Status::Error, Status::Idle];
    assert_eq!(with_error.iter().max(), Some(&Status::Error));
}

#[test]
fn test_light_state_aggregation() {
    let mut light = LightState::new("/home/user/project".to_string(), "project".to_string());

    // No sessions = Idle
    light.aggregate_status();
    assert_eq!(light.status, Status::Idle);

    // Add working session
    light.sessions.push(SessionRef {
        session_id: "s1".to_string(),
        tool: Tool::ClaudeCode,
        status: Status::Working,
        started_at: Instant::now(),
    });
    light.aggregate_status();
    assert_eq!(light.status, Status::Working);

    // Add error session - should override
    light.sessions.push(SessionRef {
        session_id: "s2".to_string(),
        tool: Tool::Codex,
        status: Status::Error,
        started_at: Instant::now(),
    });
    light.aggregate_status();
    assert_eq!(light.status, Status::Error);
}
