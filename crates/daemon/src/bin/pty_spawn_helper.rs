#![forbid(unsafe_code)]

use anyhow::{Result, anyhow};
use std::os::unix::process::CommandExt;
use std::process::Command;

fn main() -> Result<()> {
    let mut args = std::env::args_os();
    let _ = args.next();
    let program = args
        .next()
        .ok_or_else(|| anyhow!("pty_spawn_helper requires a target program"))?;
    let error = Command::new(&program).args(args).exec();
    Err(anyhow!(
        "failed to exec {} via pty_spawn_helper: {error}",
        program.to_string_lossy()
    ))
}
