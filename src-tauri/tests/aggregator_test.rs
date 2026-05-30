use ai_light::aggregator::StateAggregator;
use ai_light::types::{Status, Tool};
use std::path::PathBuf;

#[test]
fn add_session_creates_project_light() {
    let agg = StateAggregator::new();
    let cwd = PathBuf::from("/home/user/project");

    agg.add_session("s1".to_string(), Tool::ClaudeCode, &cwd, Status::Working);

    let lights = agg.get_lights();
    assert_eq!(lights.len(), 1);
    assert_eq!(lights[0].status, Status::Working);
    assert_eq!(lights[0].sessions.len(), 1);
}

#[test]
fn update_session_status_reaggregates_light() {
    let agg = StateAggregator::new();
    let cwd = PathBuf::from("/home/user/project");

    agg.add_session("s1".to_string(), Tool::ClaudeCode, &cwd, Status::Working);
    agg.update_session_status("s1", Status::Done);

    let lights = agg.get_lights();
    assert_eq!(lights[0].status, Status::Done);
}

#[test]
fn remove_working_session_removes_empty_light() {
    let agg = StateAggregator::new();
    let cwd = PathBuf::from("/home/user/project");

    agg.add_session("s1".to_string(), Tool::ClaudeCode, &cwd, Status::Working);
    agg.remove_session("s1");

    assert!(agg.get_lights().is_empty());
}

#[test]
fn done_light_is_retained_until_confirmed() {
    let agg = StateAggregator::new();
    let cwd = PathBuf::from("/home/user/project");

    agg.add_session("s1".to_string(), Tool::ClaudeCode, &cwd, Status::Done);
    let project_id = agg.get_lights()[0].project_id.clone();
    agg.remove_session("s1");

    let lights = agg.get_lights();
    assert_eq!(lights.len(), 1);
    assert_eq!(lights[0].status, Status::Done);

    agg.confirm_light(&project_id);
    assert!(agg.get_lights().is_empty());
}

#[test]
fn aggregates_across_tools_by_severity() {
    let agg = StateAggregator::new();
    let cwd = PathBuf::from("/home/user/project");

    agg.add_session("s1".to_string(), Tool::ClaudeCode, &cwd, Status::Working);
    agg.add_session("s2".to_string(), Tool::Codex, &cwd, Status::Error);

    let lights = agg.get_lights();
    assert_eq!(lights.len(), 1);
    assert_eq!(lights[0].status, Status::Error);
    assert_eq!(lights[0].sessions.len(), 2);
}

#[test]
fn preserves_first_seen_project_order() {
    let agg = StateAggregator::new();

    agg.add_session(
        "s1".to_string(),
        Tool::ClaudeCode,
        &PathBuf::from("/home/user/first"),
        Status::Working,
    );
    agg.add_session(
        "s2".to_string(),
        Tool::Codex,
        &PathBuf::from("/home/user/second"),
        Status::Working,
    );
    agg.update_session_status("s1", Status::Done);

    let labels: Vec<_> = agg
        .get_lights()
        .iter()
        .map(|light| light.project_label.clone())
        .collect();

    assert_eq!(labels, vec!["first", "second"]);
}
