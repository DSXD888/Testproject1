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

export type PermitResponsibility =
  | 'authority_provided'
  | 'contractor_provided'
  | 'shared'
  | 'unknown';

export type GoodsValueShareBasis = 'contract_value' | 'cost_proxy';

export type SupplierQuoteClass = 'installer_quote' | (string & {});

export type MaintenanceMarginSignal = string & {};

export type MaintenanceSchedule = {
  visits_per_year: number | null;
  term_years: number | null;
  visit_scope: string | null;
} & { [key: string]: Json | undefined };

export type InstallationRequirement = {
  requires_installation: boolean;
  requires_maintenance: boolean;
  installation_scope: string | null;
  maintenance_schedule: MaintenanceSchedule | string | null;
  site_access: string | null;
  at_height_m: number | null;
  uk_region: string | null;
  estimated_units: number | null;
  equipment_hint: string | null;
  permit_responsibility: PermitResponsibility;
  public_realm: boolean | null;
  lane_rental_zone: boolean | null;
} & { [key: string]: Json | undefined };

export type ProductCandidateCriticalAttributes = {
  installation_requirement?: InstallationRequirement;
} & { [key: string]: Json | undefined };

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

export interface ProductCandidateRow {
  id: string;
  process_id: string;
  notice_id: string;
  lot_id: string;
  parent_candidate_id: string | null;
  candidate_key: string | null;
  candidate_role: string;
  name: string;
  candidate_type: string;
  match_archetype: string;
  head_noun: string;
  quantity: number | null;
  quantity_unit: string | null;
  specification_text: string | null;
  estimated_unit_weight_kg: number | null;
  estimated_unit_volume_m3: number | null;
  candidate_value_share_bps: number | null;
  packing_hint: string | null;
  handling_restrictions_json: Json | null;
  material: string | null;
  brand_model: string | null;
  critical_attributes_json: ProductCandidateCriticalAttributes | Json | null;
  numeric_specs_json: Json | null;
  standards_json: Json | null;
  preferred_terms_json: Json | null;
  excluded_terms_json: Json | null;
  source_evidence_json: Json | null;
  quoteability_score_bps: number | null;
  recall_risk_score_bps: number | null;
  identity_confidence_bps: number | null;
  spec_confidence_bps: number | null;
  quantity_confidence_bps: number | null;
  owner_id: string;
  row_version: number;
  last_local_seq: number | null;
  stage_run_id: string | null;
  updated_at: string;
  created_at: string;
}

export interface SupplierQuoteRow {
  id: string;
  process_id: string;
  notice_id: string;
  product_candidate_id: string | null;
  search_attempt_id: string | null;
  product_name: string;
  region: string;
  quote_class: SupplierQuoteClass;
  source_name: string;
  source_url: string | null;
  source_listing_id: string | null;
  ship_from_country: string | null;
  country_of_origin: string | null;
  country_of_dispatch: string | null;
  incoterm: string | null;
  quote_cost_boundary: string | null;
  quoted_quantity: number | null;
  quoted_quantity_unit: string | null;
  unit_price_original_minor: number;
  currency: string;
  unit_price_gbp_pence: number;
  offer_total_original_minor: number | null;
  offer_total_gbp_pence: number | null;
  price_basis_qty: number | null;
  price_basis_unit: string | null;
  minimum_order_quantity: number | null;
  lead_time_days: number | null;
  includes_shipping: boolean;
  price_signal_role: string;
  listing_pack_qty: number | null;
  listing_pack_unit: string | null;
  normalized_unit_basis: string | null;
  is_used_or_refurbished: boolean;
  is_ready_to_ship: boolean | null;
  vat_inclusive: boolean | null;
  compliance_evidence_tier: string | null;
  raw_match_score_bps: number | null;
  match_confidence_bps: number | null;
  quote_usability_confidence_bps: number | null;
  shipping_basis_confidence_bps: number | null;
  quote_realism_confidence_bps: number | null;
  negotiation_uplift_bps: number | null;
  evidence_states_json: Json | null;
  query_wave_number: number | null;
  evidence: string;
  valid_until: string | null;
  owner_id: string;
  row_version: number;
  last_local_seq: number | null;
  stage_run_id: string | null;
  updated_at: string;
  created_at: string;
}

export interface MaintenancePeriodEstimateRow {
  id: string;
  process_id: string;
  notice_id: string;
  product_candidate_id: string;
  landed_cost_estimate_id: string;
  supplier_quote_id: string | null;
  visit_scope: string | null;
  visits_per_year: number;
  term_years: number;
  per_visit_labour_cost_gbp_pence: number;
  annualised_opex_gbp_pence: number;
  total_visible_term_opex_gbp_pence: number;
  maintenance_confidence_bps: number;
  margin_signal: MaintenanceMarginSignal | null;
  warnings: Json;
  evidence_json: Json;
  owner_id: string;
  row_version: number;
  last_local_seq: number | null;
  stage_run_id: string | null;
  updated_at: string;
  created_at: string;
}

