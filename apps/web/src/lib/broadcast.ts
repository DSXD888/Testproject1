import {
  getViewerSupabaseClient,
  type ViewerSupabaseClient,
} from './supabase';

export type ContractChangeKind = 'assessment' | 'decision' | 'manifest' | 'error';

export type ProcessChangeKind =
  | 'quotes'
  | 'landed_cost'
  | 'manifest'
  | 'jobs'
  | 'errors'
  | 'canonical_process';

export type BroadcastSyncHealth = 'healthy' | 'degraded' | 'offline';

export type ContractsChangedUpdate = {
  event: 'contracts.changed';
  processIds: string[];
  changed: ContractChangeKind[];
  maxSeq: number;
};

export type ProcessUpdatedUpdate = {
  event: 'process.updated';
  processId: string;
  changed: ProcessChangeKind[];
  major: boolean;
  maxSeq: number;
};

export type SyncUpdatedUpdate = {
  event: 'sync.updated';
  syncHealth: BroadcastSyncHealth;
  lastAckedSeq: number;
  lastSuccessAt: string;
  projectionGeneration: number;
};

export type PortalUpdate =
  | ContractsChangedUpdate
  | ProcessUpdatedUpdate
  | SyncUpdatedUpdate;

export type BroadcastTopic =
  | `org:${string}:contracts`
  | `process:${string}:updates`
  | `org:${string}:status`
  | `org:${string}:errors`
  | `org:${string}:sync`;

type ViewerBroadcastChannel = ReturnType<ViewerSupabaseClient['channel']>;

export interface BroadcastSubscription {
  topics: readonly BroadcastTopic[];
  unsubscribe: () => Promise<void>;
}

export interface BroadcastHandlerContext {
  topic: BroadcastTopic;
}

export type BroadcastUpdateHandler<Update extends PortalUpdate = PortalUpdate> = (
  update: Update,
  context: BroadcastHandlerContext,
) => void;

export interface ContractsPageBroadcastHandlers {
  onContractsChanged?: BroadcastUpdateHandler<ContractsChangedUpdate>;
  onSyncUpdated?: BroadcastUpdateHandler<SyncUpdatedUpdate>;
  onUpdate?: BroadcastUpdateHandler;
}

export interface ContractDetailBroadcastHandlers {
  onProcessUpdated?: BroadcastUpdateHandler<ProcessUpdatedUpdate>;
  onSyncUpdated?: BroadcastUpdateHandler<SyncUpdatedUpdate>;
  onUpdate?: BroadcastUpdateHandler;
}

export interface StatusPageBroadcastHandlers {
  onSyncUpdated?: BroadcastUpdateHandler<SyncUpdatedUpdate>;
  onUpdate?: BroadcastUpdateHandler;
}

export interface PageBroadcastOptions {
  client?: ViewerSupabaseClient;
}

const CONTRACT_CHANGE_KINDS = [
  'assessment',
  'decision',
  'manifest',
  'error',
] as const satisfies readonly ContractChangeKind[];

const PROCESS_CHANGE_KINDS = [
  'quotes',
  'landed_cost',
  'manifest',
  'jobs',
  'errors',
  'canonical_process',
] as const satisfies readonly ProcessChangeKind[];

const SYNC_HEALTH_VALUES = [
  'healthy',
  'degraded',
  'offline',
] as const satisfies readonly BroadcastSyncHealth[];

export function contractsTopic(orgId: string): BroadcastTopic {
  return `org:${orgId}:contracts`;
}

export function processUpdatesTopic(processId: string): BroadcastTopic {
  return `process:${processId}:updates`;
}

export function statusTopic(orgId: string): BroadcastTopic {
  return `org:${orgId}:status`;
}

export function errorsTopic(orgId: string): BroadcastTopic {
  return `org:${orgId}:errors`;
}

export function syncTopic(orgId: string): BroadcastTopic {
  return `org:${orgId}:sync`;
}

export function contractsPageTopics(orgId: string): readonly BroadcastTopic[] {
  return [contractsTopic(orgId), syncTopic(orgId)];
}

export function contractDetailPageTopics(
  orgId: string,
  processId: string,
): readonly BroadcastTopic[] {
  return [processUpdatesTopic(processId), syncTopic(orgId)];
}

export function statusPageTopics(orgId: string): readonly BroadcastTopic[] {
  return [statusTopic(orgId), errorsTopic(orgId), syncTopic(orgId)];
}

