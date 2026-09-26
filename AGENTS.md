# AGENTS.md — VisualRust house rules

VisualRust is a native Windows RAD IDE: built in Rust on
[`xui`](https://github.com/va1erian/xui), apps are scripted in
[Dyon](https://github.com/PistonDevelopers/dyon), and projects export to
self-contained `.exe` files. Read `docs/PLAN.md` for the architecture and the
milestone map. New to the project? `docs/TUTORIAL.md` is a quick Dyon +
VisualRust crash course.

## Environment

- Windows only, `x86_64-pc-windows-msvc`, edition 2024.
- A C++ toolchain (MSVC Build Tools) is required to build the vendored
  Scintilla sources in `vr-scintilla-sys`.
- `xui` is pre-1.0: it is pinned by rev in the workspace `Cargo.toml`.
  Never change the pin to a branch, and never add a second version.

## Pre-submit checks

Run all three, in this order, from the repo root; a PR is not ready until they
are green:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

If your change touches the UI, also capture a screenshot (below) and attach it
to the PR for both light and dark themes.

## Code rules

- **Files stay under 500 lines.** Split by responsibility; the limit is a hard
  signal to look for a seam, not a target to squeeze under.
- **Comments explain why, not what.** No banner comments, no restating the
  signature, no commented-out code. Document non-obvious invariants and unsafe
  preconditions.
- **`unsafe` lives only in a `sys` module** (or a `*-sys` crate) and every
  block carries a `// SAFETY:` note. Everything else forbids it.
- **No `unwrap`/`expect`/panics outside tests.** Return `Result`; define typed
  errors with `thiserror`. Public APIs do not expose raw `windows` types.
- **No `TODO` without a linked issue number.**
- **Dependencies are deliberate.** Prefer the workspace dependency; pin by
  version, and add a one-line justification when introducing one.

## Tests

- Unit tests live next to the code; integration tests in `tests/`.
- Tests must be headless, quiet and side-effect free: no audio, no
  notifications, no writes to real user data. Use temp directories and
  fixtures.
- UI tests that grab focus or input run under the Windows Sandbox runner
  (`scripts/sandbox/run.ps1`) — see `docs/TESTING.md`.
- Verify UI work programmatically before claiming it works: launch the app,
  capture a PNG with `vr-tooling` (built on xui's composited capture), and
  look at it. An agent's opinion of its own rendering is not evidence.

## Git and PRs

- One issue per PR. Branch from the latest `main`: `feat/<issue>-<slug>`.
- Rebase, never merge, to sync with `main`.
- Stage only the files the issue owns; keep unrelated changes out.
- PR body: link the issue (`Closes #n`), summarise, and list the checks you ran
  and their result. Attach screenshots for UI changes.
- Never force-push a branch that moved underneath you.
- One landing at a time; delete a worktree once its PR merges.

## Worktrees

Parallel agents each get their own worktree **and their own build directory**.
Never share `target/` between worktrees: concurrent builds collide and produce
phantom failures. Use `scripts/dispatch.ps1` / `scripts/land.ps1`.

## Tooling

- `scripts/dispatch.ps1` — create a worktree + branch for an issue.
- `scripts/land.ps1` — rebase, check, push, wait for CI, rebase-merge, clean up.
- `scripts/sandbox/run.ps1` — run UI tests in Windows Sandbox.

The orchestrator is OpenCode. Do not invoke the Claude-oriented dispatch
scripts from an OpenCode session; use the Task subagent tool instead.
