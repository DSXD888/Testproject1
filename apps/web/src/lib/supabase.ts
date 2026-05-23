import { createClient, type SupabaseClient } from '@supabase/supabase-js';
import type {
  CandidatePricingAssessmentRow,
  ErrorManifestSummaryRow,
  LandedCostEstimateRow,
  ManifestRow,
  OperatorDecisionSummaryRow,
  PipelineAssessmentRow,
  ProcurementProcessRow,
  ProductCandidateRow,
  SharedVisibilityDatabase,
  SupplierQuoteRow,
  ValidationVerdictSummaryRow,
  ViewerSyncStatusRow,
  WorkflowJobSummaryRow,
} from '@project/contracts-schema';

export type ViewerSupabaseClient = SupabaseClient<SharedVisibilityDatabase>;

export interface ViewerSupabaseConfig {
  url: string;
  anonKey: string;
}

export interface ContractListCursor {
  latestUpdatedAt: string;
  id: string;
}

export interface ContractListPage {
  rows: ProcurementProcessRow[];
  nextCursor: ContractListCursor | null;
  hasMore: boolean;
}

export interface ListContractSummariesPageOptions {
  cursor?: ContractListCursor | null;
  limit?: number;
}

export interface ContractDetailSnapshot {
  process: ProcurementProcessRow;
  pipelineAssessment: PipelineAssessmentRow | null;
  candidates: ProductCandidateRow[];
  quotes: SupplierQuoteRow[];
  pricingAssessments: CandidatePricingAssessmentRow[];
  landedCosts: LandedCostEstimateRow[];
  manifest: ManifestRow | null;
  bidWorkflowSummary: BidWorkflowSummaryRow | null;
  decisionSummary: OperatorDecisionSummaryRow | null;
  errorSummaries: ErrorManifestSummaryRow[];
  workflowJobs: WorkflowJobSummaryRow[];
  validationVerdicts: ValidationVerdictSummaryRow[];
  loadedAt: string;
}

export interface BidWorkflowSummaryRow {
  process_id: string;
  bid_state: string;
  review_required: boolean;
  bid_job_id: string | null;
  updated_at: string;
  bid_state_summary: string;
  error_manifest_link: string | null;
}

export interface GetContractDetailOptions {
  limit?: number;
}

export interface StatusPageSnapshot {
  syncStatus: ViewerSyncStatusRow | null;
  workflowJobs: WorkflowJobSummaryRow[];
  errorSummaries: ErrorManifestSummaryRow[];
  loadedAt: string;
}

export interface GetStatusPageSnapshotOptions {
  limit?: number;
}

type ViewerEnv = Record<string, string | undefined>;

let cachedViewerClient: ViewerSupabaseClient | null = null;

const BID_WORKFLOW_SUMMARY_COLUMNS =
  'process_id,bid_state,review_required,bid_job_id,updated_at' as const;

interface SupabaseSingleResult<Row> {
  data: Row | null;
  error: { message: string } | null;
}

interface BidWorkflowSummarySelectBuilder {
  eq(column: 'process_id', value: string): {
    maybeSingle(): Promise<SupabaseSingleResult<BidWorkflowProjectionRow>>;
  };
}

interface BidWorkflowSummaryTable {
  select(columns: typeof BID_WORKFLOW_SUMMARY_COLUMNS): BidWorkflowSummarySelectBuilder;
}

interface BidWorkflowProjectionClient {
  from(table: 'bid_workflow_states'): BidWorkflowSummaryTable;
}

type BidWorkflowProjectionRow = Pick<
  BidWorkflowSummaryRow,
  'process_id' | 'bid_state' | 'review_required' | 'bid_job_id' | 'updated_at'
>;

export function getViewerSupabaseConfig(env: ViewerEnv = process.env): ViewerSupabaseConfig {
  const url = env.NEXT_PUBLIC_SUPABASE_URL;
  const anonKey = env.NEXT_PUBLIC_SUPABASE_ANON_KEY;

  if (!url || !anonKey) {
    throw new Error('Viewer Supabase URL and anon key are required');
  }

  return { url, anonKey };
}

export function createViewerSupabaseClient(
  config: ViewerSupabaseConfig = getViewerSupabaseConfig(),
): ViewerSupabaseClient {
  return createClient<SharedVisibilityDatabase>(config.url, config.anonKey, {
    auth: {
      autoRefreshToken: false,
      detectSessionInUrl: false,
      persistSession: false,
    },
    global: {
      headers: {
        'x-client-info': 'l4-web-viewer',
      },
    },
  });
}

export function getViewerSupabaseClient(): ViewerSupabaseClient {
  cachedViewerClient ??= createViewerSupabaseClient();
  return cachedViewerClient;
}

function requireRows<Row>(data: Row[] | null, error: { message: string } | null): Row[] {
  if (error) {
    throw new Error(error.message);
  }

  return data ?? [];
}

