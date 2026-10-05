// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Verification gate — runs compiler checks and linters on generated diffs
//! before applying them, providing a safety net against broken code.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tpt_anvil_indexer::syntax;
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

/// Detect toolchain-availability failures in a subprocess's output.
///
/// These are environment problems, not defects in the model's answer:
///
/// * `No module named mypy` / `No module named pytest` — Python package absent
/// * `This is not the tsc command you are looking for` — `typescript` is not
///   installed locally, so `npx tsc` cannot resolve a real compiler
/// * `program not found` — the launcher could not be spawned
/// * `is not recognized as an internal or external command`
///
/// Returns `true` when the output indicates a missing toolchain rather than a
/// genuine compile/lint/test failure.
pub fn is_toolchain_missing(output: &str) -> bool {
    const SIGNALS: &[&str] = &[
        "No module named",
        "is not the tsc command you are looking for",
        "program not found",
        "is not recognized as an internal or external command",
        "command not found",
        // A globally installed `tsc` (e.g. on CI runners) resolves via `npx`
        // but, with no tsconfig.json in the project, just prints its usage
        // banner and exits non-zero. No project-level compiler is available.
        "tsc: The TypeScript Compiler",
        // `npx` reaches the network when a package is not installed locally, so
        // it can hang until the subprocess timeout rather than printing a
        // recognisable error. That is still a missing toolchain, not a defect
        // in the model's answer.
        "timed out after",
    ];
    SIGNALS.iter().any(|s| output.contains(s))
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

/// Resolve a Node.js launcher (`npx` / `npm`) to something spawnable.
///
/// On Windows these are `.cmd` shims; `Command::new("npx")` fails with
/// "program not found" because no extensionless `npx` file exists. Return the
/// resolved path when we can find one.
fn node_launcher(name: &str) -> String {
    if !cfg!(windows) {
        return name.to_string();
    }
    // A Node install contains `npx`, `npx.cmd`, and `npx.ps1` side by side.
    // The extensionless `npx` is a shell script, not a PE binary, so
    // `Command::new` fails on it. `.cmd` is the real Windows launcher and is
    // what npm itself documents, so prefer it.
    for ext in ["cmd", "exe"] {
        if let Ok(path) = which(&format!("{name}.{ext}")) {
            return path.to_string_lossy().to_string();
        }
    }
    if let Ok(path) = which(name) {
        let candidate = path.to_string_lossy().to_string();
        if Path::new(&candidate).extension().is_some() {
            return candidate;
        }
    }
    name.to_string()
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
                Some((node_launcher("npx"), vec!["tsc".into(), "--noEmit".into()]))
            }
        }
        "python" => {
            let py = python_command()?;
            Some((py, vec!["-m".into(), "mypy".into(), ".".into()]))
        }
        "go" => Some(("go".into(), vec!["build".into(), "./...".into()])),
        _ => None,
    }
}

/// Resolve a working Python interpreter.
///
/// On Windows the `python3` name is frequently an App Execution Alias stub
/// (`%LOCALAPPDATA%\Microsoft\WindowsApps\python3.exe`) that exits with a
/// "Python was not found" store prompt instead of running. Prefer `python`,
/// which is the real interpreter on Windows, and fall back to `python3` on
/// Unix where `python` may not exist.
fn python_command() -> Option<String> {
    for candidate in ["python", "python3"] {
        if command_exists(candidate) {
            return Some(candidate.to_string());
        }
    }
    None
}

/// Whether `name` resolves to an executable that is not the WindowsApps alias
/// stub (which exists on disk but cannot actually run).
fn command_exists(name: &str) -> bool {
    let Ok(path) = which(name) else {
        return false;
    };
    let path_str = path.to_string_lossy();
    if !cfg!(windows) {
        return true;
    }
    // `...\WindowsApps\python.exe` / `python3.exe` are the Microsoft Store
    // aliases, not a real interpreter.
    !path_str.contains("WindowsApps")
}

