# VisualRust — plan

A native Windows VB6/PureBasic-style RAD IDE written in Rust on `win32ui`,
where apps are scripted in **Dyon** and exported to self-contained `.exe`
files.

## Locked decisions

- **Export**: a prebuilt `vr-runtime.exe` stub plus an appended compressed
  bundle (footer magic + offset at EOF); the runtime reads its own module,
  extracts and runs. Icon/version patched via `UpdateResource`. No Rust
  toolchain on the end-user machine. An optional "build as a Rust project"
  mode supports native extension crates.
- **Form designer**: hosts live `win32ui` controls in a `Panel` with an overlay
  for selection/drag/grid. The `.vrform` file is the source of truth; codegen
  emits Dyon that calls native `ui_*` builders. Code-behind is separate.
- **Language**: Dyon only for app code, with optional hand-written Rust
  extension DLLs listed in the manifest.
- **Editor**: Scintilla 5.5.x vendored in `vr-scintilla-sys`, a safe wrapper +
  win32ui host widget in `vr-scintilla`. Dyon highlighting uses the container
  lexer (`SCI_SETILEXER(NULL)` + `SCN_STYLENEEDED`) fed by `vr-syntax`.
  A `ForeignWidget` capability is offered upstream to win32ui.
- **First slice**: M0–M3 (foundation, IDE shell, editor, forms designer).

## Execution model

The packaged app is a `win32ui::App`. Dyon `main` creates widgets through
native `ui_*` functions, installs a layout, registers handler names, then calls
native `ui_run()` which blocks in the win32ui loop. Each `App::update(msg)`
dispatches to the mapped Dyon function; widget handles live in the runtime as
Dyon custom objects. `App::update` is never re-entered — Dyon calls are
sequential. Web/DB work runs off-thread and talks back through `Proxy<Msg>` /
Dyon `in` types.

## Crates

```
crates/
  vr-core/          project model, vrproj.toml, settings
  vr-dyon/          Dyon runtime, native registration, ui_* bindings
  vr-syntax/        Dyon lexer, highlighting, completion model
  vr-scintilla-sys/ vendored Scintilla build + raw FFI
  vr-scintilla/     safe wrapper + win32ui host widget
  vr-editor/        editor <-> document glue
  vr-forms/         .vrform model, designer, codegen
  vr-std/           PureBasic-inspired standard library
  vr-db/            SQLite exposed to Dyon
  vr-web/           HTTP framework exposed to Dyon
  vr-api/           endpoint model + scaffolding
  vr-tooling/       capture, sandbox, automation test support
  vr-ide/           the IDE application
runtime/
  vr-runtime/       packaged app stub (win32ui App host)
```

## Milestones

- **M0 Foundation** — workspace/CI, manifest, project model, Dyon runtime,
  hello-window, and the tooling (capture/sandbox/automation) that lets agents
  verify the UI programmatically.
- **M1 IDE shell** — main window, panes, project explorer, dialogs, run/stop.
- **M2 Editor** — Scintilla integration, Dyon highlighting, completion,
  diagnostics.
- **M3 Forms** — designer, palette, properties, codegen, design/run.
- **M4 Stdlib v1** — dialogs, strings, math, date, files, encoding.
- **M5 Packaging** — bundle + export.
- **M6 Data** — SQLite + browser.
- **M7 Web/API** — HTTP framework + API editor.
- **M8 Polish** — docs, installer, upstream contributions.

The issue tracker is the source of truth for individual tasks; see the
milestones and `area:*` labels.

## Upstream work in win32ui

Tracked in `va1erian/win32ui`: file dialogs (`FileDialog`), a first-class
`ForeignWidget` for hosting foreign HWNDs, and routing `WM_NOTIFY`/`WM_COMMAND`
from hosted children to their owning widget.
