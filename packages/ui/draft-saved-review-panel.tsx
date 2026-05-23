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
  onOpenSessionViewer?: () => void;
  submitDisabledReason?: string;
}

function renderScreenshot(path: string, index: number): ReactElement {
  return (
    <li key={`${path}-${index}`}>
      <a href={path}>{`Step ${index + 1}`}</a>
      <img
        alt={`Bid workflow screenshot step ${index + 1}`}
        loading="lazy"
        src={path}
      />
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
  onOpenSessionViewer,
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
          <dd data-testid="draft-ref">{draftRef}</dd>
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
          <ol data-testid="screenshot-gallery">{screenshotPaths.map(renderScreenshot)}</ol>
        ) : (
          <p>No screenshots recorded.</p>
        )}
        {manifestHref ? (
          <a data-testid="manifest-link" href={manifestHref}>
            Open run manifest
          </a>
        ) : null}
      </section>

      <div className="draft-saved-review-panel__actions">
        <button
          type="button"
          data-testid="action-resume"
          disabled={!onResumeDraft}
          onClick={onResumeDraft}
        >
          Resume draft
        </button>
        <button
          type="button"
          data-testid="action-submit-final"
          disabled={submitDisabled}
          onClick={onSubmitFinal}
        >
          Submit final
        </button>
        <button
          type="button"
          data-testid="action-abandon"
          disabled={!onAbandonDraft}
          onClick={onAbandonDraft}
        >
          Mark abandoned
        </button>
      </div>
      {onOpenSessionViewer ? (
        <button
          type="button"
          data-testid="open-session-viewer"
          onClick={onOpenSessionViewer}
        >
          View session
        </button>
      ) : null}
      {submitDisabledReason ? (
        <p
          className="draft-saved-review-panel__blocker"
          data-testid="formal-submit-blocker"
          role="status"
        >
          {submitDisabledReason}
        </p>
      ) : null}
    </section>
  );
}
