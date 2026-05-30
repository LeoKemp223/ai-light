use ai_light::project::identify_project;
use std::path::Path;
use std::process::Command;

#[test]
fn identifies_git_root_from_nested_directory() {
    let repo = std::env::temp_dir().join(unique_name("ai-light-git-repo"));
    let nested = repo.join("src").join("nested");
    std::fs::create_dir_all(&nested).unwrap();

    let status = Command::new("git")
        .arg("init")
        .arg("--quiet")
        .current_dir(&repo)
        .status()
        .expect("git should be available for project identification tests");
    assert!(status.success());

    let (project_id, project_label) = identify_project(&nested);

    assert_eq!(Path::new(&project_id), repo.canonicalize().unwrap());
    assert_eq!(project_label, repo.file_name().unwrap().to_string_lossy());

    std::fs::remove_dir_all(repo).unwrap();
}

#[test]
fn falls_back_to_cwd_outside_git_repo() {
    let cwd = std::env::temp_dir().join(unique_name("ai-light-plain-dir"));
    std::fs::create_dir_all(&cwd).unwrap();

    let (project_id, project_label) = identify_project(&cwd);

    assert_eq!(Path::new(&project_id), cwd.canonicalize().unwrap());
    assert_eq!(project_label, cwd.file_name().unwrap().to_string_lossy());

    std::fs::remove_dir_all(cwd).unwrap();
}

fn unique_name(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    format!("{prefix}-{nanos}")
}
