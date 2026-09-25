//! Binary entry point for the VisualRust IDE shell.

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    match vr_ide::run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("vr-ide: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(windows))]
fn main() {}
