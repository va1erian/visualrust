# VisualRust — plan

A native Windows VB6/PureBasic-style RAD IDE written in Rust on `xui`,
where apps are scripted in **Dyon** and exported to self-contained `.exe`
files.

New to the project? Start with [the quick tutorial](TUTORIAL.md).

## Status (as of 2026-09-26)

48 PRs merged; `main` is green and protected (PR + `check` + linear history).
A working vertical slice exists: build a UI from Dyon, generate a form, run it
with events and anchoring.

- **M0 Foundation — done.** Workspace/CI, `vrproj.toml` manifest, project model,
  Dyon runtime, hello-window, and the capture/sandbox tooling.
- **M1 IDE shell — partial.** Explorer, Scintilla editor pane, form designer
  pane, property inspector and output pane in one window (the prototype).
  Docking polish and run/stop remain.
- **M2 Editor — mostly done.** Vendored Scintilla + safe wrapper + xui host,
  incremental Dyon highlighting, fixed-width font. Completion, diagnostics and
  find/replace remain.
- **M3 Forms — partial.** `.vrform` model, painted designer surface
  (select/move/resize/palette/inspector), anchors, and `.vrform → Dyon` codegen.
  Design/run, the events tab and a code-behind merge remain.
- **M4 Stdlib v1 — partial.** Strings, math, date/time, files/console,
  encoding/regex/hash. Dialogs and the JSON/XML/HTTP modules (#131–#134) remain.
- **M5 Packaging — mostly done.** Bundle format, self-locating loader, export
  flow, PE resource patching, packaged-exe verification. Wiring icon/version
  into export remains.
- **M6 Data — partial.** SQLite bindings + transactions/prepared statements/
  blobs. The database browser remains.
- **M7 Web/API — core done.** Message-based HTTP server, Dyon dispatch bridge,
  cookies/sessions/multipart/JSON/static files, SQLite-from-handlers, endpoint
  model and an end-to-end sample. The API editor UI remains.
- **M8 Polish — started.** Tutorial, branch protection; installer and the
  upstream contributions below remain.
- **M9 Demos & API gaps — roadmapped.** See "Demos" below.

## Locked decisions

- **Export**: a prebuilt `vr-runtime.exe` stub plus an appended compressed
  bundle (footer magic + offset at EOF); the runtime reads its own module,
  extracts and runs. Icon/version patched via `UpdateResource`. No Rust
  toolchain on the end-user machine. An optional "build as a Rust project" mode
  supports native extension crates.
- **Runtime UI**: Dyon builds widgets through `ui_*` functions. `ui_run`
  **registers and returns**; the runtime then hosts the message loop with the
  Dyon runtime moved into the host `App`, so a handler runs off-stack and can
  call setters but never re-enter Dyon.
- **Anchoring**: xui owns the anchor engine and `Layout::free` (`va1erian/xui`
  #32); `vr-forms` maps the persisted `.vrform` anchors onto it.
- **Form designer**: a painted `xui` `Custom` surface that draws the form, its
  control proxies and selection adorners, and handles input; `.vrform` is the
  source of truth and codegen emits Dyon `ui_*` that the runtime implements.
- **Editor**: Scintilla 5.5.x vendored in `vr-scintilla-sys`, a safe wrapper +
  xui host widget, container lexer (`SCI_SETILEXER(NULL)` + `SCN_STYLENEEDED`)
  fed by `vr-syntax`, fixed-width font.
- **Language**: Dyon for app code, with optional hand-written Rust extension
  DLLs listed in the manifest.
- **Rich text**: a lightweight **litehtml** component in xui (`va1erian/xui`
  #49) will render HTML/CSS-subset content; until then demos use basic native
  styling (themes, fonts, `FlowText` runs, images).

## Execution model

The packaged app is a `xui::App`. Dyon `main` builds a plan (`ui_window`,
widget constructors, `ui_add`, `ui_on`, `ui_anchor`) and calls `ui_run`, which
records it and returns. After `main`, the runtime takes the plan and starts the
message loop with an `App` that owns the Dyon runtime; each event calls the
named handler in `App::update`, which is never re-entered. Web/DB work runs
off-thread and talks back through `Proxy<Msg>` / Dyon `in` types.

## Crates

```
crates/
  vr-core/          project model, vrproj.toml, settings
  vr-dyon/          Dyon runtime, native registration, ui_* bindings
  vr-syntax/        Dyon lexer, highlighting, completion model
  vr-scintilla-sys/ vendored Scintilla build + raw FFI
  vr-scintilla/     safe wrapper + xui host widget
  vr-editor/        editor <-> document glue
  vr-forms/         .vrform model, designer, anchor engine, codegen
  vr-std/           PureBasic-inspired standard library
  vr-db/            SQLite exposed to Dyon
  vr-web/           message-based HTTP framework exposed to Dyon
  vr-api/           endpoint model + scaffolding
  vr-pack/          bundle format, export, PE resource patching
  vr-tooling/       capture, sandbox, automation test support
  vr-ide/           the IDE application
runtime/
  vr-runtime/       packaged app stub (xui App host + self-loader)
```

## Milestones

- **M0 Foundation** — done.
- **M1 IDE shell** — panes, explorer, editor/designer modes (run/stop remain).
- **M2 Editor** — Scintilla, highlighting (completion/diagnostics remain).
- **M3 Forms** — designer, palette, properties, codegen (design/run remains).
- **M4 Stdlib v1** — strings/math/date/files/encoding (dialogs, HTTP, JSON,
  XML, images remain).
- **M5 Packaging** — bundle + export + resources (icon wiring remains).
- **M6 Data** — SQLite (+ transactions); browser remains.
- **M7 Web/API** — HTTP framework + dispatch + sample; API editor UI remains.
- **M8 Polish** — docs, installer, upstream contributions.
- **M9 Demos & API gaps** — the three demo apps and the gaps they need.

The issue tracker is the source of truth for individual tasks; see the
milestones and `area:*` labels.

## Demos (M9)

The goal is three complete apps; the gaps are filed as issues:

1. **Rich styled RSS client** — HTTP(+TLS), XML/RSS, image loading, widget
   bindings; basic native styling now, litehtml later.
2. **Breakout game** — a canvas/game surface, animation/timers and input.
3. **Web chat backed by Fireworks AI** — HTTP client, SSE streaming, JSON, env,
   and the message-based `vr-web` server.

## Upstream work in `va1erian/xui`

- **Anchor engine + `Layout::free`** — merged (#32).
- **litehtml rich-HTML component** — proposed (#49).
- **File dialogs** (`FileDialog`), a first-class **`ForeignWidget`** and
  `WM_NOTIFY`/`WM_COMMAND` routing to owning widgets — proposed.
