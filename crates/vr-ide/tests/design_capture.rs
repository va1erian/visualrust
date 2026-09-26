//! End-to-end capture of design mode hosting the real `FormDesigner`.
//!
//! It launches the real IDE (no CLI argument, so it uses the built-in sample),
//! enters design mode with `VR_IDE_PROTOTYPE_DESIGN=1`, selects a control and
//! anchors it `Fill` with `VR_IDE_PROTOTYPE_DESIGN_DEMO=1`, then resizes the
//! form to the pane with `VR_IDE_PROTOTYPE_DESIGN_RESIZE_MS`. A helper thread
//! captures the live window twice with `vr_tooling::capture_window_rendered`:
//! once before the resize and once after.
//!
//! The PNGs are inspected for the designer page (`theme.raised`), the selection
//! handles (`theme.accent`) and the inspector's edit fields
//! (`theme.input_background`), and the two captures are compared to prove the
//! `Fill` control stretched. A session without a desktop prints `SKIP` and
//! passes.

use std::path::Path;
use std::time::Duration;

use vr_tooling::{WindowSelector, capture_window_rendered, diff, read_png};
use xui::Theme;

/// How long the shell stays open, leaving room for both captures.
const AUTOCLOSE_MS: &str = "6000";
/// Milliseconds after build at which the form is resized to the pane.
const RESIZE_MS: &str = "2600";
/// When the first (pre-resize) capture is attempted.
const BEFORE_AFTER: Duration = Duration::from_millis(1200);
/// How long after the first capture to wait before the second.
const BETWEEN: Duration = Duration::from_millis(2200);
const RETRY: Duration = Duration::from_millis(200);

#[test]
fn design_mode_hosts_the_form_designer() {
    // SAFETY: this integration-test binary contains exactly one test, so no
    // other test thread races the environment, and the shell reads these only
    // when it builds the window below.
    unsafe {
        std::env::set_var("xui_DEMO_THEME", "dark");
        std::env::set_var("xui_DEMO_AUTOCLOSE_MS", AUTOCLOSE_MS);
        std::env::set_var("VR_IDE_PROTOTYPE_DESIGN", "1");
        std::env::set_var("VR_IDE_PROTOTYPE_DESIGN_DEMO", "1");
        std::env::set_var("VR_IDE_PROTOTYPE_DESIGN_RESIZE_MS", RESIZE_MS);
    }

    let dir = std::env::temp_dir().join("vr-ide");
    let before_path = dir.join(format!("design-before-{}.png", std::process::id()));
    let after_path = dir.join(format!("design-after-{}.png", std::process::id()));
    let capture = {
        let before = before_path.clone();
        let after = after_path.clone();
        std::thread::spawn(move || {
            std::thread::sleep(BEFORE_AFTER);
            if !capture_retry(&before, 20) {
                return None;
            }
            std::thread::sleep(BETWEEN);
            capture_retry(&after, 20).then_some((before, after))
        })
    };

    let outcome = vr_ide::run_with(None);
    let captured = capture.join().ok().flatten();
    let Some((before, after)) = captured else {
        eprintln!("SKIP vr-ide design capture: no interactive desktop");
        return;
    };
    if outcome.is_err() {
        panic!("the IDE ran and captured but returned an error: {outcome:?}");
    }

    let pre = read_png(&before).expect("read the pre-resize capture");
    let post = read_png(&after).expect("read the post-resize capture");
    let theme = Theme::dark();

    let (page, accent, inspector) = probe(&pre, &theme);
    eprintln!(
        "vr-ide design capture: before {} ({}x{}), after {} ({}x{}); page={page}, \
         handles={accent}, inspector={inspector}",
        before.display(),
        pre.width,
        pre.height,
        after.display(),
        post.width,
        post.height,
    );
    assert!(page, "the designer page's raised surface is absent");
    assert!(accent, "the selected control's accent handles are absent");
    assert!(inspector, "the property inspector's edit fields are absent");

    let report = diff(&pre, &post).expect("both captures are the same window size");
    eprintln!(
        "Fill anchor: {:.2}% of pixels changed after the resize ({} of {})",
        report.percent(),
        report.differing,
        report.total,
    );
    assert!(
        report.fraction() > 0.01,
        "the Fill-anchored control did not visibly stretch on resize \
         ({:.3}% changed)",
        report.percent(),
    );
}

/// Captures `path`, retrying up to `attempts` times while the window paints.
fn capture_retry(path: &Path, attempts: u32) -> bool {
    for _ in 0..attempts {
        if capture_window_rendered(WindowSelector::title(vr_ide::TITLE), path).is_ok() {
            return true;
        }
        std::thread::sleep(RETRY);
    }
    false
}

/// Whether the capture shows the page, the selection handles and the inspector.
fn probe(image: &xui::RgbaImage, theme: &Theme) -> (bool, bool, bool) {
    let page = [theme.raised.r, theme.raised.g, theme.raised.b];
    let accent = [theme.accent.r, theme.accent.g, theme.accent.b];
    let edit = [
        theme.input_background.r,
        theme.input_background.g,
        theme.input_background.b,
    ];
    let (mut page_seen, mut accent_seen, mut inspector_seen) = (false, false, false);
    for (index, pixel) in image.pixels.as_chunks::<4>().0.iter().enumerate() {
        let color = [pixel[0], pixel[1], pixel[2]];
        let x = index as u32 % image.width.max(1);
        let y = index as u32 / image.width.max(1);
        let fx = f64::from(x) / f64::from(image.width.max(1));
        let fy = f64::from(y) / f64::from(image.height.max(1));
        if color == page && (0.25..0.75).contains(&fx) && (0.2..0.7).contains(&fy) {
            page_seen = true;
        }
        if color == accent && (0.25..0.9).contains(&fx) && (0.2..0.7).contains(&fy) {
            accent_seen = true;
        }
        // The inspector sits under the explorer on the left.
        if color == edit && fx < 0.22 && (0.55..0.95).contains(&fy) {
            inspector_seen = true;
        }
    }
    (page_seen, accent_seen, inspector_seen)
}
