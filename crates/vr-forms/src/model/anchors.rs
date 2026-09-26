//! The pure anchor engine: where a control lands when its form is resized.
//!
//! Every length here is a [`Dip`], so a layout is DPI-independent by
//! construction: the designer converts device pixels to design units on the way
//! in, but the resize maths itself never sees a pixel and so cannot pick up a
//! stale DPI. The rules are the classic nine-point anchor set plus the
//! stretch/fill combinations a resizable window needs.

use super::Form;
use super::geometry::{Anchor, Bounds, Dip, Size};

/// The smallest a control may become, in design units. Anchor maths may shrink
/// a control toward this floor but must never produce a zero or negative width
/// or height.
pub const MIN_ANCHOR_DIP: f64 = 1.0;

/// `bounds` repositioned and resized by a parent change from `origin` to
/// `resized`, under `anchor`.
///
/// A point anchor translates the whole control (or keeps it still) and leaves
/// its size alone; a stretch anchor moves the pinned edge(s) and leaves the
/// rest of the control's offset alone, so the extent grows by the parent's
/// delta. The result is clamped positive and inside `resized`.
pub fn anchored(origin: Size, resized: Size, bounds: Bounds, anchor: Anchor) -> Bounds {
    let dx = resized.width.get() - origin.width.get();
    let dy = resized.height.get() - origin.height.get();
    let x = bounds.x.get();
    let y = bounds.y.get();
    let w = bounds.width.get();
    let h = bounds.height.get();

    let (x, y, w, h) = match anchor {
        Anchor::TopLeft => (x, y, w, h),
        Anchor::Top => (x + dx / 2.0, y, w, h),
        Anchor::TopRight => (x + dx, y, w, h),
        Anchor::Left => (x, y + dy / 2.0, w, h),
        Anchor::Center => (x + dx / 2.0, y + dy / 2.0, w, h),
        Anchor::Right => (x + dx, y + dy / 2.0, w, h),
        Anchor::BottomLeft => (x, y + dy, w, h),
        Anchor::Bottom => (x + dx / 2.0, y + dy, w, h),
        Anchor::BottomRight => (x + dx, y + dy, w, h),
        Anchor::StretchHorizontal => (x, y, w + dx, h),
        Anchor::StretchVertical => (x, y, w, h + dy),
        Anchor::Fill => (x, y, w + dx, h + dy),
    };

    clamp_to_form(
        Bounds {
            x: Dip::new(x),
            y: Dip::new(y),
            width: Dip::new(w),
            height: Dip::new(h),
        },
        resized,
    )
}

/// Repositions every control of `form` for a resize to `new_size` and adopts
/// the new size.
///
/// The origin is captured before any control moves, so every anchor is applied
/// against the form's previous size rather than a partially updated one.
pub fn apply_anchors(form: &mut Form, new_size: Size) {
    let origin = form.size;
    for control in &mut form.controls {
        control.bounds = anchored(origin, new_size, control.bounds, control.anchor);
    }
    form.size = new_size;
}

