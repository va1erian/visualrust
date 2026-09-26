# VisualRust quick tutorial

VisualRust is a native Windows RAD toolkit: you build a window in **Dyon** (a
small, dynamically typed scripting language), running on **xui** (a native Win32
widget toolkit). The IDE edits forms, generates the Dyon, and (later) exports a
self-contained `.exe`. This tutorial gets you from zero to a running window and
shows how forms become Dyon.

Nothing here needs the IDE: a `.dyon` file plus the runtime is enough.

---

## 1. Dyon in ten minutes

Dyon reads like a cross between Rust and a spreadsheet formula. It is
**dynamically typed** and **expression-oriented**. Comments are `//` and
`/* ... */`.

### Values and variables

```dyon
name := "VisualRust"      // := declares (infers the type)
count := 42               // f64 under the hood — Dyon has one number type
flag := true              // bool
items := [1, 2, 3]        // array
person := {name: "Ada", age: 36}   // object

count = 43                // = assigns to an existing variable
person.age = 37           // assign to an object field
person.city := "London"   // := adds a new key
```

Lookup is zero-based: `items[0]` is `1`. Multidimensional: `grid[[0, 1]]`.

### Control flow

```dyon
if count > 40 { println("big") } else { println("small") }

for i 10 { print(i) }              // 0..10
for i [2, 6) { print(i) }          // 2..6
loop { break }                     // infinite loop; break/continue work

x := if flag { 1 } else { 0 }      // if is an expression
```

### Functions

```dyon
fn greet(name) {                   // no return value
    println("Hello, " + name)
}

fn double(x) -> { return x * 2 }   // returns a value
fn inc(x) = x + 1                  // math form
```

Calling: `greet("Ada")`, `d := double(21)`.

### Option, result and `?`

Absence and failure are values, not exceptions:

```dyon
found := some(3)      // or none()
okay := ok("fine")    // or err("boom")

value := unwrap(found)   // prints a trace if it is none()

fn load(x) -> {
    v := maybe()?        // `?` unwraps ok/some, propagates err/none
    return v
}
```

### Interoperating with Rust

Native functions are just functions to Dyon. The stdlib gives you strings,
math, dates, files, and more — for example `str_len("abc")`, `num_str(42)`,
`str_trim("  x ")`, `println`, `read_line`. Names that would clash with Dyon's
own built-ins are prefixed (`str_`, `num_`).

---

## 2. Your first window

A window is built with a small `ui_*` vocabulary, then `ui_run` hands it to the
message loop:

```dyon
fn main() {
    win  := ui_window("Hello", 480, 320)
    root := ui_free(480, 320)                       // the design surface

    greeting := ui_label("Hello, window!", 12, 12, 320, 28)
    ui_add(root, greeting)
    ui_anchor(greeting, "top_left")

    ui_run(win, root)
}
```

- `ui_window(title, w, h)` — the frame, sized in design units.
- `ui_free(w, h)` — an absolute ("free") layout; every child is placed at its
  own design coordinates and follows the parent by its **anchor**.
- Widget constructors take `(text, x, y, w, h)`. `ui_add(root, child)` places it.
- `ui_run(win, root)` registers the window and **returns**; the runtime starts
  the loop afterwards. (This is why a handler can never re-enter Dyon — see §5.)

### Widgets

`ui_label`, `ui_button`, `ui_edit`, `ui_check_box`, `ui_radio_group`,
`ui_combo_box`, `ui_group_box`, `ui_panel`, `ui_scroll_view`, `ui_list_view`,
`ui_tree_view`, `ui_tabs`, `ui_menu`, `ui_status_bar`, `ui_toolbar`,
`ui_progress_bar`, `ui_slider`, `ui_grid_view`, `ui_color_picker`,
`ui_flow_text`.

Some families have no native child control yet and are rendered as a labelled
placeholder (radio group, combo box, list/tree view, tabs, menu, toolbar, grid
view) so a form always opens; they still lay out and anchor.

### Properties (setters)

After construction you can change a widget:

```dyon
ui_set_text(button, "Save")
ui_set_enabled(button, false)
ui_set_visible(label, false)
ui_set_tooltip(edit, "Your name")
ui_set_checked(box, true)
ui_set_items(combo, ["A", "B"])
ui_set_selected(combo, "A")
ui_set_range(slider, 0, 10)
ui_set_value(slider, 5)
ui_set_columns(list, ["Name", "Size"])
```

