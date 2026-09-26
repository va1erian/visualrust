# VisualRust

A native Windows VB6/PureBasic-style **RAD IDE**: build a window with a form
designer, script the behaviour in **Dyon**, and export a self-contained `.exe`.
UI is native, on [`xui`](https://github.com/va1erian/xui); app code runs on the
[`dyon`](https://github.com/PistonDevelopers/dyon) runtime.

> Status: an early working prototype. A Dyon program can build a window with
> widgets, events and anchoring; forms generate to Dyon and run; Scintilla backs
> the editor. See [`docs/PLAN.md`](docs/PLAN.md) for the milestone status.

## Quick start

New here? Read the 10-minute [tutorial](docs/TUTORIAL.md), then:

```
cargo run -p vr-ide                                            # the IDE
cargo test -p vr-runtime --test hello_window -- --nocapture   # the sample + a PNG
```

Set `xui_DEMO_AUTOCLOSE_MS=3000` to make a window close itself (used by headless
tests and screenshots).

## What works today

- **Dyon UI runtime** — `ui_window`/`ui_free`/`ui_add`, widget constructors and
  setters, `ui_on` events to named Dyon handlers, and per-control anchors
  (`fill`, `stretch_horizontal`, …) over xui's `Layout::free`.
- **Forms** — a `.vrform` model, a designer surface (select/move/resize/palette/
  inspector), and `.vrform → Dyon` code generation.
- **Editor** — vendored Scintilla with incremental Dyon highlighting and a
  fixed-width font.
- **Web** — a message-based HTTP server that dispatches requests to Dyon
  handlers, with cookies/sessions/multipart/JSON/static files and SQLite.
- **Stdlib** — strings, math, date/time, files, encoding/regex/hash.
- **Packaging** — bundle format, self-locating loader, export, PE resource
  patching.

## Repository layout

```
crates/            the workspace (vr-core, vr-dyon, vr-forms, vr-web, vr-ide, …)
runtime/vr-runtime the packaged-app stub + self-loader
examples/          sample projects
docs/              PLAN.md, TUTORIAL.md, TESTING.md
scripts/           dispatch / land / sandbox helpers
```

## Development

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

House rules are in [`AGENTS.md`](AGENTS.md); UI changes attach light and dark
screenshots captured with `vr-tooling` (see [`docs/TESTING.md`](docs/TESTING.md)).

## Roadmap

Three demo apps are the near-term goal: a rich styled RSS client, a breakout
game, and a web chat backed by Fireworks AI. See [`docs/PLAN.md`](docs/PLAN.md)
and the `M9` milestone issues for the API gaps each one needs.
