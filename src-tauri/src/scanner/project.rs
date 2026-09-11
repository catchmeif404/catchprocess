//! 프로세스의 작업 디렉터리(cwd)에서 "프로젝트" 정보를 도출한다.
//!
//! - cwd가 git 저장소 안이면: 저장소 루트 경로의 폴더명이 프로젝트 이름, 브랜치는 현재 HEAD.
//! - 아니면: cwd 자체의 폴더명을 이름으로 쓴다 (임시 스크립트 등에 유용).
//!
//! git 호출은 이 모듈의 가장자리에만 존재하고, 파싱/이름 도출은 순수 함수로 유닛 테스트한다.

use std::fs;
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiTarget {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatabaseTarget {
    pub engine: String,
    pub database: String,
    pub port: u16,
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

fn api_target_from_token(token: &str) -> Option<ApiTarget> {
    let token = token.split(|c: char| c.is_whitespace() || matches!(c, '\'' | '"' | '`')).next()?;
    if token.contains('@') || token.contains('$') || token.contains('{') { return None; }
    let token = token.trim_matches(|character: char| {
        matches!(character, '"' | '\'' | '`' | ')' | '}' | ']' | ',' | ';')
    });
    let authority = token
        .strip_prefix("http://")
        .or_else(|| token.strip_prefix("https://"))
        .unwrap_or(token)
        .split('/').next()?;
    let authority = authority.split('?').next()?.split('#').next()?;
    let (host, port) = if let Some(rest) = authority.strip_prefix('[') {
        let (host, port) = rest.split_once("]:")?;
        (host.to_string(), port.parse().ok()?)
    } else {
        match authority.rsplit_once(':') {
            Some((host, port)) if port.parse::<u16>().is_ok() => (host.to_string(), port.parse().ok()?),
            _ => (authority.to_string(), if token.starts_with("https://") { 443 } else { 80 }),
        }
    };
    if host.is_empty() || port == 0 { return None; }
    Some(ApiTarget { host, port })
}

fn extract_api_targets(text: &str) -> Vec<ApiTarget> {
    let mut targets = Vec::new();
    let mut in_comment = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if in_comment {
            if trimmed.contains("*/") { in_comment = false; }
            continue;
        }
        if trimmed.starts_with("/*") { in_comment = !trimmed.contains("*/"); continue; }
        if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with('*') { continue; }
        let lowered = line.to_lowercase();
        if !["api_base_url", "api_base", "api_url", "baseurl", "fetch("].iter().any(|key| lowered.contains(key)) {
            continue;
        }
        for marker in ["http://", "https://", "localhost:", "127.0.0.1:", "[::1]:"] {
            let mut offset = 0;
            while let Some(found) = line[offset..].find(marker) {
                let start = offset + found;
                let token = &line[start..];
                if let Some(target) = api_target_from_token(token) {
                    if !targets.contains(&target) { targets.push(target); }
                }
                offset = start + marker.len();
            }
        }
    }
    targets
}

fn extract_database_targets(text: &str) -> Vec<DatabaseTarget> {
    let mut targets = Vec::new();
    for line in text.lines() {
        let lowered = line.to_lowercase();
        let Some(start) = lowered.find("jdbc:") else { continue; };
        let rest = &line[start + 5..];
        let Some((engine, after_scheme)) = rest.split_once("://") else { continue; };
        let engine = engine.to_lowercase();
        if !matches!(engine.as_str(), "postgresql" | "mysql" | "mariadb" | "sqlserver") { continue; }
        let authority = after_scheme.split('/').next().unwrap_or_default();
        let port = authority
            .rsplit_once(':')
            .and_then(|(_, value)| value.split(['}', '?']).next()?.parse().ok())
            .unwrap_or(match engine.as_str() { "postgresql" => 5432, "sqlserver" => 1433, _ => 3306 });
        let database_part = if engine == "sqlserver" {
            after_scheme
                .split(';')
                .find_map(|part| part.strip_prefix("databaseName="))
                .unwrap_or_default()
        } else {
            let Some(database) = after_scheme.split('/').nth(1) else { continue; };
            database
        };
        let database_part = database_part
            .split(['?', ' ', '"', '\'', '}'])
            .next()
            .unwrap_or_default();
        if database_part.contains("${") && !database_part.contains(':') { continue; }
        let database = database_part
            .split_once(':')
            .map(|(_, value)| value)
            .unwrap_or(database_part)
            .trim_matches(['{', '$', '}'])
            .to_string();
        if database.is_empty() || !database.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.')) { continue; }
        let target = DatabaseTarget { engine, database, port };
        if !targets.contains(&target) { targets.push(target); }
    }
    targets
}

/// Read only shallow source/config files. Values are reduced to host/port pairs immediately;
/// secrets and arbitrary file contents never enter ProjectInfo.
pub fn discover_api_targets(root: &Path) -> Vec<ApiTarget> {
    let mut files = Vec::new();
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        let depth = directory.strip_prefix(root).map(|path| path.components().count()).unwrap_or(99);
        if depth > 4 { continue; }
        let Ok(entries) = fs::read_dir(&directory) else { continue; };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().map(|t| t.is_symlink()).unwrap_or(true) { continue; }
            let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
            if name.ends_with(".d.ts") || name.contains(".test.") || name.contains(".spec.") { continue; }
            if path.is_dir() {
                if !matches!(name, ".git" | "node_modules" | ".next" | "target" | "dist" | "build" | ".gradle") {
                    directories.push(path);
                }
            } else if files.len() < 200 && (path.file_name().and_then(|name| name.to_str()).is_some_and(|name| name == ".env" || name.starts_with(".env.")) || matches!(path.extension().and_then(|ext| ext.to_str()), Some("ts" | "tsx" | "js" | "jsx" | "mjs" | "cjs" | "properties" | "yml" | "yaml"))) {
                files.push(path);
            }
        }
    }
    let mut targets = Vec::new();
    for file in files {
        if fs::metadata(&file).map(|m| m.len() > 512 * 1024).unwrap_or(true) { continue; }
        let Ok(text) = fs::read_to_string(file) else { continue; };
        let text = text.chars().take(512 * 1024).collect::<String>();
        for target in extract_api_targets(&text) {
            if !targets.contains(&target) { targets.push(target); }
        }
    }
    targets.sort_by(|left, right| left.port.cmp(&right.port).then(left.host.cmp(&right.host)));
    targets
}