export function isPortalUpdate(value: unknown): value is PortalUpdate {
  if (!isRecord(value) || typeof value.event !== 'string') {
    return false;
  }

  switch (value.event) {
    case 'contracts.changed':
      return (
        isStringArray(value.processIds) &&
        isAllowedStringArray(value.changed, CONTRACT_CHANGE_KINDS) &&
        isFiniteNumber(value.maxSeq)
      );
    case 'process.updated':
      return (
        typeof value.processId === 'string' &&
        isAllowedStringArray(value.changed, PROCESS_CHANGE_KINDS) &&
        typeof value.major === 'boolean' &&
        isFiniteNumber(value.maxSeq)
      );
    case 'sync.updated':
      return (
        isAllowedString(value.syncHealth, SYNC_HEALTH_VALUES) &&
        isFiniteNumber(value.lastAckedSeq) &&
        typeof value.lastSuccessAt === 'string' &&
        isFiniteNumber(value.projectionGeneration)
      );
    default:
      return false;
  }
}

export function subscribeToContractsPage(
  orgId: string,
  handlers: ContractsPageBroadcastHandlers,
  options: PageBroadcastOptions = {},
): BroadcastSubscription {
  return subscribeToPortalTopics(
    contractsPageTopics(orgId),
    (update, context) => {
      handlers.onUpdate?.(update, context);

      if (update.event === 'contracts.changed') {
        handlers.onContractsChanged?.(update, context);
      }

      if (update.event === 'sync.updated') {
        handlers.onSyncUpdated?.(update, context);
      }
    },
    options.client,
  );
}

export function subscribeToContractDetailPage(
  orgId: string,
  processId: string,
  handlers: ContractDetailBroadcastHandlers,
  options: PageBroadcastOptions = {},
): BroadcastSubscription {
  return subscribeToPortalTopics(
    contractDetailPageTopics(orgId, processId),
    (update, context) => {
      handlers.onUpdate?.(update, context);

      if (update.event === 'process.updated') {
        handlers.onProcessUpdated?.(update, context);
      }

      if (update.event === 'sync.updated') {
        handlers.onSyncUpdated?.(update, context);
      }
    },
    options.client,
  );
}

export function subscribeToStatusPage(
  orgId: string,
  handlers: StatusPageBroadcastHandlers,
  options: PageBroadcastOptions = {},
): BroadcastSubscription {
  return subscribeToPortalTopics(
    statusPageTopics(orgId),
    (update, context) => {
      handlers.onUpdate?.(update, context);

      if (update.event === 'sync.updated') {
        handlers.onSyncUpdated?.(update, context);
      }
    },
    options.client,
  );
}

function subscribeToPortalTopics(
  topics: readonly BroadcastTopic[],
  onUpdate: BroadcastUpdateHandler,
  client: ViewerSupabaseClient = getViewerSupabaseClient(),
): BroadcastSubscription {
  const uniqueTopics = [...new Set(topics)];
  const channels = uniqueTopics.map((topic) =>
    subscribeToTopic(client, topic, onUpdate),
  );

  return {
    topics: uniqueTopics,
    unsubscribe: async () => {
      await Promise.all(channels.map((channel) => client.removeChannel(channel)));
    },
  };
}

function subscribeToTopic(
  client: ViewerSupabaseClient,
  topic: BroadcastTopic,
  onUpdate: BroadcastUpdateHandler,
): ViewerBroadcastChannel {
  const channel = client.channel(topic);

  channel
    .on('broadcast', { event: '*' }, (message) => {
      const update = normalizePortalUpdate(message);

      if (update) {
        onUpdate(update, { topic });
      }
    })
    .subscribe();

  return channel;
}

function normalizePortalUpdate(message: unknown): PortalUpdate | null {
  if (isPortalUpdate(message)) {
    return message;
  }

  if (isRecord(message) && isPortalUpdate(message.payload)) {
    return message.payload;
  }

  return null;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function isStringArray(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === 'string');
}

function isAllowedString<T extends string>(
  value: unknown,
  allowed: readonly T[],
): value is T {
  return typeof value === 'string' && (allowed as readonly string[]).includes(value);
}

function isAllowedStringArray<T extends string>(
  value: unknown,
  allowed: readonly T[],
): value is T[] {
  return Array.isArray(value) && value.every((item) => isAllowedString(item, allowed));
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}
