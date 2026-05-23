import { describe, expect, it } from 'vitest';
import type { AgentSession } from '../bid-scope-types';
import {
  ProcontractWorkflow,
  ProcontractWorkflowEnvelopeError,
  parseDraftSaveResult,
  parsePortalStateVerification,
  parseSubmissionResult,
} from '../procontract-workflow';

function sessionWithEnvelope(envelope: unknown): AgentSession {
  return {
    awaitResultEnvelope: async () => envelope,
  };
}

describe('L6-T-bid_portal_procontract_skill-002: Procontract workflow', () => {
  it('implements the Procontract BidWorkflow interface without browser-side effects', async () => {
    const workflow = new ProcontractWorkflow();
    const session = sessionWithEnvelope({});

    expect(workflow.portalType).toBe('procontract');
    await expect(workflow.authenticate(session, minimalCompanyProfile())).resolves.toBeUndefined();
    await expect(workflow.locateOpportunity(session, 'DN-OPP-123')).resolves.toBeUndefined();
    await expect(workflow.fillForms(session, minimalBidScope(), {})).resolves.toBeUndefined();
    await expect(workflow.uploadDocuments(session, [])).resolves.toBeUndefined();
  });

  it('parses portal-state checkpoints into the package portal state contract', async () => {
    const workflow = new ProcontractWorkflow();
    const envelope = {
      step: 4,
      side_effect_class_reached: 'READ_ONLY',
      screenshot_paths: ['sessions/2026/05/bid/step-04.webp'],
      portal_state: {
        opportunity_ref_seen: 'DN-OPP-123',
        portal_status: 'open',
        existing_draft_detected: false,
        existing_submission_detected: false,
      },
    };

    await expect(workflow.verifyPortalState(sessionWithEnvelope(envelope), 'DN-OPP-123')).resolves.toEqual({
      opportunityFound: true,
      existingDraftDetected: false,
      existingSubmissionDetected: false,
      portalStatus: 'open',
    });
  });

  it('parses draft-save and final-submission result envelopes', async () => {
    expect(
      parseDraftSaveResult({
        kind: 'draft_save_result',
        draft_saved: true,
        portal_draft_ref: 'DRAFT-123',
        side_effect_class: 'REMOTE_DRAFT',
      }),
    ).toEqual({
      draftSaved: true,
      portalDraftRef: 'DRAFT-123',
      sideEffectClass: 'REMOTE_DRAFT',
    });

    expect(
      parseSubmissionResult({
        kind: 'submission_result',
        submitted: true,
        confirmation_ref: 'PC-2026-000123',
        final_side_effect_class: 'FORMAL_SUBMISSION',
        screenshot_paths: ['sessions/2026/05/bid/step-10b.webp'],
        portal_timestamp: '2026-05-23T12:00:00Z',
      }),
    ).toEqual({
      submitted: true,
      confirmationRef: 'PC-2026-000123',
      finalSideEffectClass: 'FORMAL_SUBMISSION',
      screenshotPaths: ['sessions/2026/05/bid/step-10b.webp'],
      portalTimestamp: '2026-05-23T12:00:00Z',
    });
  });

  it('rejects malformed result envelopes before callers treat them as portal evidence', () => {
    expect(() => parseSubmissionResult({ kind: 'draft_save_result' })).toThrow(
      ProcontractWorkflowEnvelopeError,
    );
    expect(() =>
      parsePortalStateVerification({
        portal_status: 'open',
        existing_draft_detected: false,
        existing_submission_detected: false,
      }),
    ).toThrow(ProcontractWorkflowEnvelopeError);
  });
});

function minimalBidScope() {
  return {
    processId: 'process-123',
    lotIds: ['lot-1'],
    candidateIds: ['candidate-1'],
    selectedQuoteIds: ['quote-1'],
    companyProfileVersion: 'profile-v1',
    bidPortalType: 'procontract',
    portalOpportunityRef: 'DN-OPP-123',
    reviewRequired: true,
    decisionBasisHash: 'basis-hash-123',
    sideEffectCap: 'REMOTE_DRAFT' as const,
  };
}

function minimalCompanyProfile() {
  return {
    companyName: 'Sorelon Ltd',
    registrationNumber: '12345678',
    address: {
      line1: '1 Market Street',
      city: 'London',
      postcode: 'SW1A 1AA',
      country: 'GB',
    },
    primaryContact: {
      name: 'Procurement Lead',
      email: 'operator@example.invalid',
      phone: '+440000000000',
      role: 'Operator',
    },
    certifications: [],
    insurancePolicies: [],
  };
}
