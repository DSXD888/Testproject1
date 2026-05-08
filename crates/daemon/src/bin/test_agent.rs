#![forbid(unsafe_code)]

use std::io::{IsTerminal, Write};
use std::os::unix::process::CommandExt;
use std::process::{self, Command, Stdio};
use std::thread;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand};
use nix::unistd::{Pid, getpgid, getsid};
use serde::Serialize;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    cmd: CommandKind,
}

#[derive(Subcommand)]
enum CommandKind {
    PtyEcho { message: String },
    ReportSessionSemantics,
    Sleep { seconds: u64 },
    SilentStall { seconds: u64 },
    IgnoreIntTerm { seconds: u64 },
    EmitJsonlGood,
    EmitJsonlMalformed,
    EmitJsonlPartial,
    EmitStreamJsonGood,
    EmitStreamJsonMalformed,
    EmitStreamJsonPartial,
    CheckpointThenCrash,
    ForkGrandchild { seconds: u64 },
    DoubleForkEscape { seconds: u64 },
    StdoutStderr,
    EmitAuthChallenge,
    ExitCode { code: i32 },
}

#[derive(Serialize)]
struct SessionSemantics {
    isatty_stdin: bool,
    isatty_stdout: bool,
    isatty_stderr: bool,
    tty: String,
    pid: u32,
    pgid: u32,
    sid: u32,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        CommandKind::PtyEcho { message } => pty_echo(&message),
        CommandKind::ReportSessionSemantics => report_session_semantics(),
        CommandKind::Sleep { seconds } | CommandKind::SilentStall { seconds } => {
            sleep_for(seconds);
            Ok(())
        }
        CommandKind::IgnoreIntTerm { seconds } => ignore_int_term(seconds),
        CommandKind::EmitJsonlGood => emit_jsonl_good(),
        CommandKind::EmitJsonlMalformed => emit_jsonl_malformed(),
        CommandKind::EmitJsonlPartial => emit_jsonl_partial(),
        CommandKind::EmitStreamJsonGood => emit_stream_json_good(),
        CommandKind::EmitStreamJsonMalformed => emit_stream_json_malformed(),
        CommandKind::EmitStreamJsonPartial => emit_stream_json_partial(),
        CommandKind::CheckpointThenCrash => checkpoint_then_crash(),
        CommandKind::ForkGrandchild { seconds } => fork_grandchild(seconds),
        CommandKind::DoubleForkEscape { seconds } => double_fork_escape(seconds),
        CommandKind::StdoutStderr => stdout_stderr(),
        CommandKind::EmitAuthChallenge => emit_auth_challenge(),
        CommandKind::ExitCode { code } => {
            process::exit(code);
        }
    }
}

fn pty_echo(message: &str) -> Result<()> {
    println!("{message}");
    std::io::stdout().flush().context("flushing PTY echo")?;
    Ok(())
}

fn report_session_semantics() -> Result<()> {
    let pid = process::id();
    let pid_raw = Pid::from_raw(pid.cast_signed());
    let report = SessionSemantics {
        isatty_stdin: std::io::stdin().is_terminal(),
        isatty_stdout: std::io::stdout().is_terminal(),
        isatty_stderr: std::io::stderr().is_terminal(),
        tty: tty_path()?,
        pid,
        pgid: getpgid(Some(pid_raw))
            .context("reading process group id")?
            .as_raw()
            .cast_unsigned(),
        sid: getsid(Some(pid_raw))
            .context("reading session id")?
            .as_raw()
            .cast_unsigned(),
    };
    println!(
        "{}",
        serde_json::to_string(&report).context("serializing session semantics")?
    );
    std::io::stdout()
        .flush()
        .context("flushing session semantics report")?;
    Ok(())
}

fn tty_path() -> Result<String> {
    let output = Command::new("tty")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .context("running tty")?;
    if output.status.success() {
        return Ok(String::from_utf8(output.stdout)
            .context("decoding tty output")?
            .trim()
            .to_owned());
    }
    Ok("/dev/not-a-tty".to_owned())
}

