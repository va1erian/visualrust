//! End-to-end check that a form resize anchors its controls in a real window.
//!
//! Hosts a [`FormDesigner`] with a sticky (`TopLeft`) button and a `Fill`
//! button, captures the window before and after
//! [`FormDesigner::resize_form`], and compares the area of the control proxies:
//! the `Fill` control must grow while the sticky one stays put. Captures go
//! through `vr_tooling::capture_window_rendered` (the `PrintWindow` path, safe
//! across processes).
//!
//! Follows the workspace skip pattern: a session with no desktop prints `SKIP`
//! and passes, but a captured window whose `Fill` control did not grow fails.

#![cfg(windows)]

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use vr_forms::design::{DesignerEvent, FormDesigner};
use vr_forms::{Anchor, Bounds, Control, ControlKind, Dip, Form, Size};
use vr_tooling::{WindowSelector, capture_window_rendered, read_png};
use xui::{App, ControlExt, Hwnd, Rect, Theme, Ui, WindowSpec, dip, run_app};

/// How long to let the widget paint before each capture, in ms.
const CAPTURE_MS: u32 = 900;

/// The form's starting design surface.
fn original_size() -> Size {
    Size {
        width: Dip::new(360.0),
        height: Dip::new(260.0),
    }
}

/// The size `resize_form` anchors to: +200 design units wide, +160 tall.
fn resized_size() -> Size {
    Size {
        width: Dip::new(560.0),
        height: Dip::new(420.0),
    }
}

fn control(name: &str, anchor: Anchor, x: f64, y: f64, w: f64, h: f64) -> Control {
    Control {
        kind: ControlKind::Button,
        name: name.into(),
        bounds: Bounds {
            x: Dip::new(x),
            y: Dip::new(y),
            width: Dip::new(w),
            height: Dip::new(h),
        },
        text: String::new(),
        enabled: true,
        visible: true,
        tooltip: None,
        anchor,
    }
}

fn sample_form() -> Form {
    Form {
        name: "AnchorDemo".into(),
        size: original_size(),
        controls: vec![
            control("sticky", Anchor::TopLeft, 16.0, 16.0, 80.0, 40.0),
            control("full", Anchor::Fill, 120.0, 16.0, 200.0, 100.0),
        ],
    }
}

/// The messages the test app handles.
enum Msg {
    Capture,
}

/// The test app: captures, resizes, captures again, then quits.
struct AnchorApp {
    designer: Option<FormDesigner<Msg>>,
    window: Hwnd,
    out_before: PathBuf,
    out_after: PathBuf,
    captures: Rc<RefCell<Vec<Result<PathBuf, String>>>>,
    /// The form as it stood immediately after the resize.
    post: Rc<RefCell<Option<Form>>>,
    tick: u32,
}

impl App for AnchorApp {
    type Msg = Msg;

    fn update(&mut self, msg: Msg, ui: &mut Ui<Msg>) {
        let Msg::Capture = msg;
        let (path, designer) = if self.tick == 0 {
            (&self.out_before, self.designer.as_mut())
        } else {
            (&self.out_after, None)
        };
        let outcome = capture_window_rendered(WindowSelector::hwnd(self.window), path)
            .map_err(|e| e.to_string());
        self.captures.borrow_mut().push(outcome);

        if self.tick == 0 {
            self.tick = 1;
            if let Some(designer) = designer {
                designer.resize_form(resized_size());
                *self.post.borrow_mut() = Some(designer.form());
            }
        } else {
            ui.quit();
        }
    }
}

/// What one theme's run produced.
struct Capture {
    before: PathBuf,
    after: PathBuf,
    before_surface: usize,
    after_surface: usize,
    width: u32,
    height: u32,
    distinct: usize,
}

/// Counts pixels exactly matching `color` in a straight-RGBA image.
fn count_color(image: &xui::RgbaImage, color: xui::Color) -> usize {
    let rgb = [color.r, color.g, color.b];
    image
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|pixel| pixel[..3] == rgb)
        .count()
}

