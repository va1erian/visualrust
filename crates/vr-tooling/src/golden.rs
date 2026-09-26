//! Golden-image storage and comparison.
//!
//! Goldens live as PNGs under `tests/golden/` next to this crate's manifest. A
//! missing golden fails loudly unless the `VR_ACCEPT_GOLDEN` environment
//! variable is set, in which case the capture is recorded as the new golden.

use std::env;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

use xui::RgbaImage;

use crate::capture::{WindowSelector, capture_image};
use crate::error::{Result, ToolingError};

/// Set this environment variable to record a missing golden instead of failing.
pub const ACCEPT_ENV: &str = "VR_ACCEPT_GOLDEN";

/// Set this environment variable to store goldens somewhere other than the
/// default `tests/golden/` (a test can point at a temp directory).
pub const GOLDEN_DIR_ENV: &str = "VR_GOLDEN_DIR";

/// The directory goldens are stored in: `$VR_GOLDEN_DIR`, or `tests/golden/`.
pub fn golden_dir() -> PathBuf {
    if let Some(dir) = env::var_os(GOLDEN_DIR_ENV) {
        return PathBuf::from(dir);
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("golden")
}

/// The golden path for `name`.
pub fn golden_path(name: &str) -> PathBuf {
    golden_dir().join(format!("{name}.png"))
}

/// How many pixels differ between a capture and a golden.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Diff {
    /// Pixels that are not byte-identical.
    pub differing: usize,
    /// Total pixel count.
    pub total: usize,
}

impl Diff {
    /// The differing fraction, `0.0..=1.0`.
    pub fn fraction(&self) -> f64 {
        self.differing as f64 / self.total.max(1) as f64
    }

    /// The differing fraction as a percentage, `0.0..=100.0`.
    pub fn percent(&self) -> f64 {
        self.fraction() * 100.0
    }
}

/// Writes `image` to `path` as an 8-bit RGBA PNG, creating parent directories.
pub fn write_png(image: &RgbaImage, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent).map_err(|source| io_error(parent, source))?;
    }
    let file = File::create(path).map_err(|source| io_error(path, source))?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), image.width, image.height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|error| png_error(path, error))?;
    writer
        .write_image_data(&image.pixels)
        .map_err(|error| png_error(path, error))
}

/// Reads an 8-bit RGBA (or RGB) PNG from `path`.
pub fn read_png(path: &Path) -> Result<RgbaImage> {
    let file = File::open(path).map_err(|source| io_error(path, source))?;
    let decoder = png::Decoder::new(BufReader::new(file));
    let mut reader = decoder
        .read_info()
        .map_err(|error| png_error(path, error))?;
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|error| png_error(path, error))?;
    if info.bit_depth != png::BitDepth::Eight {
        return Err(ToolingError::Png {
            path: path.to_path_buf(),
            message: format!("unsupported bit depth {:?}; expected 8-bit", info.bit_depth),
        });
    }
    let pixels = match info.color_type {
        png::ColorType::Rgba => buf[..info.buffer_size()].to_vec(),
        png::ColorType::Rgb => rgb_to_rgba(&buf[..info.buffer_size()]),
        other => {
            return Err(ToolingError::Png {
                path: path.to_path_buf(),
                message: format!("unsupported colour type {other:?}; expected RGBA or RGB"),
            });
        }
    };
    Ok(RgbaImage {
        width: info.width,
        height: info.height,
        pixels,
    })
}

/// Compares two images pixel by pixel, or `None` when their sizes differ.
pub fn diff(actual: &RgbaImage, golden: &RgbaImage) -> Option<Diff> {
    if actual.width != golden.width || actual.height != golden.height {
        return None;
    }
    let total = actual.width as usize * actual.height as usize;
    let differing = actual
        .pixels
        .as_chunks::<4>()
        .0
        .iter()
        .zip(golden.pixels.as_chunks::<4>().0.iter())
        .filter(|(actual, golden)| actual != golden)
        .count();
    Some(Diff { differing, total })
}

/// Records `image` as the golden for `name`, overwriting any existing one.
pub fn accept_golden(image: &RgbaImage, name: &str) -> Result<PathBuf> {
    let path = golden_path(name);
    write_png(image, &path)?;
    Ok(path)
}