pub fn discover_database_targets(root: &Path) -> Vec<DatabaseTarget> {
    let mut files = Vec::new();
    let mut directories = vec![root.to_path_buf()];
    while let Some(directory) = directories.pop() {
        let depth = directory.strip_prefix(root).map(|path| path.components().count()).unwrap_or(99);
        if depth > 4 { continue; }
        let Ok(entries) = fs::read_dir(&directory) else { continue; };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().and_then(|name| name.to_str()).unwrap_or_default();
            if path.is_dir() {
                if !matches!(name, ".git" | "node_modules" | ".next" | "target" | "dist" | "build" | ".gradle") {
                    directories.push(path);
                }
            } else if files.len() < 200 && matches!(path.extension().and_then(|ext| ext.to_str()), Some("properties" | "yml" | "yaml")) {
                files.push(path);
            }
        }
    }
    let mut targets = Vec::new();
    for file in files {
        let Ok(text) = fs::read_to_string(file) else { continue; };
        let text = text.chars().take(512 * 1024).collect::<String>();
        for target in extract_database_targets(&text) {
            if !targets.contains(&target) { targets.push(target); }
        }
    }
    targets.sort_by(|left, right| left.port.cmp(&right.port).then(left.database.cmp(&right.database)));
    targets
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
    fn extracts_local_api_target_without_external_urls() {
        let targets = extract_api_targets(
            "const API_BASE_URL = process.env.NEXT_PUBLIC_API_BASE_URL ?? 'http://localhost:8080';\nconst SITE_URL = 'https://example.com';",
        );
        assert_eq!(targets, vec![ApiTarget { host: "localhost".into(), port: 8080 }]);
    }

    #[test]
    fn extracts_external_api_default_https_port() {
        let targets = extract_api_targets("const API_BASE_URL = 'https://api.example.com';");
        assert_eq!(targets, vec![ApiTarget { host: "api.example.com".into(), port: 443 }]);
    }

    #[test]
    fn extracts_api_base_from_env_without_retaining_the_value() {
        let targets = extract_api_targets("API_BASE=http://localhost:8080\nSECRET_TOKEN=not-a-url");
        assert_eq!(targets, vec![ApiTarget { host: "localhost".into(), port: 8080 }]);
    }

    #[test]
    fn ignores_documentation_and_secret_bearing_urls() {
        assert!(extract_api_targets("// see https://nextjs.org/docs/app/api-reference\n/* API_URL = 'https://example.com' */").is_empty());
        assert!(extract_api_targets("const API_URL = 'https://user:secret@api.example.com';").is_empty());
        assert!(extract_database_targets("jdbc:postgresql://localhost:5432/${DB_NAME}").is_empty());
    }

    #[test]
    fn extracts_database_name_without_credentials() {
        let targets = extract_database_targets(
            "spring.datasource.url=jdbc:postgresql://${DB_HOST:localhost}:${DB_PORT:5432}/${DB_NAME:aws_calculator}",
        );
        assert_eq!(
            targets,
            vec![DatabaseTarget { engine: "postgresql".into(), database: "aws_calculator".into(), port: 5432 }]
        );
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