function requireSingle<Row>(data: Row | null, error: { message: string } | null): Row | null {
  if (error) {
    throw new Error(error.message);
  }

  return data;
}

export async function listContractSummaries(
  client: ViewerSupabaseClient = getViewerSupabaseClient(),
  limit = 50,
): Promise<ProcurementProcessRow[]> {
  const { data, error } = await client
    .from('procurement_processes')
    .select('*')
    .order('latest_updated_at', { ascending: false })
    .order('id', { ascending: false })
    .limit(limit);

  return requireRows<ProcurementProcessRow>(data, error);
}

export async function listContractSummariesPage(
  options: ListContractSummariesPageOptions = {},
  client: ViewerSupabaseClient = getViewerSupabaseClient(),
): Promise<ContractListPage> {
  const limit = Math.max(1, Math.min(options.limit ?? 25, 50));
  let query = client
    .from('procurement_processes')
    .select('*')
    .order('latest_updated_at', { ascending: false })
    .order('id', { ascending: false })
    .limit(limit + 1);

  if (options.cursor) {
    query = query.or(
      [
        `latest_updated_at.lt.${options.cursor.latestUpdatedAt}`,
        `and(latest_updated_at.eq.${options.cursor.latestUpdatedAt},id.lt.${options.cursor.id})`,
      ].join(','),
    );
  }

  const { data, error } = await query;
  const rows = requireRows<ProcurementProcessRow>(data, error);
  const visibleRows = rows.slice(0, limit);
  const lastVisibleRow = visibleRows.at(-1);

  return {
    rows: visibleRows,
    hasMore: rows.length > limit,
    nextCursor:
      rows.length > limit && lastVisibleRow
        ? {
            latestUpdatedAt: lastVisibleRow.latest_updated_at,
            id: lastVisibleRow.id,
          }
        : null,
  };
}

export async function getContractSummary(
  processId: string,
  client: ViewerSupabaseClient = getViewerSupabaseClient(),
): Promise<ProcurementProcessRow | null> {
  const { data, error } = await client
    .from('procurement_processes')
    .select('*')
    .eq('id', processId)
    .maybeSingle();

  return requireSingle<ProcurementProcessRow>(data, error);
}

export async function getContractDetail(
  processId: string,
  orgId: string,
  options: GetContractDetailOptions = {},
  client: ViewerSupabaseClient = getViewerSupabaseClient(),
): Promise<ContractDetailSnapshot | null> {
  const limit = Math.max(1, Math.min(options.limit ?? 50, 100));
  const [
    process,
    pipelineAssessment,
    candidates,
    quotes,
    pricingAssessments,
    landedCosts,
    manifest,
    bidWorkflowSummary,
    decisionSummary,
    errorSummaries,
    workflowJobs,
    validationVerdicts,
  ] = await Promise.all([
    getContractSummary(processId, client),
    getPipelineAssessment(processId, client),
    listProductCandidates(processId, limit, client),
    listSupplierQuotes(processId, limit, client),
    listCandidatePricingAssessments(processId, limit, client),
    listLandedCostEstimates(processId, limit, client),
    getLatestManifest(processId, client),
    loadBidWorkflowSummary(processId, client),
    getOperatorDecisionSummary(processId, client),
    listErrorManifestSummaries(orgId, limit, client),
    listWorkflowJobSummaries(orgId, client, limit),
    listValidationVerdictSummaries(orgId, limit, client),
  ]);

  if (!process) {
    return null;
  }

  return {
    process,
    pipelineAssessment,
    candidates,
    quotes,
    pricingAssessments,
    landedCosts,
    manifest,
    bidWorkflowSummary,
    decisionSummary,
    errorSummaries,
    workflowJobs,
    validationVerdicts,
    loadedAt: new Date().toISOString(),
  };
}

export async function getStatusPageSnapshot(
  orgId: string,
  options: GetStatusPageSnapshotOptions = {},
  client: ViewerSupabaseClient = getViewerSupabaseClient(),
): Promise<StatusPageSnapshot> {
  const limit = Math.max(1, Math.min(options.limit ?? 50, 100));
  const [syncStatus, workflowJobs, errorSummaries] = await Promise.all([
    getViewerSyncStatus(orgId, client),
    listWorkflowJobSummaries(orgId, client, limit),
    listErrorManifestSummaries(orgId, limit, client),
  ]);

  return {
    syncStatus,
    workflowJobs,
    errorSummaries,
    loadedAt: new Date().toISOString(),
  };
}

async function listProductCandidates(
  processId: string,
  limit: number,
  client: ViewerSupabaseClient,
): Promise<ProductCandidateRow[]> {
  const { data, error } = await client
    .from('product_candidates')
    .select('*')
    .eq('process_id', processId)
    .order('updated_at', { ascending: false })
    .limit(limit);

  return requireRows<ProductCandidateRow>(data, error);
}