/// Minimal `which`: walk `PATH` looking for an executable named `name`.
fn which(name: &str) -> std::result::Result<std::path::PathBuf, ()> {
    let path_var = std::env::var_os("PATH").ok_or(())?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Ok(candidate);
        }
        // Windows resolves `.exe`/`.cmd`/`.bat` transparently.
        for ext in ["exe", "cmd", "bat"] {
            let with_ext = dir.join(format!("{name}.{ext}"));
            if with_ext.is_file() {
                return Ok(with_ext);
            }
        }
    }
    Err(())
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
                Some((node_launcher("npx"), vec!["eslint".into(), ".".into()]))
            }
        }
        _ => None,
    }
}

/// Get the test command for a language.
fn test_command(language: &str) -> Option<(String, Vec<String>)> {
    match language {
        "rust" => Some(("cargo".into(), vec!["test".into()])),
        "typescript" | "javascript" => Some((node_launcher("npm"), vec!["test".into()])),
        "python" => {
            let py = python_command()?;
            Some((py, vec!["-m".into(), "pytest".into()]))
        }
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

/// Syntax-check `patch_content` with tree-sitter, for use when no real
/// toolchain is available.
///
/// Returns `Some(true)`/`Some(false)` when the language has a bundled
/// grammar, and `None` when it cannot be checked at all (caller should then
/// treat the task as skipped). This is strictly weaker than a compiler: it
/// catches truncated or malformed output but not type errors.
fn syntax_check_fallback(patch_content: &str, language: &str) -> Option<bool> {
    let errors = syntax::syntax_errors(patch_content, language)?;
    Some(errors.is_empty())
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
            // A missing toolchain is an environment problem, not a defect in
            // the model's answer. Degrade to a tree-sitter syntax check rather
            // than failing the task or excluding it from the score: `tsc` and
            // `mypy` are frequently absent (a throwaway sandbox has no
            // `node_modules`, no virtualenv), and silently shrinking the
            // denominator makes scores non-comparable across machines.
            if is_toolchain_missing(&output) {
                match syntax_check_fallback(patch_content, language) {
                    Some(true) => {
                        // Clean syntax. Log the degradation rather than pushing
                        // an `errors` entry: a passing result with a non-empty
                        // error list is misleading to every consumer.
                        info!(
                            "{language}: compiler unavailable, fell back to tree-sitter \
                             syntax check (passed)"
                        );
                    }
                    Some(false) => {
                        let report = syntax::format_errors(
                            &syntax::syntax_errors(patch_content, language).unwrap_or_default(),
                            file_path,
                        );
                        result.passed = false;
                        result.errors.push(format!(
                            "compiler unavailable; syntax check failed:\n{report}"
                        ));
                    }
                    None => {
                        // No grammar either — genuinely unverifiable.
                        result.passed = false;
                        result
                            .errors
                            .push(format!("compiler check failed:\n{output}"));
                    }
                }
            } else {
                result.passed = false;
                result
                    .errors
                    .push(format!("compiler check failed:\n{output}"));
            }
        }
    }

    // Run linter
    if config.run_linter && result.passed {
        if let Some((cmd, args)) = linter_command(language, project_root) {
            info!("running linter: {cmd} {}", args.join(" "));
            let (passed, output) = run_command(&cmd, &args, project_root, timeout).await;
            result.lint_output = Some(output.clone());
            if !passed {
                // Same reasoning as the compiler above: `npx eslint` without a
                // local `node_modules` is an environment problem, not a model
                // defect, so it must not fail an otherwise-clean result.
                if is_toolchain_missing(&output) {
                    info!("{language}: linter unavailable, skipping lint gate");
                } else {
                    result.passed = false;
                    result.errors.push(format!("lint check failed:\n{output}"));
                }
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

    #[test]
    fn syntax_fallback_accepts_valid_typescript() {
        let good = "export function add(a: number, b: number): number {\n  return a + b;\n}\n";
        assert_eq!(syntax_check_fallback(good, "typescript"), Some(true));
    }

    #[test]
    fn syntax_fallback_rejects_truncated_typescript() {
        // The failure mode this fallback exists to catch: a model response
        // truncated mid-function.
        let truncated = "export function add(a: number, b: number): number {\n  return a + b;\n";
        assert_eq!(syntax_check_fallback(truncated, "typescript"), Some(false));
    }

    #[test]
    fn syntax_fallback_accepts_valid_python() {
        assert_eq!(
            syntax_check_fallback("def add(a, b):\n    return a + b\n", "python"),
            Some(true)
        );
    }

    #[test]
    fn syntax_fallback_accepts_balanced_but_semantically_wrong_python() {
        // Proves the limitation honestly: tree-sitter cannot see that `+` is
        // the wrong operator, only that the source is malformed.
        let wrong_but_balanced = "def add(a, b):\n    return a - b\n";
        assert_eq!(
            syntax_check_fallback(wrong_but_balanced, "python"),
            Some(true)
        );
    }

    #[test]
    fn syntax_fallback_returns_none_for_unknown_language() {
        assert_eq!(syntax_check_fallback("DISPLAY ...", "cobol"), None);
    }

    #[tokio::test]
    async fn missing_typescript_toolchain_falls_back_instead_of_failing() {
        // End-to-end for the regression this fixes: `npx tsc` cannot resolve a
        // real compiler in a sandbox, which used to fail every TS task.
        let root = tempdir_for_test();
        let original = "export function add(a: number, b: number): number {\n  return a + b;\n}\n";
        tokio::fs::write(root.join("utils.ts"), original)
            .await
            .unwrap();

        let result = verify_patch(
            original,
            // Valid, complete TypeScript: the fallback must let this through.
            "export function add(a: number, b: number): number {\n  return a + b;\n}\n",
            "utils.ts",
            &root,
            &VerifyConfig {
                enabled: true,
                run_linter: false,
                ..VerifyConfig::default()
            },
        )
        .await;

        let compiler = result.compiler_output.as_deref().unwrap_or("");
        if is_toolchain_missing(compiler) {
            // Degraded to a syntax check, which must pass this valid content
            // rather than recording a failure.
            assert!(
                result.passed,
                "valid TS must pass via syntax fallback; errors: {:?}",
                result.errors
            );
            assert!(
                result.errors.is_empty(),
                "a passing result must not carry errors: {:?}",
                result.errors
            );
        } else {
            // A real `tsc` was available; a genuine typecheck also passes.
            assert!(result.passed, "errors: {:?}", result.errors);
        }
    }

    #[tokio::test]
    async fn truncated_typescript_fails_even_without_a_toolchain() {
        // The complement of the test above: when no compiler exists, malformed
        // output must still be rejected rather than waved through.
        let root = tempdir_for_test();
        let original = "export function add(a: number, b: number): number {\n  return a + b;\n}\n";
        tokio::fs::write(root.join("utils.ts"), original)
            .await
            .unwrap();

        let result = verify_patch(
            original,
            "export function add(a: number, b: number): number {\n  return a + b;\n",
            "utils.ts",
            &root,
            &VerifyConfig {
                enabled: true,
                run_linter: false,
                ..VerifyConfig::default()
            },
        )
        .await;

        if is_toolchain_missing(result.compiler_output.as_deref().unwrap_or("")) {
            assert!(
                !result.passed,
                "truncated output must fail the syntax fallback"
            );
        }
        let _ = tokio::fs::remove_dir_all(&root).await;
    }

    #[tokio::test]
    async fn verify_restores_original_content_after_fallback() {
        let root = tempdir_for_test();
        let original = "export function add(a: number, b: number): number {\n  return a + b;\n}\n";
        tokio::fs::write(root.join("utils.ts"), original)
            .await
            .unwrap();

        let _ = verify_patch(
            original,
            "export function broken( {\n",
            "utils.ts",
            &root,
            &VerifyConfig {
                enabled: true,
                run_linter: false,
                ..VerifyConfig::default()
            },
        )
        .await;

        let after = tokio::fs::read_to_string(root.join("utils.ts"))
            .await
            .unwrap();
        assert_eq!(after, original, "verification must not leave edits behind");
        let _ = tokio::fs::remove_dir_all(&root).await;
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

    #[test]
    fn which_finds_cargo() {
        // cargo is required for the rust verification path, so it must resolve.
        assert!(which("cargo").is_ok(), "cargo should be on PATH");
    }

    #[test]
    fn which_rejects_nonexistent_command() {
        assert!(which("definitely-not-a-real-command-xyz").is_err());
    }

    #[test]
    fn python_command_never_returns_windows_store_stub() {
        if let Some(py) = python_command() {
            assert!(
                !command_exists("nonexistent"),
                "sanity: nonexistent command must not exist"
            );
            if cfg!(windows) {
                let resolved = which(&py).expect("resolved python");
                assert!(
                    !resolved.to_string_lossy().contains("WindowsApps"),
                    "python_command() returned the Store alias stub: {}",
                    resolved.display()
                );
            }
        }
    }

    #[test]
    fn compiler_command_python_uses_resolved_interpreter() {
        if let Some((cmd, args)) = compiler_command("python", Path::new(".")) {
            assert!(cmd == "python" || cmd == "python3");
            assert_eq!(
                args,
                vec!["-m".to_string(), "mypy".to_string(), ".".to_string()]
            );
        }
    }

    #[test]
    fn test_command_python_uses_resolved_interpreter() {
        if let Some((cmd, args)) = test_command("python") {
            assert!(cmd == "python" || cmd == "python3");
            assert_eq!(args, vec!["-m".to_string(), "pytest".to_string()]);
        }
    }

    #[test]
    fn compiler_command_rust_unchanged() {
        let (cmd, args) = compiler_command("rust", Path::new(".")).expect("rust command");
        assert_eq!(cmd, "cargo");
        assert_eq!(args, vec!["check".to_string()]);
    }

    #[test]
    fn compiler_command_unknown_is_none() {
        assert!(compiler_command("cobol", Path::new(".")).is_none());
    }

    #[test]
    fn command_exists_is_false_for_absent_binary() {
        assert!(!command_exists("definitely-not-a-real-command-xyz"));
    }

    #[test]
    fn node_launcher_resolves_to_spawnable_path() {
        if cfg!(windows) {
            let npx = node_launcher("npx");
            if which("npx.cmd").is_ok() {
                assert!(
                    npx.to_ascii_lowercase().ends_with(".cmd"),
                    "npx launcher should resolve to npx.cmd, got {npx}"
                );
                assert!(
                    std::path::Path::new(&npx).exists(),
                    "resolved npx launcher must exist on disk: {npx}"
                );
            }
        } else {
            assert_eq!(node_launcher("npx"), "npx");
        }
    }

    #[test]
    fn is_toolchain_missing_detects_missing_mypy() {
        assert!(is_toolchain_missing(
            "C:\\Python313\\python.exe: No module named mypy"
        ));
    }

    #[test]
    fn is_toolchain_missing_detects_missing_typescript() {
        let out = "This is not the tsc command you are looking for\n\
                   Use npm install typescript to first add TypeScript";
        assert!(is_toolchain_missing(out));
    }

    #[test]
    fn is_toolchain_missing_detects_subprocess_timeout() {
        // `npx` hangs trying to reach the network when a package is absent, so
        // the timeout is how a missing TypeScript/ESLint toolchain surfaces.
        assert!(is_toolchain_missing(
            "C:\\Program Files\\nodejs\\npx.cmd timed out after 30s"
        ));
    }

    #[test]
    fn is_toolchain_missing_detects_spawn_failure() {
        assert!(is_toolchain_missing("failed to run npx: program not found"));
        assert!(is_toolchain_missing(
            "'npx' is not recognized as an internal or external command"
        ));
    }

    #[test]
    fn is_toolchain_missing_ignores_real_compile_errors() {
        // A genuine compile failure must NOT be treated as an environment gap,
        // otherwise real model errors would be silently excluded from scoring.
        let out = "error[E0601]: `main` function not found in crate `main`\n  \
                   error: could not compile `anvil-bench-scaffold` (bin \"main\")";
        assert!(!is_toolchain_missing(out));
    }

    #[test]
    fn is_toolchain_missing_ignores_type_errors() {
        let out = "error[E0308]: mismatched types\n  = note: expected `i32`";
        assert!(!is_toolchain_missing(out));
    }

    #[tokio::test]
    async fn resolved_npx_launcher_actually_runs() {
        let cmd = node_launcher("npx");
        let (passed, _output) = run_command(
            &cmd,
            &["--version".to_string()],
            std::env::temp_dir().as_path(),
            Duration::from_secs(60),
        )
        .await;
        assert!(passed, "`{cmd} --version` should succeed");
    }
}
