# Vendored Scintilla

`vendor/scintilla` is a pinned checkout of the Scintilla C++ source. The build
compiles it with `cc`; it never fetches anything, so no git submodules or
network access are needed at build time.

- **Tag:** `rel-5-5-6`
- **Upstream commit:** `d3277558d108402908ba1c474a2f44c30559a645`
- **Source:** <https://github.com/craftward/scintilla> (daily Mercurial mirror of
  the canonical <https://www.scintilla.org/> repository). The tag is identical
  to Scintilla's own release tag; it is taken from a mirror because the project
  publishes releases as Mercurial tags and only mirrors expose them to git.

## What is kept

Only the files the Windows static build needs:

```
vendor/scintilla/
  include/   public headers
  src/       portable C++ sources
  win32/     Windows platform backend
  License.txt
  README
  VERSION    this pin
```

`gtk/`, `cocoa/`, `qt/`, `doc/`, `test/`, and the Lexilla-related trees are
removed: VisualRust uses Scintilla's container lexer (`SCI_SETILEXER(NULL)` +
`SCN_STYLENEEDED`) even before it ships a lexer, not Lexilla.

## Updating

1. Pick the new release tag `rel-5-<minor>-<patch>` from the Scintilla project.
2. Clone it into a scratch directory (never into this tree):
   `git clone --depth 1 --branch rel-5-5-6 https://github.com/craftward/scintilla`
3. Remove `.git` and everything except `include/`, `src/`, `win32/`,
   `License.txt`, and `README`; copy the result over `vendor/scintilla`.
4. Write the new tag into `vendor/scintilla/VERSION`.
5. Check `win32/makefile`'s `COMPONENT_OBJS` against `SRC_SOURCES` /
   `WIN32_SOURCES` in `build.rs` and add or drop translation units as needed.
6. Re-read `include/Scintilla.h` for any changed `SCI_*` / `SCN_*` values.

## Licence

Scintilla is distributed under the Historical Permission Notice and Disclaimer
(HPND). The full notice is in `LICENSE-SCINTILLA` and
`vendor/scintilla/License.txt`, and is referenced from `src/lib.rs`.
