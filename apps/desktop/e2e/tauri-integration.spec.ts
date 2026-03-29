import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

import { test } from '@playwright/test';

const REPO_ROOT = path.resolve(fileURLToPath(new URL('../../..', import.meta.url)));

function runNativeCase(caseName: string): void {
  execFileSync(
    'cargo',
    ['test', '-p', 'procintel-desktop', '--test', 'native_shell', '--', '--exact', caseName],
    {
      cwd: REPO_ROOT,
      stdio: 'pipe',
      env: {
        ...process.env,
        CARGO_TERM_COLOR: 'never',
      },
    },
  );
}

test.describe.configure({ mode: 'serial' });

test('TAU-INT-001 Quit UI leaves the daemon-owned runtime surface intact', async () => {
  runNativeCase('tau_int_001_quit_ui_records_native_exit_without_stopping_daemon');
});

test('TAU-INT-002 Stop background daemon and quit is a distinct desktop action', async () => {
  runNativeCase('tau_int_002_stop_background_daemon_and_quit_records_native_stop_then_exit');
});

test('TAU-INT-003 auto-launch bootstrap can still resolve to full control', async () => {
  runNativeCase('tau_int_003_startup_autolaunch_reaches_full_control_from_native_bootstrap');
});

test('TAU-INT-004 readable storage plus unavailable daemon maps to browse-only', async () => {
  runNativeCase('tau_int_004_storage_ready_daemon_unavailable_maps_to_browse_only');
});

test('TAU-INT-005 incompatible native bootstrap renders the mismatch mode', async () => {
  runNativeCase('tau_int_005_schema_mismatch_maps_to_incompatible');
});

test('TAU-INT-006 unreadable or repairing storage renders recovery-required mode', async () => {
  runNativeCase('tau_int_006_unreadable_storage_maps_to_recovery_required');
});

test('TAU-INT-007 degraded daemon maps to control-limited mode', async () => {
  runNativeCase('tau_int_007_degraded_daemon_maps_to_control_limited');
});

test('TAU-INT-008 the five-mode matrix stays total and non-overlapping', async () => {
  runNativeCase('tau_int_008_mode_matrix_stays_total_and_non_overlapping_in_native_bootstrap');
});

test('TAU-INT-009 same-daemon reconnect can retain the instance id', async () => {
  runNativeCase('tau_int_009_same_daemon_reconnect_retains_instance_id');
});

test('TAU-INT-010 changed instance ids can invalidate live resume assumptions', async () => {
  runNativeCase('tau_int_010_changed_instance_ids_invalidate_live_resume');
});

test('TAU-INT-011 native and browser payload shapes stay app-level compatible', async () => {
  runNativeCase('tau_int_011_native_payload_shapes_match_app_contracts');
});

test('TAU-INT-012 the native harness avoids the dev bridge control path entirely', async () => {
  runNativeCase('tau_int_012_native_ipc_bootstrap_path_works_without_browser_dev_bridge');
});
