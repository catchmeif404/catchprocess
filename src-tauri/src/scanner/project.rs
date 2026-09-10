//! 프로세스의 작업 디렉터리(cwd)에서 "프로젝트" 정보를 도출한다.
//!
//! - cwd가 git 저장소 안이면: 저장소 루트 경로의 폴더명이 프로젝트 이름, 브랜치는 현재 HEAD.
//! - 아니면: cwd 자체의 폴더명을 이름으로 쓴다 (임시 스크립트 등에 유용).
//!
//! git 호출은 이 모듈의 가장자리에만 존재하고, 파싱/이름 도출은 순수 함수로 유닛 테스트한다.

use std::path::Path;
use std::process::Command;

use serde::Serialize;

/// 서비스 카드에 표시할 프로젝트 정보.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    /// git 저장소 폴더명 또는 cwd 폴더명.
    pub name: String,
    /// 판단 근거가 된 전체 경로 (저장소 루트 또는 cwd).
    pub path: String,
    /// git 저장소가 아닐 경우 생략된다.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

/// 경로의 마지막 구성 요소(폴더명)를 반환한다. 루트/빈 경로는 "unknown".
pub fn derive_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Returns whether a process is a Gradle background daemon rather than an app service.
/// Gradle daemons use a versioned working directory under `.gradle/daemon` and should not
/// appear as development services in the widget.
pub fn is_gradle_daemon_path(path: &Path) -> bool {
    let mut components = path.components().peekable();
    while let Some(component) = components.next() {
        if component.as_os_str() == ".gradle"
            && components
                .next()
                .is_some_and(|next| next.as_os_str() == "daemon")
        {
            return true;
        }
    }
    false
}

/// Homebrew is itself a Git checkout, but directories under its installation prefix are not
/// user projects. Treat those paths as ordinary service directories instead of showing the
/// misleading `homebrew · stable` project label on database cards.
pub fn is_homebrew_root(path: &Path) -> bool {
    matches!(
        path.to_string_lossy().as_ref(),
        "/opt/homebrew" | "/usr/local/Homebrew" | "/usr/local/homebrew"
    )
}

/// `git rev-parse --show-toplevel --abbrev-ref HEAD` 출력을 파싱한다.
/// 첫 줄은 저장소 루트, 둘째 줄은 브랜치 이름.
pub fn parse_git_output(output: &str) -> Option<(String, String)> {
    let mut lines = output.lines().filter(|line| !line.trim().is_empty());
    let toplevel = lines.next()?.trim().to_string();
    if toplevel.is_empty() {
        return None;
    }
    let branch = lines.next()?.trim().to_string();
    if branch.is_empty() {
        return None;
    }
    Some((toplevel, branch))
}

/// cwd에서 프로젝트 정보를 도출한다. cwd를 알 수 없으면 None.
pub fn resolve(cwd: Option<&Path>) -> Option<ProjectInfo> {
    let cwd = cwd?;
    let output = Command::new("git")
        .arg("--no-optional-locks")
        .arg("-C")
        .arg(cwd)
        .args(["rev-parse", "--quiet", "--show-toplevel", "--abbrev-ref", "HEAD"])
        .output();
    if let Ok(output) = output {
        if output.status.success() {
            if let Some((toplevel, branch)) = parse_git_output(&String::from_utf8_lossy(&output.stdout))
            {
                let path = Path::new(&toplevel).to_path_buf();
                if !is_homebrew_root(&path) {
                    return Some(ProjectInfo {
                        name: derive_name(&path),
                        path: toplevel,
                        branch: Some(branch),
                    });
                }
            }
        }
    }
    // git 저장소가 아니거나 git 호출 실패: cwd 폴더명으로 대체한다.
    Some(ProjectInfo {
        name: derive_name(cwd),
        path: cwd.to_string_lossy().to_string(),
        branch: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn derive_name_returns_last_component() {
        assert_eq!(derive_name(Path::new("/Users/k/dev/my-app")), "my-app");
    }

    #[test]
    fn derive_name_handles_root() {
        assert_eq!(derive_name(Path::new("/")), "unknown");
    }

    #[test]
    fn detects_gradle_daemon_directory() {
        assert!(is_gradle_daemon_path(Path::new("/Users/k/.gradle/daemon/9.7.1")));
        assert!(!is_gradle_daemon_path(Path::new("/Users/k/projects/app/.gradle")));
    }

    #[test]
    fn identifies_homebrew_installation_roots() {
        assert!(is_homebrew_root(Path::new("/opt/homebrew")));
        assert!(is_homebrew_root(Path::new("/usr/local/Homebrew")));
        assert!(!is_homebrew_root(Path::new("/Users/k/projects/homebrew-app")));
    }

    #[test]
    fn parse_git_output_reads_toplevel_and_branch() {
        let parsed = parse_git_output("/Users/k/dev/my-app\nfeature/login\n");
        assert_eq!(
            parsed,
            Some(("/Users/k/dev/my-app".to_string(), "feature/login".to_string()))
        );
    }

    #[test]
    fn parse_git_output_rejects_incomplete_output() {
        assert_eq!(parse_git_output("/only/toplevel\n"), None);
        assert_eq!(parse_git_output(""), None);
    }

    #[test]
    fn resolve_falls_back_to_cwd_name_outside_repository() {
        // 존재하지 않는 경로라도 폴더명 도출은 동작한다 (git 실패 경로).
        let info = resolve(Some(&PathBuf::from("/tmp/definitely-not-a-repo-xyz")));
        let info = info.expect("fallback project info");
        assert_eq!(info.name, "definitely-not-a-repo-xyz");
        assert_eq!(info.branch, None);
    }

    #[test]
    fn resolve_returns_none_without_cwd() {
        assert_eq!(resolve(None), None);
    }
}