/// Captures `selector` and asserts it matches the golden `name` within
/// `tolerance` (a differing-pixel fraction, `0.0..=1.0`).
pub fn assert_selector_matches_golden(
    selector: WindowSelector,
    name: &str,
    tolerance: f64,
) -> Result<()> {
    let image = capture_image(&selector)?;
    assert_matches_golden(&image, name, tolerance)
}

/// Asserts `image` matches the golden `name` within `tolerance` (a
/// differing-pixel fraction, `0.0..=1.0`).
///
/// A missing golden is recorded instead of failing when [`ACCEPT_ENV`] is set.
/// A size mismatch always fails. A difference over the tolerance fails with the
/// percentage and writes the capture to a temp file for inspection.
pub fn assert_matches_golden(image: &RgbaImage, name: &str, tolerance: f64) -> Result<()> {
    if !(0.0..=1.0).contains(&tolerance) {
        return Err(ToolingError::InvalidTolerance { tolerance });
    }
    let path = golden_path(name);
    if !path.exists() {
        if env::var_os(ACCEPT_ENV).is_some() {
            write_png(image, &path)?;
            return Ok(());
        }
        return Err(ToolingError::GoldenMissing {
            name: name.to_string(),
            path,
        });
    }
    let golden = read_png(&path)?;
    match diff(image, &golden) {
        None => Err(ToolingError::GoldenSize {
            name: name.to_string(),
            actual_width: image.width,
            actual_height: image.height,
            golden_width: golden.width,
            golden_height: golden.height,
        }),
        Some(report) if report.fraction() > tolerance => {
            let actual = write_mismatch(image, name)?;
            Err(ToolingError::GoldenDiff {
                name: name.to_string(),
                diff_percent: report.percent(),
                tolerance_percent: tolerance * 100.0,
                actual,
            })
        }
        Some(_) => Ok(()),
    }
}

/// Writes a mismatching capture to a temp path so a human can inspect it.
fn write_mismatch(image: &RgbaImage, name: &str) -> Result<PathBuf> {
    let path = env::temp_dir()
        .join("vr-tooling")
        .join("golden-actual")
        .join(format!("{name}.png"));
    write_png(image, &path)?;
    Ok(path)
}

fn rgb_to_rgba(rgb: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(rgb.len() / 3 * 4);
    for pixel in rgb.as_chunks::<3>().0 {
        rgba.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 0xFF]);
    }
    rgba
}

fn io_error(path: &Path, source: std::io::Error) -> ToolingError {
    ToolingError::Io {
        path: path.to_path_buf(),
        source,
    }
}

fn png_error(path: &Path, error: impl std::fmt::Display) -> ToolingError {
    ToolingError::Png {
        path: path.to_path_buf(),
        message: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(width: u32, height: u32, channels: [u8; 4]) -> RgbaImage {
        let pixels = channels
            .iter()
            .copied()
            .cycle()
            .take(width as usize * height as usize * 4)
            .collect();
        RgbaImage {
            width,
            height,
            pixels,
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        env::temp_dir()
            .join("vr-tooling")
            .join("unit-tests")
            .join(format!("{name}.png"))
    }

    #[test]
    fn png_round_trips_pixels() {
        let image = sample(3, 2, [10, 20, 30, 255]);
        let path = temp_path("round-trip");
        write_png(&image, &path).expect("write");
        let read = read_png(&path).expect("read");
        assert_eq!(read, image);
    }

    #[test]
    fn diff_counts_changed_pixels() {
        let golden = sample(2, 2, [0, 0, 0, 255]);
        let identical = sample(2, 2, [0, 0, 0, 255]);
        let mut changed = golden.clone();
        changed.pixels[0] = 255;
        assert_eq!(diff(&identical, &golden).expect("same size").differing, 0);
        let report = diff(&changed, &golden).expect("same size");
        assert_eq!(report.differing, 1);
        assert!((report.percent() - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn diff_rejects_size_mismatch() {
        let small = sample(2, 2, [0, 0, 0, 255]);
        let large = sample(3, 3, [0, 0, 0, 255]);
        assert!(diff(&small, &large).is_none());
    }
}
