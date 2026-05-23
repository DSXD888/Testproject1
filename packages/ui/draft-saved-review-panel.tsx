import type { ReactElement } from 'react';

import type { SideEffectClass } from '@testproject1/bidding/bid-scope-types';

export interface DraftSavedReviewPanelProps {
  processId: string;
  draftRef: string;
  screenshotPaths: readonly string[];
  manifestHref?: string;
  savedAt?: string;
  sideEffectCap?: SideEffectClass;
  onResumeDraft?: () => void;
  onSubmitFinal?: () => void;
  onAbandonDraft?: () => void;
  submitDisabledReason?: string;
}

function renderScreenshot(path: string, index: number): ReactElement {
  return (
    <li key={`${path}-${index}`}>
      <a href={path}>{`Step ${index + 1}`}</a>
      <span>{path}</span>
    </li>
  );
}

export function DraftSavedReviewPanel({
  processId,
  draftRef,
  screenshotPaths,
  manifestHref,
  savedAt,
  sideEffectCap = 'REMOTE_DRAFT',
  onResumeDraft,
  onSubmitFinal,
  onAbandonDraft,
  submitDisabledReason,
}: DraftSavedReviewPanelProps): ReactElement {
  const submitDisabled = Boolean(submitDisabledReason) || !onSubmitFinal;

  return (
    <section className="draft-saved-review-panel" data-process-id={processId}>
      <header>
        <p className="draft-saved-review-panel__eyebrow">Operator review required</p>
        <h3>Draft saved</h3>
      </header>

      <dl className="draft-saved-review-panel__details">
        <div>
          <dt>Draft reference</dt>
          <dd>{draftRef}</dd>
        </div>
        <div>
          <dt>Process</dt>
          <dd>{processId}</dd>
        </div>
        <div>
          <dt>Side-effect cap</dt>
          <dd>{sideEffectCap}</dd>
        </div>
        {savedAt ? (
          <div>
            <dt>Saved at</dt>
            <dd>
              <time dateTime={savedAt}>{savedAt}</time>
            </dd>
          </div>
        ) : null}
      </dl>

      <section className="draft-saved-review-panel__evidence">
        <h4>Screenshot trail</h4>
        {screenshotPaths.length > 0 ? (
          <ol>{screenshotPaths.map(renderScreenshot)}</ol>
        ) : (
          <p>No screenshots recorded.</p>
        )}
        {manifestHref ? <a href={manifestHref}>Open run manifest</a> : null}
      </section>

      <div className="draft-saved-review-panel__actions">
        <button type="button" disabled={!onResumeDraft} onClick={onResumeDraft}>
          Resume draft
        </button>
        <button type="button" disabled={submitDisabled} onClick={onSubmitFinal}>
          Submit final
        </button>
        <button type="button" disabled={!onAbandonDraft} onClick={onAbandonDraft}>
          Mark abandoned
        </button>
      </div>
      {submitDisabledReason ? (
        <p className="draft-saved-review-panel__blocker" role="status">
          {submitDisabledReason}
        </p>
      ) : null}
    </section>
  );
}
