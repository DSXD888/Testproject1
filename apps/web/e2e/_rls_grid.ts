export const WEB_RLS_ROLES = ['viewer', 'operator', 'admin', 'daemon'] as const;

export type WebRlsRole = (typeof WEB_RLS_ROLES)[number];

export const WEB_RLS_CATEGORIES = [
  'bid_state_projection_summary',
  'bid_scope_payload',
  'run_manifest_body_and_forensic_paths',
  'bid_mutation_controls_and_rpcs',
  'company_profile_fields',
] as const;

export type WebRlsCategory = (typeof WEB_RLS_CATEGORIES)[number];

export type WebRlsAccess = 'allow_summary' | 'deny_web_projection' | 'deny_mutation';

export interface WebRlsGridCell {
  role: WebRlsRole;
  category: WebRlsCategory;
  access: WebRlsAccess;
  table: string;
  method: 'GET' | 'PATCH' | 'POST';
  columns: readonly string[];
}

export interface RoleScopedSelectAttempt extends WebRlsGridCell {
  processId: string;
  path: string;
  requestRoleHeader: WebRlsRole;
}

export interface RoleScopedAttemptResult {
  attempt: RoleScopedSelectAttempt;
  status: number;
  body: string;
}

export const BID_WORKFLOW_SUMMARY_COLUMNS = [
  'process_id',
  'bid_state',
  'review_required',
  'bid_job_id',
  'updated_at',
] as const;

export const BID_WORKFLOW_SENSITIVE_COLUMNS = [
  'bid_scope_json',
  'started_at',
  'completed_at',
  'owner_id',
  'last_local_seq',
  'stage_run_id',
] as const;

export const WEB_RLS_SENSITIVE_TOKENS = [
  'bid_scope_json',
  'company_profile',
  'shared_company_profile',
  'run_manifest',
  'manifest_body',
  'portal_artifact_path',
  'forensic_pty_bytes_path',
  'screenshot_path',
  'local_document_path',
  'TestCompanyName-Sentinel-L6-2026',
  'sales@sorelon.com',
] as const;

const CATEGORY_REQUESTS = {
  bid_state_projection_summary: {
    access: 'allow_summary',
    table: 'bid_workflow_states',
    method: 'GET',
    columns: BID_WORKFLOW_SUMMARY_COLUMNS,
  },
  bid_scope_payload: {
    access: 'deny_web_projection',
    table: 'bid_workflow_states',
    method: 'GET',
    columns: BID_WORKFLOW_SENSITIVE_COLUMNS,
  },
  run_manifest_body_and_forensic_paths: {
    access: 'deny_web_projection',
    table: 'bid_workflow_states',
    method: 'GET',
    columns: [
      'run_manifest_body',
      'run_manifest_path',
      'forensic_pty_bytes_path',
      'portal_artifact_path',
      'screenshot_path',
    ],
  },
  bid_mutation_controls_and_rpcs: {
    access: 'deny_mutation',
    table: 'bid_workflow_states',
    method: 'PATCH',
    columns: ['bid_state'],
  },
  company_profile_fields: {
    access: 'deny_web_projection',
    table: 'shared_company_profile',
    method: 'GET',
    columns: [
      'company_name',
      'registration_number',
      'vat_number',
      'address_json',
      'contact_json',
      'bid_profile_json',
    ],
  },
} as const satisfies Record<
  WebRlsCategory,
  {
    access: WebRlsAccess;
    table: string;
    method: 'GET' | 'PATCH' | 'POST';
    columns: readonly string[];
  }
>;

export function buildWebRlsGrid(): readonly WebRlsGridCell[] {
  return WEB_RLS_ROLES.flatMap((role) =>
    WEB_RLS_CATEGORIES.map((category) => ({
      role,
      category,
      ...CATEGORY_REQUESTS[category],
    })),
  );
}

export function buildRoleScopedSelectAttempts(
  processId: string,
): readonly RoleScopedSelectAttempt[] {
  return buildWebRlsGrid().map((cell) => ({
    ...cell,
    processId,
    requestRoleHeader: cell.role,
    path:
      cell.method === 'PATCH'
        ? `/rest/v1/${cell.table}?process_id=eq.${encodeURIComponent(processId)}`
        : `/rest/v1/${cell.table}?select=${encodeURIComponent(
            cell.columns.join(','),
          )}&process_id=eq.${encodeURIComponent(processId)}`,
  }));
}

export async function issueRoleScopedSelectAttempts(
  processId: string,
  requester: (attempt: RoleScopedSelectAttempt) => Promise<RoleScopedAttemptResult>,
): Promise<readonly RoleScopedAttemptResult[]> {
  const attempts = buildRoleScopedSelectAttempts(processId);

  return Promise.all(attempts.map((attempt) => requester(attempt)));
}
