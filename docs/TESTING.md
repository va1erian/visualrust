# Testing guide

## Layers

- **Unit tests** — next to the code they test.
- **Integration tests** — under `tests/`, using temp directories and fixtures.
- **UI tests** — launch a real window and verify it visually and by input.

## UI verification

Agents must verify UI work programmatically; a description of a rendering is
not evidence.

1. **Screenshot**: `vr-tooling` wraps win32ui's composited capture
   (`Window::capture_composited` / `capture::capture_hwnd`, behind win32ui's
   `wgc` feature) to capture a window by title/HWND/PID to a PNG. Compare with
   a checked-in golden and fail on a diff over the tolerance.
2. **Input**: `vr-tooling` can post keyboard/mouse input to a target window and
   wait for a condition, so a test can drive the IDE (`click`, `type_text`,
   `send_keys`, `wait_for`).
3. **Sandbox**: UI tests that take focus run inside Windows Sandbox via
   `scripts/sandbox/run.ps1`, so a live desktop is never disturbed.

Tests must be quiet and side-effect free: no audio, notifications, or writes to
real user data.

## Running checks

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

UI suite in the sandbox:

```
scripts/sandbox/run.ps1
```
