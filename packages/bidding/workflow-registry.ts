import type {
  AgentSession,
  BidDocument,
  BidScopePayload,
  BidWorkflow,
  CompanyProfile,
  DraftSaveResult,
  PipelineData,
  PortalStateVerification,
  SubmissionResult,
} from './bid-scope-types';

export type SupportedBidPortal = 'procontract';

export class BidWorkflowRegistryError extends Error {
  readonly portalType: string;

  constructor(portalType: string) {
    super(`Unsupported bid portal type: ${portalType}`);
    this.name = 'BidWorkflowRegistryError';
    this.portalType = portalType;
  }
}

class ProcontractWorkflowMethodUnavailableError extends Error {
  constructor(method: keyof BidWorkflow) {
    super(`Procontract bid workflow method is not available yet: ${method}`);
    this.name = 'ProcontractWorkflowMethodUnavailableError';
  }
}

export class ProcontractWorkflow implements BidWorkflow {
  readonly portalType = 'procontract';

  async authenticate(
    _session: AgentSession,
    _profile: CompanyProfile,
  ): Promise<void> {
    throw new ProcontractWorkflowMethodUnavailableError('authenticate');
  }

  async locateOpportunity(
    _session: AgentSession,
    _opportunityRef: string,
  ): Promise<void> {
    throw new ProcontractWorkflowMethodUnavailableError('locateOpportunity');
  }

  async verifyPortalState(
    _session: AgentSession,
    _opportunityRef: string,
  ): Promise<PortalStateVerification> {
    throw new ProcontractWorkflowMethodUnavailableError('verifyPortalState');
  }

  async fillForms(
    _session: AgentSession,
    _scope: BidScopePayload,
    _pipelineData: PipelineData,
  ): Promise<void> {
    throw new ProcontractWorkflowMethodUnavailableError('fillForms');
  }

  async uploadDocuments(
    _session: AgentSession,
    _documents: BidDocument[],
  ): Promise<void> {
    throw new ProcontractWorkflowMethodUnavailableError('uploadDocuments');
  }

  async saveDraft(_session: AgentSession): Promise<DraftSaveResult> {
    throw new ProcontractWorkflowMethodUnavailableError('saveDraft');
  }

  async submitFinal(_session: AgentSession): Promise<SubmissionResult> {
    throw new ProcontractWorkflowMethodUnavailableError('submitFinal');
  }
}

const workflowFactories: Record<SupportedBidPortal, () => BidWorkflow> = {
  procontract: () => new ProcontractWorkflow(),
};

export function listSupportedPortals(): SupportedBidPortal[] {
  return Object.keys(workflowFactories) as SupportedBidPortal[];
}

export function getWorkflow(portalType: string): BidWorkflow {
  const factory = workflowFactories[portalType as SupportedBidPortal];

  if (!factory) {
    throw new BidWorkflowRegistryError(portalType);
  }

  return factory();
}
