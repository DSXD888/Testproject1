export interface BidScopePayload {
  processId: string;
  lotIds: string[];
  candidateIds: string[];
  selectedQuoteIds: string[];
  companyProfileVersion: string;
  bidPortalType: string;
  portalOpportunityRef: string;
  reviewRequired: boolean;
  decisionBasisHash: string;
  sideEffectCap: SideEffectClass;
}

export type SideEffectClass =
  | 'READ_ONLY'
  | 'REMOTE_DRAFT'
  | 'NOTIFICATION_OR_LOCK'
  | 'LEGAL_DECLARATION'
  | 'FORMAL_SUBMISSION'
  | 'FUNDS_COMMITTED';

export const SIDE_EFFECT_CLASS_ORDER: SideEffectClass[] = [
  'READ_ONLY',
  'REMOTE_DRAFT',
  'NOTIFICATION_OR_LOCK',
  'LEGAL_DECLARATION',
  'FORMAL_SUBMISSION',
  'FUNDS_COMMITTED',
];

export interface BidDocument {
  documentType: string;
  fileName: string;
  filePath: string;
  contentHash: string;
  expiryDate?: string;
  issuingBody?: string;
}

export interface SubmissionResult {
  submitted: boolean;
  confirmationRef?: string;
  finalSideEffectClass: SideEffectClass;
  screenshotPaths: string[];
  portalTimestamp?: string;
}

export interface DraftSaveResult {
  draftSaved: boolean;
  portalDraftRef?: string;
  sideEffectClass: SideEffectClass;
}

export interface PortalStateVerification {
  opportunityFound: boolean;
  existingDraftDetected: boolean;
  existingSubmissionDetected: boolean;
  portalStatus: string;
}

export interface CompanyProfile {
  companyName: string;
  registrationNumber: string;
  vatNumber?: string;
  address: {
    line1: string;
    line2?: string;
    city: string;
    county?: string;
    postcode: string;
    country: string;
  };
  primaryContact: {
    name: string;
    email: string;
    phone: string;
    role: string;
  };
  certifications: Array<{
    type: string;
    number: string;
    expiryDate: string;
    issuingBody: string;
  }>;
  insurancePolicies: Array<{
    type: string;
    provider: string;
    policyNumber: string;
    coverAmountPence: number;
    expiryDate: string;
  }>;
  financialStatements?: {
    lastFiledYear: number;
    companiesHouseRef: string;
  };
  smeStatus?: 'micro' | 'small' | 'medium' | 'large';
  equalityStatement?: string;
  environmentalPolicy?: string;
}

export type AgentSession = Readonly<Record<string, unknown>>;

export type PipelineData = Readonly<Record<string, unknown>>;

export interface BidWorkflow {
  portalType: string;
  authenticate(session: AgentSession, profile: CompanyProfile): Promise<void>;
  locateOpportunity(session: AgentSession, ref: string): Promise<void>;
  verifyPortalState(session: AgentSession, ref: string): Promise<PortalStateVerification>;
  fillForms(session: AgentSession, scope: BidScopePayload, data: PipelineData): Promise<void>;
  uploadDocuments(session: AgentSession, documents: BidDocument[]): Promise<void>;
  saveDraft(session: AgentSession): Promise<DraftSaveResult>;
  submitFinal(session: AgentSession): Promise<SubmissionResult>;
}