/// Keeps a computed rectangle positive and inside `form`.
///
/// Anchoring can push an edge past its opposite (a control wider than a form
/// that shrank underneath it) or push the whole control off the form. The clamp
/// gives it a positive extent and slides it back in without changing which
/// edges were pinned. `form` is a validated, positive size, so the `max(0.0)`
/// guards only cover a caller that skipped validation.
fn clamp_to_form(bounds: Bounds, form: Size) -> Bounds {
    let max_width = form.width.get().max(0.0);
    let max_height = form.height.get().max(0.0);
    let width = bounds.width.get().max(MIN_ANCHOR_DIP).min(max_width);
    let height = bounds.height.get().max(MIN_ANCHOR_DIP).min(max_height);
    let x = bounds.x.get().clamp(0.0, (max_width - width).max(0.0));
    let y = bounds.y.get().clamp(0.0, (max_height - height).max(0.0));
    Bounds {
        x: Dip::new(x),
        y: Dip::new(y),
        width: Dip::new(width),
        height: Dip::new(height),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(width: f64, height: f64) -> Size {
        Size {
            width: Dip::new(width),
            height: Dip::new(height),
        }
    }

    fn bounds(x: f64, y: f64, width: f64, height: f64) -> Bounds {
        Bounds {
            x: Dip::new(x),
            y: Dip::new(y),
            width: Dip::new(width),
            height: Dip::new(height),
        }
    }

    /// A 100x100 growth of a 400x300 form; a (10, 20) 50x40 control.
    fn fixture(anchor: Anchor) -> Bounds {
        anchored(
            size(400.0, 300.0),
            size(500.0, 400.0),
            bounds(10.0, 20.0, 50.0, 40.0),
            anchor,
        )
    }

    fn rect(bounds: Bounds) -> (f64, f64, f64, f64) {
        (
            bounds.x.get(),
            bounds.y.get(),
            bounds.width.get(),
            bounds.height.get(),
        )
    }

    #[test]
    fn every_scheme_has_its_documented_result() {
        let cases = [
            (Anchor::TopLeft, (10.0, 20.0, 50.0, 40.0)),
            (Anchor::Top, (60.0, 20.0, 50.0, 40.0)),
            (Anchor::TopRight, (110.0, 20.0, 50.0, 40.0)),
            (Anchor::Left, (10.0, 70.0, 50.0, 40.0)),
            (Anchor::Center, (60.0, 70.0, 50.0, 40.0)),
            (Anchor::Right, (110.0, 70.0, 50.0, 40.0)),
            (Anchor::BottomLeft, (10.0, 120.0, 50.0, 40.0)),
            (Anchor::Bottom, (60.0, 120.0, 50.0, 40.0)),
            (Anchor::BottomRight, (110.0, 120.0, 50.0, 40.0)),
            (Anchor::StretchHorizontal, (10.0, 20.0, 150.0, 40.0)),
            (Anchor::StretchVertical, (10.0, 20.0, 50.0, 140.0)),
            (Anchor::Fill, (10.0, 20.0, 150.0, 140.0)),
        ];
        for (anchor, expected) in cases {
            assert_eq!(rect(fixture(anchor)), expected, "{anchor:?}");
        }
    }

    #[test]
    fn a_sticky_control_keeps_its_left_edge_and_a_right_anchor_keeps_the_gap() {
        // Sticky: nothing moves at all.
        let sticky = fixture(Anchor::StretchHorizontal);
        assert_eq!(sticky.x.get(), 10.0, "pinned left edge stays put");
        assert_eq!(
            sticky.x.get() + sticky.width.get(),
            160.0,
            "the right edge follows the parent by exactly the delta"
        );
        assert_eq!(
            sticky.width.get(),
            50.0 + 100.0,
            "a StretchHorizontal width grows by the parent delta"
        );

        // A point Right anchor keeps its distance to the parent's right edge:
        // the control translates by the parent delta but the gap is unchanged.
        let right = fixture(Anchor::Right);
        assert_eq!(500.0 - (right.x.get() + right.width.get()), 340.0);
        assert_eq!(right.x.get(), 10.0 + 100.0);
    }

    #[test]
    fn fill_grows_both_axes_and_center_stays_centred() {
        let fill = fixture(Anchor::Fill);
        assert_eq!((fill.width.get(), fill.height.get()), (150.0, 140.0));

        // The control's offset from the parent's centre is preserved: it was
        // (-165, -110) around the 400x300 centre and stays so around 500x400.
        let center = fixture(Anchor::Center);
        assert_eq!(center.x.get() + center.width.get() / 2.0, 85.0);
        assert_eq!(center.y.get() + center.height.get() / 2.0, 90.0);
    }

    #[test]
    fn apply_anchors_repositions_the_whole_form() {
        let mut form = Form {
            name: "Main".into(),
            size: size(400.0, 300.0),
            controls: vec![
                control("sticky", Anchor::TopLeft, bounds(10.0, 10.0, 40.0, 20.0)),
                control(
                    "wide",
                    Anchor::StretchHorizontal,
                    bounds(10.0, 100.0, 40.0, 20.0),
                ),
                control("full", Anchor::Fill, bounds(10.0, 200.0, 40.0, 20.0)),
            ],
        };
        apply_anchors(&mut form, size(600.0, 400.0));

        assert_eq!(form.size.width.get(), 600.0);
        assert_eq!(rect(form.controls[0].bounds), (10.0, 10.0, 40.0, 20.0));
        assert_eq!(rect(form.controls[1].bounds), (10.0, 100.0, 240.0, 20.0));
        assert_eq!(rect(form.controls[2].bounds), (10.0, 200.0, 240.0, 120.0));
        form.validate().expect("anchored form stays valid");
    }

    #[test]
    fn nothing_leaves_the_form_when_it_shrinks_past_a_control() {
        let mut form = Form {
            name: "Main".into(),
            size: size(400.0, 300.0),
            controls: vec![
                control(
                    "bottom",
                    Anchor::BottomRight,
                    bounds(300.0, 220.0, 80.0, 60.0),
                ),
                control("full", Anchor::Fill, bounds(0.0, 0.0, 380.0, 280.0)),
            ],
        };
        apply_anchors(&mut form, size(60.0, 40.0));

        for control in &form.controls {
            let b = control.bounds;
            assert!(
                b.width.get() > 0.0 && b.height.get() > 0.0,
                "{} positive",
                control.name
            );
            assert!(
                b.x.get() >= 0.0 && b.y.get() >= 0.0,
                "{} origin",
                control.name
            );
            assert!(
                b.x.get() + b.width.get() <= 60.0 + f64::EPSILON
                    && b.y.get() + b.height.get() <= 40.0 + f64::EPSILON,
                "{} inside the shrunken form",
                control.name
            );
        }
        form.validate()
            .expect("control sized to the form stays valid");
    }

    fn control(name: &str, anchor: Anchor, bounds: Bounds) -> super::super::Control {
        super::super::Control {
            kind: super::super::ControlKind::Button,
            name: name.into(),
            bounds,
            text: String::new(),
            enabled: true,
            visible: true,
            tooltip: None,
            anchor,
        }
    }
}
