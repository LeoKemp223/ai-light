use std::path::{Path, PathBuf};
use std::process::Command;

/// Identify the project represented by a working directory.
///
/// Returns `(project_id, project_label)`, where `project_id` is the git root
/// when available and otherwise the cwd fallback. `project_label` is the final
/// path component shown under the traffic light.
pub fn identify_project(cwd: &Path) -> (String, String) {
    let project_path = find_git_root(cwd).unwrap_or_else(|| normalize_path(cwd));
    let project_label = project_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();

    (project_path.to_string_lossy().to_string(), project_label)
}

fn find_git_root(cwd: &Path) -> Option<PathBuf> {
    let output = Command::new("git")
        .arg("rev-parse")
        .arg("--show-toplevel")
        .current_dir(cwd)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    let root = stdout.trim();

    if root.is_empty() {
        None
    } else {
        Some(normalize_path(Path::new(root)))
    }
}

fn normalize_path(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_uses_cwd_when_git_root_is_unavailable() {
        let cwd = std::env::temp_dir().join(unique_name("ai-light-no-git"));
        std::fs::create_dir_all(&cwd).unwrap();

        let (project_id, project_label) = identify_project(&cwd);

        assert_eq!(
            project_id,
            normalize_path(&cwd).to_string_lossy().to_string()
        );
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
}