fn run(theme: Theme, name: &str) -> Result<Capture, String> {
    let dir = std::env::temp_dir().join("vr-forms-issue38");
    let out_before = dir.join(format!("{name}-before.png"));
    let out_after = dir.join(format!("{name}-after.png"));
    let captures = Rc::new(RefCell::new(Vec::new()));
    let post: Rc<RefCell<Option<Form>>> = Rc::new(RefCell::new(None));
    let setup_error: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let before_in = out_before.clone();
    let after_in = out_after.clone();
    let captures_in = Rc::clone(&captures);
    let post_in = Rc::clone(&post);
    let setup_in = Rc::clone(&setup_error);
    let spec = WindowSpec::new(format!("vr-forms anchor resize {name}"))
        .size(dip(780.0), dip(560.0))
        .theme(theme);

    let ran = run_app(spec, move |ui| {
        let mut designer = match FormDesigner::new(ui, sample_form(), Dip::new(8.0)) {
            Ok(designer) => designer,
            Err(error) => {
                *setup_in.borrow_mut() = Some(error.to_string());
                ui.quit();
                return AnchorApp {
                    designer: None,
                    window: ui.hwnd(),
                    out_before: before_in.clone(),
                    out_after: after_in.clone(),
                    captures: Rc::clone(&captures_in),
                    post: Rc::clone(&post_in),
                    tick: 0,
                };
            }
        };
        designer.set_bounds(Rect::new(0, 0, 760, 520));
        designer.on_change(|event| match event {
            DesignerEvent::SelectionChanged(_) | DesignerEvent::FormEdited => None,
        });
        if let Ok(timer) = ui.set_timer(CAPTURE_MS) {
            ui.on_timer(move |fired| (fired == timer).then_some(Msg::Capture));
        }
        AnchorApp {
            designer: Some(designer),
            window: ui.hwnd(),
            out_before: before_in.clone(),
            out_after: after_in.clone(),
            captures: Rc::clone(&captures_in),
            post: Rc::clone(&post_in),
            tick: 0,
        }
    });

    if let Err(error) = ran {
        return Err(format!("could not create a window: {error}"));
    }
    if let Some(error) = setup_error.borrow_mut().take() {
        return Err(format!("could not build the designer: {error}"));
    }

    // The model half of the evidence: the resize stretched the Fill control and
    // left the sticky one alone.
    let post = post.borrow_mut().take().ok_or("the resize never ran")?;
    let full = &post.controls[1].bounds;
    assert_eq!(
        (full.width.get(), full.height.get()),
        (400.0, 260.0),
        "Fill width/height grew by the parent delta"
    );
    assert_eq!(post.controls[0].bounds, sample_form().controls[0].bounds);

    let captures = captures.borrow();
    let before = match captures.first() {
        Some(Ok(path)) => path.clone(),
        Some(Err(error)) => return Err(format!("first capture unavailable: {error}")),
        None => return Err("the loop ended before the first capture".to_owned()),
    };
    let after = match captures.get(1) {
        Some(Ok(path)) => path.clone(),
        Some(Err(error)) => return Err(format!("second capture unavailable: {error}")),
        None => return Err("the loop ended before the second capture".to_owned()),
    };

    let before_image = read_png(&before).map_err(|e| e.to_string())?;
    let after_image = read_png(&after).map_err(|e| e.to_string())?;
    let surface = theme.surface;
    let mut distinct = std::collections::HashSet::new();
    for pixel in after_image.pixels.as_chunks::<4>().0 {
        distinct.insert([pixel[0], pixel[1], pixel[2]]);
    }
    Ok(Capture {
        before,
        after,
        before_surface: count_color(&before_image, surface),
        after_surface: count_color(&after_image, surface),
        width: after_image.width,
        height: after_image.height,
        distinct: distinct.len(),
    })
}

#[test]
fn a_form_resize_stretches_a_fill_control_and_leaves_a_sticky_one() {
    xui::init();

    let mut captured = 0usize;
    for (name, theme) in [("light", Theme::light()), ("dark", Theme::dark())] {
        match run(theme, name) {
            Err(reason) => eprintln!("SKIP vr-forms anchor capture ({name}): {reason}"),
            Ok(capture) => {
                captured += 1;
                eprintln!(
                    "vr-forms anchor capture ({name}): before {} ({} surface px), after {} \
                     ({} surface px), {}x{}, {} distinct colours",
                    capture.before.display(),
                    capture.before_surface,
                    capture.after.display(),
                    capture.after_surface,
                    capture.width,
                    capture.height,
                    capture.distinct,
                );
                assert!(
                    capture.after_surface > capture.before_surface,
                    "{name}: the Fill proxy did not grow ({} -> {} surface px)",
                    capture.before_surface,
                    capture.after_surface
                );
                assert!(
                    capture.distinct >= 3,
                    "{name}: only {} distinct colours; the page and proxies did not render",
                    capture.distinct
                );
            }
        }
    }

    if captured == 0 {
        eprintln!("SKIP vr-forms anchor capture: no interactive desktop");
    }
}
