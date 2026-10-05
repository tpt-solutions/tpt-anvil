// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Verification gate — runs compiler checks and linters on generated diffs
//! before applying them, providing a safety net against broken code.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tracing::info;

/// Configuration for the verification gate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyConfig {
    pub enabled: bool,
    pub run_tests: bool,
    pub run_linter: bool,
    pub timeout_seconds: u64,
    pub max_retries: u32,
}

impl Default for VerifyConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            run_tests: false,
            run_linter: true,
            timeout_seconds: 60,
            max_retries: 1,
        }
    }
}

/// Result of verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub passed: bool,
    pub compiler_output: Option<String>,
    pub test_output: Option<String>,
    pub lint_output: Option<String>,
    pub errors: Vec<String>,
    /// Number of retry attempts that were made after the initial attempt.
    #[serde(default)]
    pub retries_used: u32,
    /// Maximum retries configured (from `VerifyConfig::max_retries`).
    #[serde(default)]
    pub max_retries: u32,
}

/// Determine the language of a file based on its extension.
fn detect_language(file_path: &str) -> Option<&'static str> {
    let ext = Path::new(file_path).extension()?.to_str()?;
    match ext {
        "rs" => Some("rust"),
        "py" => Some("python"),
        "ts" | "tsx" => Some("typescript"),
        "js" | "jsx" => Some("javascript"),
        "go" => Some("go"),
        "java" => Some("java"),
        _ => None,
    }
}

/// Get the appropriate compiler/type-checker command for a language.
fn compiler_command(language: &str, project_root: &Path) -> Option<(String, Vec<String>)> {
    match language {
        "rust" => Some(("cargo".into(), vec!["check".into()])),
        "typescript" | "javascript" => {
            let tsc = project_root.join("node_modules").join(".bin").join("tsc");
            if tsc.exists() {
                Some((tsc.to_str()?.into(), vec!["--noEmit".into()]))
            } else {
                Some(("npx".into(), vec!["tsc".into(), "--noEmit".into()]))
            }
        }
        "python" => Some((
            "python3".into(),
            vec!["-m".into(), "mypy".into(), ".".into()],
        )),
        "go" => Some(("go".into(), vec!["build".into(), "./...".into()])),
        _ => None,
    }
}

/// Get the appropriate linter command for a language.
fn linter_command(language: &str, project_root: &Path) -> Option<(String, Vec<String>)> {
    match language {
        "rust" => Some((
            "cargo".into(),
            vec!["clippy".into(), "--".into(), "-D".into(), "warnings".into()],
        )),
        "typescript" | "javascript" => {
            let eslint = project_root
                .join("node_modules")
                .join(".bin")
                .join("eslint");
            if eslint.exists() {
                Some((eslint.to_str()?.into(), vec![".".into()]))
            } else {
                Some(("npx".into(), vec!["eslint".into(), ".".into()]))
            }
        }
        _ => None,
    }
}

/// Get the test command for a language.
fn test_command(language: &str) -> Option<(String, Vec<String>)> {
    match language {
        "rust" => Some(("cargo".into(), vec!["test".into()])),
        "typescript" | "javascript" => Some(("npm".into(), vec!["test".into()])),
        "python" => Some(("python3".into(), vec!["-m".into(), "pytest".into()])),
        "go" => Some(("go".into(), vec!["test".into(), "./...".into()])),
        _ => None,
    }
}

/// Run a subprocess with a timeout.
async fn run_command(cmd: &str, args: &[String], cwd: &Path, timeout: Duration) -> (bool, String) {
    let run = Command::new(cmd).args(args).current_dir(cwd).output();

    match tokio::time::timeout(timeout, run).await {
        Ok(Ok(output)) => {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let combined = format!("{stdout}\n{stderr}");
            (output.status.success(), combined)
        }
        Ok(Err(e)) => (false, format!("failed to run {cmd}: {e}")),
        Err(_) => (
            false,
            format!("{cmd} timed out after {}s", timeout.as_secs()),
        ),
    }
}

/// Resolve `file_path` against `project_root`, rejecting anything that would
/// escape the project directory (absolute paths, `..` traversal, or symlinks
/// that resolve outside the root). `file_path` comes from client-supplied
/// `CodeContext` over the IPC channel and must never be trusted directly for
/// filesystem writes.
async fn resolve_target(
    project_root: &Path,
    file_path: &str,
) -> Result<std::path::PathBuf, String> {
    let requested = Path::new(file_path);
    if requested.is_absolute()
        || requested
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(format!(
            "rejected file path outside project root: {file_path}"
        ));
    }

    let canonical_root = tokio::fs::canonicalize(project_root)
        .await
        .map_err(|e| format!("failed to resolve project root: {e}"))?;

    let target = canonical_root.join(requested);

    // The target file may not exist yet (a brand-new file); canonicalize
    // whichever ancestor does exist and confirm it's still inside the root.
    let mut check = target.clone();
    let canonical_check = loop {
        match tokio::fs::canonicalize(&check).await {
            Ok(c) => break c,
            Err(_) => {
                let Some(parent) = check.parent() else {
                    return Err(format!("failed to resolve path ancestor for: {file_path}"));
                };
                check = parent.to_path_buf();
            }
        }
    };

    if !canonical_check.starts_with(&canonical_root) {
        return Err(format!(
            "rejected file path outside project root: {file_path}"
        ));
    }

    Ok(target)
}

