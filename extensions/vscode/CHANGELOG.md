# Changelog — TPT Anvil VS Code extension

All notable changes to the TPT Anvil VS Code extension are documented here.
Daemon, workspace, and cross-cutting changes are in the [root changelog](../../CHANGELOG.md).

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project uses [Semantic Versioning](https://semver.org/).

---

## [Unreleased]

### Added
- Chat panel (`AnvilChatViewProvider`, a `WebviewViewProvider`) with Markdown rendering,
  slash-command autocomplete, and streaming responses.
- Slash-command handlers and context-menu actions for `/generate`, `/test`, `/explain`,
  `/fix`, and `/docs`.
- Diff viewer with a three-button dialog (Apply / Preview / Dismiss) and side-by-side diff.
- `anvil.status` command and status bar indicator for the active model, backend, and
  connection state.
- Daemon IPC client over the Unix socket or Windows named pipe.

### Changed
- Build and test toolchain upgraded: TypeScript 7.0.2, Vitest 5, `@vitest/coverage-v8` 5,
  ESLint 10, Mocha 12, `@vscode/vsce` 4, esbuild 0.28, and glob 14.

### Fixed
- `tsconfig` files declare the `node` types (and `mocha` for the E2E suite) explicitly, so
  type resolution works under TypeScript 7.

[Unreleased]: https://github.com/tpt-solutions/tpt-anvil/commits/master/extensions/vscode
