import { createClient, type SupabaseClient } from '@supabase/supabase-js';
import type {
  ProcurementProcessRow,
  SharedVisibilityDatabase,
  ViewerSyncStatusRow,
  WorkflowJobSummaryRow,
} from '@project/contracts-schema';

export type ViewerSupabaseClient = SupabaseClient<SharedVisibilityDatabase>;

export interface ViewerSupabaseConfig {
  url: string;
  anonKey: string;
}

type ViewerEnv = Record<string, string | undefined>;

let cachedViewerClient: ViewerSupabaseClient | null = null;

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