export interface CandidatePricingAssessmentRow {
  id: string;
  process_id: string;
  notice_id: string;
  product_candidate_id: string;
  selected_quote_id: string;
  selected_quote_row_version: number;
  quote_cost_boundary: string;
  revenue_basis_type: string;
  expected_addressable_revenue_ex_vat_pence: number;
  revenue_confidence_bps: number;
  gross_margin_bps: number;
  gross_estimate_confidence_bps: number;
  identity_gate_passed: boolean;
  usability_gate_passed: boolean;
  pricing_input_confidence_bps: number;
  commercial_basis_kind: string;
  normal_quote_coverage: number;
  uk_quote_id: string | null;
  china_import_quote_id: string | null;
  benchmark_quote_id: string | null;
  selected_supply_quote_id: string | null;
  supplier_spread_bps: number | null;
  fallback_discount_bps: number | null;
  missing_required_roles_json: Json;
  coverage_evidence_json: Json;
  owner_id: string;
  row_version: number;
  last_local_seq: number | null;
  stage_run_id: string | null;
  updated_at: string;
  created_at: string;
}

export interface LandedCostEstimateRow {
  id: string;
  process_id: string;
  notice_id: string;
  product_candidate_id: string;
  supplier_quote_id: string;
  shipping_mode: string;
  mode_confidence_bps: number;
  packing_scenario: string | null;
  declarable_commodity_code_10: string | null;
  candidate_commodity_codes_json: Json | null;
  goods_landed_cost_gbp_pence: number | null;
  installation_labour_cost_gbp_pence: number | null;
  installation_labour_low_gbp_pence: number | null;
  installation_labour_high_gbp_pence: number | null;
  installation_labour_confidence_bps: number | null;
  permit_traffic_mgmt_cost_gbp_pence: number | null;
  permit_traffic_mgmt_low_gbp_pence: number | null;
  permit_traffic_mgmt_high_gbp_pence: number | null;
  permit_responsibility: PermitResponsibility | null;
  economic_landed_cost_gbp_pence: number;
  net_margin_low_bps: number;
  net_margin_base_bps: number;
  net_margin_high_bps: number;
  decision_grade: string;
  confidence_bps: number;
  warnings: Json;
  blockers: Json;
  owner_id: string;
  row_version: number;
  last_local_seq: number | null;
  stage_run_id: string | null;
  updated_at: string;
  created_at: string;
}

export interface ManifestRow {
  id: string;
  process_id: string;
  markdown: string;
  artifact_hash: string | null;
  artifact_locator: string | null;
  stoplight: string | null;
  manifest_basis_hash: string | null;
  suggested_action: string | null;
  suggested_action_reasoning: string | null;
  clarification_prompts_json: Json;
  manifest_template_version: string;
  candidate_count: number;
  priced_candidate_count: number;
  clarification_prompt_count: number;
  owner_id: string;
  row_version: number;
  last_local_seq: number | null;
  stage_run_id: string | null;
  updated_at: string;
  created_at: string;
}

export interface PipelineAssessmentRow {
  process_id: string;
  pipeline_state: string;
  review_required: boolean;
  availability_status: string;
  validation_state: string;
  gross_margin_bps: number | null;
  net_margin_bps: number | null;
  priced_coverage_bps: number | null;
  landed_cost_coverage_bps: number | null;
  net_qualified_coverage_bps: number | null;
  priced_value_coverage_bps: number | null;
  landed_cost_value_coverage_bps: number | null;
  net_qualified_value_coverage_bps: number | null;
  goods_value_share_bps: number | null;
  goods_value_share_basis: GoodsValueShareBasis | null;
  installation_labour_coverage_bps: number | null;
  installation_labour_value_coverage_bps: number | null;
  permit_traffic_mgmt_coverage_bps: number | null;
  permit_traffic_mgmt_value_coverage_bps: number | null;
  unresolved_mandatory_lots: number;
  decision_basis_hash: string | null;
  review_stage: string | null;
  review_reason: string | null;
  assessed_at: string;
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
  | 'product_candidates'
  | 'supplier_quotes'
  | 'maintenance_period_estimates'
  | 'candidate_pricing_assessments'
  | 'landed_cost_estimates'
  | 'manifests'
  | 'pipeline_assessments'
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
      product_candidates: ReadOnlyTable<ProductCandidateRow>;
      supplier_quotes: ReadOnlyTable<SupplierQuoteRow>;
      maintenance_period_estimates: ReadOnlyTable<MaintenancePeriodEstimateRow>;
      candidate_pricing_assessments: ReadOnlyTable<CandidatePricingAssessmentRow>;
      landed_cost_estimates: ReadOnlyTable<LandedCostEstimateRow>;
      manifests: ReadOnlyTable<ManifestRow>;
      pipeline_assessments: ReadOnlyTable<PipelineAssessmentRow>;
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
  'product_candidates',
  'supplier_quotes',
  'maintenance_period_estimates',
  'candidate_pricing_assessments',
  'landed_cost_estimates',
  'manifests',
  'pipeline_assessments',
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
