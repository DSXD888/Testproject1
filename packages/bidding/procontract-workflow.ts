import type {
  AgentSession,
  BidDocument,
  BidScopePayload,
  BidWorkflow,
  CompanyProfile,
  DraftSaveResult,
  PipelineData,
  PortalStateVerification,
  SideEffectClass,
  SubmissionResult,
} from './bid-scope-types';

type ResultEnvelopeSession = AgentSession & {
  awaitResultEnvelope: () => Promise<unknown>;
};

const SIDE_EFFECT_CLASSES = new Set<SideEffectClass>([
  'READ_ONLY',
  'REMOTE_DRAFT',
  'NOTIFICATION_OR_LOCK',
  'LEGAL_DECLARATION',
  'FORMAL_SUBMISSION',
  'FUNDS_COMMITTED',
]);

export class ProcontractWorkflowEnvelopeError extends Error {
  constructor(message: string) {
    super(message);
    this.name = 'ProcontractWorkflowEnvelopeError';
  }
}

export class ProcontractWorkflow implements BidWorkflow {
  readonly portalType = 'procontract';

  async authenticate(
    _session: AgentSession,
    _profile: CompanyProfile,
  ): Promise<void> {
    return Promise.resolve();
  }

  async locateOpportunity(
    _session: AgentSession,
    _opportunityRef: string,
  ): Promise<void> {
    return Promise.resolve();
  }

  async verifyPortalState(
    session: AgentSession,
    opportunityRef: string,
  ): Promise<PortalStateVerification> {
    const envelope = await requireResultEnvelopeSession(session).awaitResultEnvelope();
    return parsePortalStateVerification(envelope, opportunityRef);
  }

  async fillForms(
    _session: AgentSession,
    _scope: BidScopePayload,
    _pipelineData: PipelineData,
  ): Promise<void> {
    return Promise.resolve();
  }

  async uploadDocuments(
    _session: AgentSession,
    _documents: BidDocument[],
  ): Promise<void> {
    return Promise.resolve();
  }

  async saveDraft(session: AgentSession): Promise<DraftSaveResult> {
    const envelope = await requireResultEnvelopeSession(session).awaitResultEnvelope();
    return parseDraftSaveResult(envelope);
  }

  async submitFinal(session: AgentSession): Promise<SubmissionResult> {
    const envelope = await requireResultEnvelopeSession(session).awaitResultEnvelope();
    return parseSubmissionResult(envelope);
  }
}

export function parsePortalStateVerification(
  envelope: unknown,
  opportunityRef?: string,
): PortalStateVerification {
  const record = asRecord(envelope, 'portal state envelope');
  const portalState = 'portal_state' in record ? asRecord(record.portal_state, 'portal_state') : record;

  const existingDraftDetected = readBoolean(portalState, 'existing_draft_detected');
  const existingSubmissionDetected = readBoolean(portalState, 'existing_submission_detected');
  const portalStatus = readString(portalState, 'portal_status');
  const opportunityFound = readOpportunityFound(portalState, opportunityRef);

  return {
    opportunityFound,
    existingDraftDetected,
    existingSubmissionDetected,
    portalStatus,
  };
}

export function parseDraftSaveResult(envelope: unknown): DraftSaveResult {
  const record = asRecord(envelope, 'draft save result envelope');
  assertKind(record, 'draft_save_result');

  const sideEffectClass = readSideEffectClass(record, 'side_effect_class');

  return {
    draftSaved: readBoolean(record, 'draft_saved'),
    portalDraftRef: readOptionalString(record, 'portal_draft_ref'),
    sideEffectClass,
  };
}

export function parseSubmissionResult(envelope: unknown): SubmissionResult {
  const record = asRecord(envelope, 'submission result envelope');
  assertKind(record, 'submission_result');

  const screenshotPaths = readScreenshotPaths(record, 'screenshot_paths');

  return {
    submitted: readBoolean(record, 'submitted'),
    confirmationRef: readOptionalString(record, 'confirmation_ref'),
    finalSideEffectClass: readSideEffectClass(record, 'final_side_effect_class'),
    screenshotPaths,
    portalTimestamp: readOptionalString(record, 'portal_timestamp'),
  };
}

function requireResultEnvelopeSession(session: AgentSession): ResultEnvelopeSession {
  const candidate = asRecord(session, 'agent session');
  const awaitResultEnvelope = candidate.awaitResultEnvelope;

  if (typeof awaitResultEnvelope !== 'function') {
    throw new ProcontractWorkflowEnvelopeError(
      'Agent session does not expose awaitResultEnvelope()',
    );
  }

  return session as ResultEnvelopeSession;
}

function assertKind(record: Readonly<Record<string, unknown>>, expected: string): void {
  const kind = readString(record, 'kind');

  if (kind !== expected) {
    throw new ProcontractWorkflowEnvelopeError(
      `Expected result kind ${expected}, received ${kind}`,
    );
  }
}

function readOpportunityFound(
  record: Readonly<Record<string, unknown>>,
  opportunityRef: string | undefined,
): boolean {
  if ('opportunity_found' in record) {
    return readBoolean(record, 'opportunity_found');
  }

  if ('opportunity_ref_seen' in record) {
    const seen = readOptionalString(record, 'opportunity_ref_seen');
    return typeof opportunityRef === 'string' && seen === opportunityRef;
  }

  throw new ProcontractWorkflowEnvelopeError(
    'Portal state is missing opportunity_found or opportunity_ref_seen',
  );
}

function readSideEffectClass(
  record: Readonly<Record<string, unknown>>,
  key: string,
): SideEffectClass {
  const value = readString(record, key);

  if (!SIDE_EFFECT_CLASSES.has(value as SideEffectClass)) {
    throw new ProcontractWorkflowEnvelopeError(
      `${key} must be a valid SideEffectClass`,
    );
  }

  return value as SideEffectClass;
}

function readScreenshotPaths(
  record: Readonly<Record<string, unknown>>,
  key: string,
): string[] {
  const value = record[key];

  if (!Array.isArray(value)) {
    throw new ProcontractWorkflowEnvelopeError(`${key} must be an array`);
  }

  const seen = new Set<string>();

  for (const item of value) {
    if (typeof item !== 'string' || item.length === 0) {
      throw new ProcontractWorkflowEnvelopeError(`${key} entries must be non-empty strings`);
    }

    if (seen.has(item)) {
      throw new ProcontractWorkflowEnvelopeError(`${key} entries must be unique`);
    }

    seen.add(item);
  }

  return [...seen];
}

function readBoolean(record: Readonly<Record<string, unknown>>, key: string): boolean {
  const value = record[key];

  if (typeof value !== 'boolean') {
    throw new ProcontractWorkflowEnvelopeError(`${key} must be a boolean`);
  }

  return value;
}

function readString(record: Readonly<Record<string, unknown>>, key: string): string {
  const value = record[key];

  if (typeof value !== 'string' || value.length === 0) {
    throw new ProcontractWorkflowEnvelopeError(`${key} must be a non-empty string`);
  }

  return value;
}

function readOptionalString(
  record: Readonly<Record<string, unknown>>,
  key: string,
): string | undefined {
  const value = record[key];

  if (value === undefined || value === null) {
    return undefined;
  }

  if (typeof value !== 'string' || value.length === 0) {
    throw new ProcontractWorkflowEnvelopeError(`${key} must be null or a non-empty string`);
  }

  return value;
}

function asRecord(value: unknown, label: string): Readonly<Record<string, unknown>> {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new ProcontractWorkflowEnvelopeError(`${label} must be an object`);
  }

  return value as Readonly<Record<string, unknown>>;
}
