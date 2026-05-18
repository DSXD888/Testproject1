//! `ut_ehlpr_001..099` — smoke tests for `bin/ensure-cdp-9222`.
//!
//! Assert the helper's structural properties (exists, executable, handles
//! unsupported OS, emits diagnostic JSON). Does NOT launch real Chrome — that
//! is exercised by the live integration scenarios that drive the shipment-sizing
//! flow against a live CDP browser.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::Command;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("project root should be two ancestors above CARGO_MANIFEST_DIR")
        .to_path_buf()
}

fn helper_path() -> PathBuf {
    project_root().join("bin").join("ensure-cdp-9222")
}

#[test]
fn ut_ehlpr_001_helper_exists() {
    assert!(
        helper_path().exists(),
        "ensure-cdp-9222 helper missing at {}",
        helper_path().display()
    );
}

#[test]
fn ut_ehlpr_002_helper_is_executable() {
    use std::os::unix::fs::PermissionsExt;
    let metadata = std::fs::metadata(helper_path()).expect("stat helper");
    let mode = metadata.permissions().mode();
    assert!(
        mode & 0o111 != 0,
        "ensure-cdp-9222 helper is not executable (mode = {mode:o})"
    );
}

#[test]
fn ut_ehlpr_003_helper_short_circuits_unsupported_os() {
    // Force the OSTYPE env override; helper should exit 2 (unsupported OS).
    // CDP_PORT=65530 ensures the initial reachability probe fails fast so the
    // helper reaches the OS check rather than short-circuiting on success
    // against a real Chrome that may be running on 9222.
    let output = Command::new(helper_path())
        .env("OSTYPE", "linux-gnu")
        .env("CDP_PORT", "65530")
        .output()
        .expect("spawn helper");
    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit 2 for unsupported OS; got {:?}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn ut_ehlpr_004_helper_emits_diagnostic_json_on_failure() {
    let output = Command::new(helper_path())
        .env("OSTYPE", "linux-gnu")
        .env("CDP_PORT", "65531")
        .output()
        .expect("spawn helper");
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Diagnostic JSON has helper, status, port, message, ts keys; check the
    // stable ones. Don't assert on `ts` since that's a timestamp.
    assert!(
        stderr.contains(r#""helper":"ensure-cdp-9222""#),
        "missing helper key in diagnostic: {stderr}"
    );
    assert!(
        stderr.contains(r#""status":"unsupported_os""#),
        "missing status key in diagnostic: {stderr}"
    );
}

// ut_ehlpr_005 — when CDP IS up, helper exits 0 in <5s. Requires a real CDP
// browser running, so gated by `#[ignore]` to keep CI green without one.
// Run locally: `cargo test --test cdp_ensure_helper -- --ignored`.
#[test]
#[ignore = "requires live CDP browser on localhost:9222"]
fn ut_ehlpr_005_helper_fast_path_when_cdp_up() {
    use std::time::Instant;
    let start = Instant::now();
    let output = Command::new(helper_path()).output().expect("spawn helper");
    let elapsed = start.elapsed();
    assert!(
        output.status.success(),
        "helper failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        elapsed.as_secs() < 5,
        "helper too slow on fast path: {elapsed:?}"
    );
}
