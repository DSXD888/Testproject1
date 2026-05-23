import { describe, expect, it } from 'vitest';
import {
  BidWorkflowRegistryError,
  ProcontractWorkflow,
  getWorkflow,
  listSupportedPortals,
} from '../workflow-registry';

describe('L6-T-bid_portal_procontract_skill-002: bid workflow registry', () => {
  it('maps the Procontract portal type to the Procontract workflow implementation', () => {
    const workflow = getWorkflow('procontract');

    expect(listSupportedPortals()).toEqual(['procontract']);
    expect(workflow).toBeInstanceOf(ProcontractWorkflow);
    expect(workflow.portalType).toBe('procontract');
  });

  it('rejects unsupported portal types with structured error context', () => {
    expect(() => getWorkflow('unknown-portal')).toThrow(BidWorkflowRegistryError);

    try {
      getWorkflow('unknown-portal');
    } catch (error) {
      expect(error).toBeInstanceOf(BidWorkflowRegistryError);
      expect((error as BidWorkflowRegistryError).portalType).toBe('unknown-portal');
    }
  });
});
