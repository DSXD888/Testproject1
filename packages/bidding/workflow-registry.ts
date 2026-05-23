import type { BidWorkflow } from './bid-scope-types';
import { ProcontractWorkflow } from './procontract-workflow';

export { ProcontractWorkflow } from './procontract-workflow';

export type SupportedBidPortal = 'procontract';

export class BidWorkflowRegistryError extends Error {
  readonly portalType: string;

  constructor(portalType: string) {
    super(`Unsupported bid portal type: ${portalType}`);
    this.name = 'BidWorkflowRegistryError';
    this.portalType = portalType;
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