/// Verify a patch by temporarily applying it, running checks, then restoring.
///
/// `patch_content` is the full file content after applying the patch.
/// `original_content` is the file content before the patch.
/// `file_path` is the path to the file being modified.
/// `project_root` is the root of the project.
pub async fn verify_patch(
    original_content: &str,
    patch_content: &str,
    file_path: &str,
    project_root: &Path,
    config: &VerifyConfig,
) -> VerificationResult {
    if !config.enabled {
        return VerificationResult {
            passed: true,
            compiler_output: None,
            test_output: None,
            lint_output: None,
            errors: vec![],
            retries_used: 0,
            max_retries: 0,
        };
    }

    let language = detect_language(file_path).unwrap_or("unknown");
    let timeout = Duration::from_secs(config.timeout_seconds);
    let mut result = VerificationResult {
        passed: true,
        compiler_output: None,
        test_output: None,
        lint_output: None,
        errors: vec![],
        retries_used: 0,
        max_retries: 0,
    };

    // `file_path` originates from client-supplied `CodeContext` over the IPC
    // channel, so it must be confined to `project_root` before we touch the
    // filesystem: reject absolute paths, `..` traversal, and symlink escapes
    // by canonicalizing and checking containment.
    let target = match resolve_target(project_root, file_path).await {
        Ok(t) => t,
        Err(e) => {
            result.passed = false;
            result.errors.push(e);
            return result;
        }
    };

    // Backup original to a dedicated temp file (not a predictable sibling
    // of the target) so verification never leaves a stray file next to it.
    let backup_suffix = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let backup = std::env::temp_dir().join(format!(
        "anvil-verify-{}-{backup_suffix}.bak",
        std::process::id()
    ));

    // Backup original
    if target.exists() {
        let _ = tokio::fs::copy(&target, &backup).await;
    }
    // Write patched content
    let _ = tokio::fs::write(&target, patch_content).await;

    // Run compiler/type-checker
    if let Some((cmd, args)) = compiler_command(language, project_root) {
        info!("running compiler: {cmd} {}", args.join(" "));
        let (passed, output) = run_command(&cmd, &args, project_root, timeout).await;
        result.compiler_output = Some(output.clone());
        if !passed {
            result.passed = false;
            result
                .errors
                .push(format!("compiler check failed:\n{output}"));
        }
    }

    // Run linter
    if config.run_linter && result.passed {
        if let Some((cmd, args)) = linter_command(language, project_root) {
            info!("running linter: {cmd} {}", args.join(" "));
            let (passed, output) = run_command(&cmd, &args, project_root, timeout).await;
            result.lint_output = Some(output.clone());
            if !passed {
                result.passed = false;
                result.errors.push(format!("lint check failed:\n{output}"));
            }
        }
    }

    // Run tests
    if config.run_tests && result.passed {
        if let Some((cmd, args)) = test_command(language) {
            info!("running tests: {cmd} {}", args.join(" "));
            let (passed, output) = run_command(&cmd, &args, project_root, timeout).await;
            result.test_output = Some(output.clone());
            if !passed {
                result.passed = false;
                result.errors.push(format!("tests failed:\n{output}"));
            }
        }
    }

    // Restore original content
    let _ = tokio::fs::write(&target, original_content).await;
    let _ = tokio::fs::remove_file(&backup).await;

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_language_rs() {
        assert_eq!(detect_language("src/main.rs"), Some("rust"));
    }

    #[test]
    fn detect_language_ts() {
        assert_eq!(detect_language("src/app.ts"), Some("typescript"));
    }

    #[test]
    fn detect_language_unknown() {
        assert_eq!(detect_language("README"), None);
    }

    #[test]
    fn default_config() {
        let cfg = VerifyConfig::default();
        assert!(cfg.enabled);
        assert!(!cfg.run_tests);
        assert!(cfg.run_linter);
    }

    #[tokio::test]
    async fn resolve_target_rejects_parent_traversal() {
        let root = std::env::temp_dir();
        let err = resolve_target(&root, "../../../etc/passwd")
            .await
            .unwrap_err();
        assert!(err.contains("outside project root"));
    }

    #[tokio::test]
    async fn resolve_target_rejects_absolute_path() {
        let root = std::env::temp_dir();
        #[cfg(unix)]
        let abs = "/etc/passwd";
        #[cfg(windows)]
        let abs = r"C:\Windows\System32\drivers\etc\hosts";
        let err = resolve_target(&root, abs).await.unwrap_err();
        assert!(err.contains("outside project root"));
    }

    #[tokio::test]
    async fn resolve_target_accepts_contained_new_file() {
        let root = tempdir_for_test();
        let resolved = resolve_target(&root, "src/new_file.rs").await.unwrap();
        assert!(resolved.starts_with(tokio::fs::canonicalize(&root).await.unwrap()));
    }

    fn tempdir_for_test() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "anvil-verify-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(dir.join("src")).unwrap();
        dir
    }
}