fn sleep_for(seconds: u64) {
    thread::sleep(Duration::from_secs(seconds));
}

fn ignore_int_term(seconds: u64) -> Result<()> {
    let error = Command::new("python3")
        .arg("-c")
        .arg(format!(
            "import signal, sys, time; signal.signal(signal.SIGINT, signal.SIG_IGN); signal.signal(signal.SIGTERM, signal.SIG_IGN); sys.stdout.write('ready\\n'); sys.stdout.flush(); time.sleep({seconds})"
        ))
        .exec();
    Err(anyhow!("exec python for ignore-int-term failed: {error}"))
}

fn emit_jsonl_good() -> Result<()> {
    println!(r#"{{"event":"started","seq":1}}"#);
    println!(r#"{{"event":"checkpoint","seq":2,"path":"checkpoint-2.json"}}"#);
    println!(r#"{{"event":"result","path":"result.json"}}"#);
    std::io::stdout().flush().context("flushing JSONL stream")?;
    Ok(())
}

fn emit_jsonl_malformed() -> Result<()> {
    println!(r#"{{"event":"started","seq":1}}"#);
    println!(r#"{{"event":"broken""#);
    std::io::stdout()
        .flush()
        .context("flushing malformed JSONL")?;
    Ok(())
}

fn emit_jsonl_partial() -> Result<()> {
    print!(r#"{{"event":"partial","seq":3"#);
    std::io::stdout()
        .flush()
        .context("flushing partial JSONL")?;
    Ok(())
}

fn emit_stream_json_good() -> Result<()> {
    print!(r#"{{"type":"progress","#);
    std::io::stdout()
        .flush()
        .context("flushing stream JSON head")?;
    print!(r#""completed":1,"total":2}}"#);
    println!();
    std::io::stdout()
        .flush()
        .context("flushing stream JSON tail")?;
    Ok(())
}

fn emit_stream_json_malformed() -> Result<()> {
    println!(r#"{{"type":"progress","completed":1"#);
    std::io::stdout()
        .flush()
        .context("flushing malformed stream JSON")?;
    Ok(())
}

fn emit_stream_json_partial() -> Result<()> {
    print!(r#"{{"type":"partial","completed":1"#);
    std::io::stdout()
        .flush()
        .context("flushing partial stream JSON")?;
    Ok(())
}

fn checkpoint_then_crash() -> Result<()> {
    println!(r#"{{"event":"checkpoint","seq":1,"path":"checkpoint-1.json"}}"#);
    std::io::stdout()
        .flush()
        .context("flushing checkpoint before crash")?;
    process::exit(7);
}

fn fork_grandchild(seconds: u64) -> Result<()> {
    let current_exe = std::env::current_exe().context("locating current executable")?;
    let _child = Command::new(current_exe)
        .arg("sleep")
        .arg("--seconds")
        .arg(seconds.to_string())
        .spawn()
        .context("spawning grandchild helper")?;
    sleep_for(seconds);
    Ok(())
}

fn double_fork_escape(seconds: u64) -> Result<()> {
    let current_exe = std::env::current_exe().context("locating current executable")?;
    let _detached = Command::new(current_exe)
        .arg("sleep")
        .arg(seconds.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .context("spawning detached helper in a separate process group")?;
    sleep_for(seconds);
    Ok(())
}

fn stdout_stderr() -> Result<()> {
    println!("stdout-frame");
    eprintln!("stderr-frame");
    std::io::stdout().flush().context("flushing stdout frame")?;
    std::io::stderr().flush().context("flushing stderr frame")?;
    Ok(())
}

fn emit_auth_challenge() -> Result<()> {
    println!("AUTH CHALLENGE: re-login required");
    std::io::stdout()
        .flush()
        .context("flushing auth challenge")?;
    Ok(())
}