async function listSupplierQuotes(
  processId: string,
  limit: number,
  client: ViewerSupabaseClient,
): Promise<SupplierQuoteRow[]> {
  const { data, error } = await client
    .from('supplier_quotes')
    .select('*')
    .eq('process_id', processId)
    .order('updated_at', { ascending: false })
    .limit(limit);

  return requireRows<SupplierQuoteRow>(data, error);
}

async function listCandidatePricingAssessments(
  processId: string,
  limit: number,
  client: ViewerSupabaseClient,
): Promise<CandidatePricingAssessmentRow[]> {
  const { data, error } = await client
    .from('candidate_pricing_assessments')
    .select('*')
    .eq('process_id', processId)
    .order('updated_at', { ascending: false })
    .limit(limit);

  return requireRows<CandidatePricingAssessmentRow>(data, error);
}

async function listLandedCostEstimates(
  processId: string,
  limit: number,
  client: ViewerSupabaseClient,
): Promise<LandedCostEstimateRow[]> {
  const { data, error } = await client
    .from('landed_cost_estimates')
    .select('*')
    .eq('process_id', processId)
    .order('updated_at', { ascending: false })
    .limit(limit);

  return requireRows<LandedCostEstimateRow>(data, error);
}

async function getLatestManifest(
  processId: string,
  client: ViewerSupabaseClient,
): Promise<ManifestRow | null> {
  const { data, error } = await client
    .from('manifests')
    .select('*')
    .eq('process_id', processId)
    .order('updated_at', { ascending: false })
    .limit(1)
    .maybeSingle();

  return requireSingle<ManifestRow>(data, error);
}

async function getPipelineAssessment(
  processId: string,
  client: ViewerSupabaseClient,
): Promise<PipelineAssessmentRow | null> {
  const { data, error } = await client
    .from('pipeline_assessments')
    .select('*')
    .eq('process_id', processId)
    .maybeSingle();

  return requireSingle<PipelineAssessmentRow>(data, error);
}

async function getOperatorDecisionSummary(
  processId: string,
  client: ViewerSupabaseClient,
): Promise<OperatorDecisionSummaryRow | null> {
  const { data, error } = await client
    .from('operator_decision_summaries')
    .select('*')
    .eq('process_id', processId)
    .maybeSingle();

  return requireSingle<OperatorDecisionSummaryRow>(data, error);
}

export async function loadBidWorkflowSummary(
  processId: string,
  client: ViewerSupabaseClient = getViewerSupabaseClient(),
): Promise<BidWorkflowSummaryRow | null> {
  const bidWorkflowStates = (client as unknown as BidWorkflowProjectionClient).from(
    'bid_workflow_states',
  );
  const { data, error } = await bidWorkflowStates
    .select(BID_WORKFLOW_SUMMARY_COLUMNS)
    .eq('process_id', processId)
    .maybeSingle();

  const row = requireSingle<BidWorkflowProjectionRow>(data, error);

  if (!row) {
    return null;
  }

  return {
    ...row,
    bid_state_summary: row.review_required
      ? 'Operator review required in desktop'
      : 'Bid workflow state synced',
    error_manifest_link:
      row.bid_state === 'bid_failed'
        ? `/status?bidJob=${encodeURIComponent(row.bid_job_id ?? row.process_id)}#errors`
        : null,
  };
}

async function listErrorManifestSummaries(
  orgId: string,
  limit: number,
  client: ViewerSupabaseClient,
): Promise<ErrorManifestSummaryRow[]> {
  const { data, error } = await client
    .from('error_manifest_summaries')
    .select('*')
    .eq('org_id', orgId)
    .order('updated_at', { ascending: false })
    .limit(limit);

  return requireRows<ErrorManifestSummaryRow>(data, error);
}

async function listValidationVerdictSummaries(
  orgId: string,
  limit: number,
  client: ViewerSupabaseClient,
): Promise<ValidationVerdictSummaryRow[]> {
  const { data, error } = await client
    .from('validation_verdict_summaries')
    .select('*')
    .eq('org_id', orgId)
    .order('updated_at', { ascending: false })
    .limit(limit);

  return requireRows<ValidationVerdictSummaryRow>(data, error);
}

export async function getViewerSyncStatus(
  orgId: string,
  client: ViewerSupabaseClient = getViewerSupabaseClient(),
): Promise<ViewerSyncStatusRow | null> {
  const { data, error } = await client
    .from('viewer_sync_status')
    .select('*')
    .eq('org_id', orgId)
    .maybeSingle();

  return requireSingle<ViewerSyncStatusRow>(data, error);
}

export async function listWorkflowJobSummaries(
  orgId: string,
  client: ViewerSupabaseClient = getViewerSupabaseClient(),
  limit = 50,
): Promise<WorkflowJobSummaryRow[]> {
  const { data, error } = await client
    .from('workflow_job_summaries')
    .select('*')
    .eq('org_id', orgId)
    .order('updated_at', { ascending: false })
    .limit(limit);

  return requireRows<WorkflowJobSummaryRow>(data, error);
}
