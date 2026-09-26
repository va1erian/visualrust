//! Self-contained app stub: runs the bundle appended to this executable.
//!
//! The exported file is this binary plus a [`vr_pack::Bundle`] at EOF; the
//! loader resolves the running image with `current_exe`, so the same stub works
//! after the app is renamed or moved. A missing or broken bundle prints a typed
//! error and exits non-zero instead of panicking.

#![forbid(unsafe_code)]

use std::process::ExitCode;

fn main() -> ExitCode {
    match vr_runtime::run_embedded() {
        Ok(report) => {
            println!(
                "vr-runtime: ran `{}` ({:?}, {} entries)",
                report.entry.display(),
                report.kind,
                report.entries.len()
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("vr-runtime: {error}");
            ExitCode::FAILURE
        }
    }
}
