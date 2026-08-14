//! Luna's supervisor.
//!
//! This is the process the user starts. It spawns the app, waits, and acts on how the
//! app exited: quit, restart, or rebuild and replace the binary.
//!
//! It exists because Windows will not let a running executable be overwritten, so the
//! add-a-tool flow needs something outside the app to do the replacing.
//!
//! It is deliberately small and changes rarely. It holds no application logic, so a
//! broken tool can make the app fail to build but can never make the launcher itself
//! unstartable.

use std::path::{Path, PathBuf};
use std::process::Command;

use luna_core::lifecycle::{self, ShutdownIntent};

mod rebuild;
mod swap;

use swap::{BinaryPaths, SwapOutcome};

/// Build profile the launcher compiles and runs.
///
/// Debug while Luna itself is under development, since that is what is on disk. This
/// becomes `release` once there are releases to make.
const PROFILE: &str = "debug";

fn main() -> std::process::ExitCode {
    let install_dir = match install_dir() {
        Some(dir) => dir,
        None => {
            eprintln!("luna_launcher: could not determine its own directory");
            return std::process::ExitCode::FAILURE;
        }
    };

    let paths = BinaryPaths::in_dir(&install_dir);

    if !paths.current.is_file() {
        eprintln!(
            "luna_launcher: no app binary at {}.\n\
             Build one with `cargo build -p luna_ui` and copy it here, or run the app \
             directly during development.",
            paths.current.display()
        );
        return std::process::ExitCode::FAILURE;
    }

    return supervise(&install_dir, &paths);
}

/// Runs the app until it asks to stop for good.
fn supervise(install_dir: &Path, paths: &BinaryPaths) -> std::process::ExitCode {
    loop {
        let code = match run_app(&paths.current) {
            Ok(code) => code,
            Err(e) => {
                eprintln!("luna_launcher: could not start {}: {e}", paths.current.display());

                // A binary that will not even start is exactly what .prev is for.
                if swap::can_roll_back(paths) {
                    eprintln!("luna_launcher: restoring the previous version and retrying");
                    if swap::rollback(paths).is_ok() {
                        continue;
                    }
                }

                return std::process::ExitCode::FAILURE;
            }
        };

        match ShutdownIntent::from_exit_code(code) {
            Some(ShutdownIntent::Exit) => return std::process::ExitCode::SUCCESS,

            Some(ShutdownIntent::Restart) => continue,

            Some(ShutdownIntent::Rebuild) => {
                if !do_rebuild(install_dir, paths) {
                    // The app relaunches either way. On failure it is the previous
                    // binary, which reads the log and tells the user what broke.
                    eprintln!("luna_launcher: rebuild failed, keeping the current version");
                }
                continue;
            }

            // Anything else means the app died rather than asked for something. Not
            // treated as a request, and not restarted in a loop.
            None => {
                eprintln!("luna_launcher: the app exited unexpectedly with code {code}");
                return std::process::ExitCode::FAILURE;
            }
        }
    }
}

/// Builds and swaps in a new binary. Returns whether the swap happened.
fn do_rebuild(install_dir: &Path, paths: &BinaryPaths) -> bool {
    println!("luna_launcher: rebuilding to pick up tool changes");

    let result = rebuild::run(install_dir, PROFILE);
    let log = rebuild::write_log(&install_dir.join("logs"), &result);

    if !result.succeeded {
        eprintln!("luna_launcher: build failed, see {}", log.display());
        return false;
    }

    let fresh = rebuild::built_binary(install_dir, PROFILE);

    return match swap::install(paths, &fresh) {
        Ok(SwapOutcome::Swapped) => {
            println!("luna_launcher: new version installed");
            true
        }
        Ok(SwapOutcome::AlreadyCurrent) => {
            println!("luna_launcher: build produced no change");
            true
        }
        Err(e) => {
            eprintln!("luna_launcher: could not install the new binary: {e}");
            false
        }
    };
}

/// Runs the app to completion and returns its exit code.
fn run_app(binary: &Path) -> std::io::Result<i32> {
    let status = Command::new(binary).status()?;

    // A process killed by a signal has no code. Treated as an unexpected exit, which
    // is what it is.
    return Ok(status.code().unwrap_or(i32::MIN));
}

/// The directory the launcher itself lives in.
fn install_dir() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os(lifecycle_env()) {
        let dir = PathBuf::from(dir);
        if !dir.as_os_str().is_empty() {
            return Some(dir);
        }
    }

    return std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()));
}

/// Shares the app's development override, so both halves agree on where things are.
fn lifecycle_env() -> &'static str {
    return luna_core::paths::INSTALL_DIR_ENV;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_known_exit_codes_are_treated_as_requests() {
        assert_eq!(
            ShutdownIntent::from_exit_code(lifecycle::EXIT_REBUILD),
            Some(ShutdownIntent::Rebuild)
        );
        assert_eq!(ShutdownIntent::from_exit_code(1), None);
    }

    #[test]
    fn the_build_profile_matches_where_the_binary_is_looked_for() {
        let built = rebuild::built_binary(Path::new("/install"), PROFILE);
        assert!(built.to_string_lossy().contains(PROFILE));
    }
}
