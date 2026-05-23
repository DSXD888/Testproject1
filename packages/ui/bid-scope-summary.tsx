import type { ReactElement } from 'react';

import type { BidScopePayload } from '@testproject1/bidding/bid-scope-types';

export interface BidScopeSummaryProps {
  scope: BidScopePayload;
  viewerMode?: boolean;
  heading?: string;
}

function joinIds(ids: readonly string[]): string {
  return ids.length === 0 ? 'None selected' : ids.join(', ');
}

function renderField(label: string, value: string | boolean): ReactElement {
  return (
    <div className="bid-scope-summary__field">
      <dt>{label}</dt>
      <dd>{typeof value === 'boolean' ? (value ? 'Yes' : 'No') : value}</dd>
    </div>
  );
}

export function BidScopeSummary({
  scope,
  viewerMode = false,
  heading = 'Bid scope',
}: BidScopeSummaryProps): ReactElement {
  return (
    <section
      className="bid-scope-summary"
      data-portal-type={scope.bidPortalType}
      data-viewer-mode={viewerMode ? 'true' : 'false'}
    >
      <h3>{heading}</h3>
      <dl>
        {renderField('Process', scope.processId)}
        {renderField('Portal', scope.bidPortalType)}
        {renderField('Opportunity reference', scope.portalOpportunityRef)}
        {renderField('Side-effect cap', scope.sideEffectCap)}
        {renderField('Review required', scope.reviewRequired)}
        {viewerMode ? null : (
          <>
            {renderField('Lots', joinIds(scope.lotIds))}
            {renderField('Candidates', joinIds(scope.candidateIds))}
            {renderField('Selected quotes', joinIds(scope.selectedQuoteIds))}
            {renderField('Company profile version', scope.companyProfileVersion)}
            {renderField('Decision basis hash', scope.decisionBasisHash)}
          </>
        )}
      </dl>
    </section>
  );
}
