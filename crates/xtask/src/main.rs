use std::path::PathBuf;
use std::process::{Command, ExitCode};

fn workspace_root() -> Result<PathBuf, String> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(std::path::Path::parent)
        .map(PathBuf::from)
        .ok_or_else(|| "failed to resolve workspace root from xtask manifest path".to_string())
}

fn run_build_module() -> Result<ExitCode, String> {
    let root = workspace_root()?;
    let script = root.join("scripts/build.sh");

    if !script.is_file() {
        return Err(format!("build script not found at {}", script.display()));
    }

    let status = Command::new("bash")
        .arg(&script)
        .current_dir(&root)
        .status()
        .map_err(|err| format!("failed to execute {}: {err}", script.display()))?;

    Ok(ExitCode::from(status.code().unwrap_or(1) as u8))
}

fn usage() {
    eprintln!("usage: cargo build-module");
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        usage();
        return ExitCode::from(2);
    }

    match run_build_module() {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}
