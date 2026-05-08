export type Json =
  | string
  | number
  | boolean
  | null
  | { [key: string]: Json | undefined }
  | Json[];

export type SyncHealth = 'healthy' | 'degraded' | 'offline';

export type OperatorDisposition =
  | 'pending'
  | 'watching'
  | 'approved'
  | 'rejected';

export interface ProcurementProcessRow {
  id: string;
  canonical_ocid: string | null;
  primary_notice_id: string | null;
  canonical_process_id: string | null;
  current_stage: string;
  buyer_name: string | null;
  title: string;
  latest_deadline_at: string | null;
  latest_published_at: string;
  latest_updated_at: string;
  owner_id: string;
  row_version: number;
  last_local_seq: number | null;
  stage_run_id: string | null;
  updated_at: string;
  created_at: string;
}

export interface WorkflowJobSummaryRow {
  job_id: string;
  org_id: string;
  job_kind: string;
  status: string;
  skill_path: string | null;
  started_at: string | null;
  completed_at: string | null;
  error_type: string | null;
  owner_id: string;
  row_version: number;
  last_local_seq: number | null;
  stage_run_id: string | null;
  updated_at: string;
  created_at: string;
}

export interface ErrorManifestSummaryRow {
  manifest_id: string;
  org_id: string;
  job_id: string | null;
  error_type: string;
  error_code: string | null;
  incident_signature: string | null;
  owner_id: string;
  row_version: number;
  last_local_seq: number | null;
  stage_run_id: string | null;
  updated_at: string;
  created_at: string;
}

export interface ValidationVerdictSummaryRow {
  verdict_id: string;
  org_id: string;
  worker_job_id: string;
  verdict: string;
  basis_sufficiency: string | null;
  enforcement_eligible: boolean;
  owner_id: string;
  row_version: number;
  last_local_seq: number | null;
  stage_run_id: string | null;
  updated_at: string;
  created_at: string;
}

export interface ViewerSyncStatusRow {
  org_id: string;
  sync_health: SyncHealth;
  last_push_success_at: string | null;
  last_push_attempt_at: string | null;
  last_acked_local_seq: number | null;
  pending_outbox_count: number;
  schema_epoch: number;
  projection_generation: number;
  daemon_version: string | null;
  updated_at: string;
}

export interface OperatorDecisionSummaryRow {
  process_id: string;
  org_id: string;
  disposition: OperatorDisposition;
  decision_stale: boolean;
  reviewed_at_stage_run_id: string | null;
  owner_id: string;
  row_version: number;
  last_local_seq: number | null;
  stage_run_id: string | null;
  updated_at: string;
  created_at: string;
}

export interface SharedCompanyProfileRow {
  id: string;
  org_id: string;
  company_name: string;
  registration_number: string | null;
  vat_number: string | null;
  address_json: Json | null;
  contact_json: Json | null;
  bid_profile_json: Json | null;
  remote_change_seq: number;
  remote_revision: number;
  deleted_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface SharedSourceConfigRow {
  id: string;
  org_id: string;
  source_id: string;
  source_type: 'procurement' | 'supplier';
  display_name: string;
  enabled: boolean;
  skill_path: string;
  schedule_cron: string | null;
  config_json: Json | null;
  remote_change_seq: number;
  remote_revision: number;
  deleted_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface OperatorDecisionHeadRow {
  id: string;
  org_id: string;
  process_id: string;
  disposition: OperatorDisposition;
  reviewed_basis_hash: string | null;
  reviewed_manifest_hash: string | null;
  reviewed_at_stage_run_id: string | null;
  decision_stale: boolean;
  operator_id: string | null;
  command_id: string | null;
  remote_change_seq: number;
  remote_revision: number;
  deleted_at: string | null;
  created_at: string;
  updated_at: string;
}

export type ReadableProjectionTable =
  | 'procurement_processes'
  | 'workflow_job_summaries'
  | 'error_manifest_summaries'
  | 'validation_verdict_summaries'
  | 'viewer_sync_status'
  | 'operator_decision_summaries';

export type RemoteOwnedTable =
  | 'shared_company_profile'
  | 'shared_source_configs'
  | 'operator_decision_heads';

export type MutationRpcName = 'apply_push_batch' | 'update_operator_decision';

type ReadOnlyTable<Row> = {
  Row: Row;
  Insert: never;
  Update: never;
  Relationships: [];
};

export interface SharedVisibilityDatabase {
  public: {
    Tables: {
      procurement_processes: ReadOnlyTable<ProcurementProcessRow>;
      workflow_job_summaries: ReadOnlyTable<WorkflowJobSummaryRow>;
      error_manifest_summaries: ReadOnlyTable<ErrorManifestSummaryRow>;
      validation_verdict_summaries: ReadOnlyTable<ValidationVerdictSummaryRow>;
      viewer_sync_status: ReadOnlyTable<ViewerSyncStatusRow>;
      operator_decision_summaries: ReadOnlyTable<OperatorDecisionSummaryRow>;
      shared_company_profile: ReadOnlyTable<SharedCompanyProfileRow>;
      shared_source_configs: ReadOnlyTable<SharedSourceConfigRow>;
      operator_decision_heads: ReadOnlyTable<OperatorDecisionHeadRow>;
    };
    Views: Record<string, never>;
    Functions: {
      apply_push_batch: {
        Args: { batch_json: Json; writer_epoch: number };
        Returns: Json;
      };
      update_operator_decision: {
        Args: {
          process_id: string;
          disposition: OperatorDisposition;
          base_revision: number;
          command_id: string;
        };
        Returns: Json;
      };
    };
    Enums: Record<string, never>;
    CompositeTypes: Record<string, never>;
  };
}

export type TableRow<TableName extends keyof SharedVisibilityDatabase['public']['Tables']> =
  SharedVisibilityDatabase['public']['Tables'][TableName]['Row'];

export const READABLE_PROJECTION_TABLES = [
  'procurement_processes',
  'workflow_job_summaries',
  'error_manifest_summaries',
  'validation_verdict_summaries',
  'viewer_sync_status',
  'operator_decision_summaries',
] as const satisfies readonly ReadableProjectionTable[];

export const REMOTE_OWNED_TABLES = [
  'shared_company_profile',
  'shared_source_configs',
  'operator_decision_heads',
] as const satisfies readonly RemoteOwnedTable[];

export const MUTATION_RPC_NAMES = [
  'apply_push_batch',
  'update_operator_decision',
] as const satisfies readonly MutationRpcName[];