Called from a handler, these mutate the **live** widget and repaint. Called in
`main` before `ui_run`, they edit the plan (and take effect when the window
opens).

---

## 3. Anchors: resizable, DPI-independent layout

Placement is in **design units**, so it is DPI-independent. When the window
resizes, each widget moves by its anchor:

| anchor | effect |
|---|---|
| `top_left` | never moves (default) |
| `top` / `bottom` | centred horizontally, pinned top / bottom |
| `left` / `right` | centred vertically, pinned left / right |
| `center` | keeps its offset from the centre |
| `top_right`, `bottom_left`, `bottom_right` | pinned corner |
| `stretch_horizontal` | left edge pinned, width grows with the parent |
| `stretch_vertical` | top edge pinned, height grows |
| `fill` | all edges pinned: grows both ways |

```dyon
toolbar_bg := ui_panel("", 0, 0, 480, 40)
ui_add(root, toolbar_bg)
ui_anchor(toolbar_bg, "stretch_horizontal")

body := ui_flow_text("", 12, 48, 456, 260)
ui_add(root, body)
ui_anchor(body, "fill")
```

---

## 4. Events

Bind a widget event to a **named Dyon function**:

```dyon
fn main() {
    win := ui_window("Counter", 320, 160)
    root := ui_free(320, 160)

    label := ui_label("0", 12, 12, 120, 28)
    ui_add(root, label)
    ui_anchor(label, "top_left")

    plus := ui_button("+1", 12, 52, 80, 32)
    ui_add(root, plus)
    ui_anchor(plus, "top_left")
    ui_on(plus, "click", "on_click_plus")

    ui_run(win, root)
}

fn on_click_plus() {
    // A handler runs on the runtime thread's loop, off the program's stack.
    println("clicked")
}
```

Event names: `click`, `change`, `select`, `scroll`.

A handler may declare **no parameters** (nothing is passed) or **one** — then it
receives the event payload where meaningful: the text (`change` on an edit), a
bool (`change` on a check box), or a value/index:

```dyon
fn on_change_name(new_text) {
    println("name is now " + new_text)
}
```

### Why handlers are safe

`ui_run` does not block. When `main` returns, the runtime moves the compiled
program and the widget tree into an `App`, and starts xui's message loop. Every
event calls its handler from `App::update` — the program is **off the stack**, so
the runtime is never re-entered. A handler may call setters freely.

---

## 5. Forms: design, generate, run

The IDE's designer edits a `.vrform`; code generation turns it into the Dyon
above. The generated shape is exactly the vocabulary in §2–§4:

```dyon
fn main() {
    w := ui_window("Main", 640, 480)
    root := ui_free(640, 480)
    c0 := ui_button("Greet", 10, 10, 90, 40)
    ui_add(root, c0)
    ui_anchor(c0, "top_left")
    ui_on(c0, "click", "on_click_Greet")
    ui_run(w, root)
}

fn on_click_Greet() {
}
```

Regeneration preserves handler bodies you have written. Anchors in the model map
one-to-one onto the runtime anchors, which are xui's own anchor layout.

---

## 6. Running and testing

- Run a project through the runtime; set `xui_DEMO_AUTOCLOSE_MS=3000` to have it
  close itself, which is how headless tests and screenshots run.
- The built-in sample lives at `examples/hello-window/`.
- Headless UI tests capture a PNG and inspect it (see `docs/TESTING.md`); a run
  with no desktop reports `NoWindow` and the test skips instead of failing.

---

## 7. Where to go next

- **Stdlib**: strings, math, date/time, files, encoding, regex, hashing — and
  (in progress) HTTP, JSON, XML/RSS, images.
- **Web**: a message-based HTTP server (`vr-web`) dispatches requests to Dyon
  handlers.
- **Packaging**: bundle a project and append it to `vr-runtime.exe` to get a
  self-contained executable.
- **Crates**: `vr-core` (project model), `vr-forms` (model + designer + codegen),
  `vr-dyon` (runtime + `ui_*`), `vr-web`, `vr-db`, `vr-std`, `vr-pack`, `vr-ide`.

See `docs/PLAN.md` for the architecture and the milestone map, and the issue
tracker for the roadmap.
