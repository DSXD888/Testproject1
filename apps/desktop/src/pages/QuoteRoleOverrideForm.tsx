import { useId, useState, type FormEvent, type ReactElement } from 'react';

export interface BidWorkflowOverrideRecord {
  kind: 'quote_role_floor_only' | 'validation_gate';
  reason: string;
  recorded_by: string;
  recorded_at: string;
}

export interface QuoteRoleOverrideFormProps {
  processId: string;
  recordedBy: string;
  onSubmit: (record: BidWorkflowOverrideRecord) => Promise<void>;
}

export function QuoteRoleOverrideForm({
  processId,
  recordedBy,
  onSubmit,
}: QuoteRoleOverrideFormProps): ReactElement {
  const reasonId = useId();
  const ackId = useId();
  const [reason, setReason] = useState('');
  const [acknowledged, setAcknowledged] = useState(false);
  const [status, setStatus] = useState<'idle' | 'saving' | 'saved'>('idle');
  const [error, setError] = useState<string | null>(null);

  async function handleSubmit(event: FormEvent<HTMLFormElement>): Promise<void> {
    event.preventDefault();
    const trimmedReason = reason.trim();
    if (!trimmedReason) {
      setError('Reason is required.');
      return;
    }
    if (!acknowledged) {
      setError('Acknowledgment is required.');
      return;
    }

    setStatus('saving');
    setError(null);
    try {
      await onSubmit({
        kind: 'quote_role_floor_only',
        reason: trimmedReason,
        recorded_by: recordedBy,
        recorded_at: new Date().toISOString(),
      });
      setStatus('saved');
      setReason('');
      setAcknowledged(false);
    } catch (submitError) {
      setStatus('idle');
      setError(
        submitError instanceof Error
          ? submitError.message
          : 'Failed to record quote-role override.',
      );
    }
  }

  return (
    <form
      className="bid-workflow-override-form"
      data-process-id={processId}
      data-testid="quote-role-override-form"
      onSubmit={(event) => void handleSubmit(event)}
    >
      <h4>Quote-role override</h4>
      <p>
        Floor-only evidence can understate real bid cost. Proceed only with a
        structured operator acknowledgment.
      </p>
      <label htmlFor={reasonId}>
        Reason
        <textarea
          id={reasonId}
          data-testid="override-reason"
          value={reason}
          onChange={(event) => setReason(event.target.value)}
          rows={3}
        />
      </label>
      <label htmlFor={ackId}>
        <input
          id={ackId}
          data-testid="override-acknowledged"
          type="checkbox"
          checked={acknowledged}
          onChange={(event) => setAcknowledged(event.target.checked)}
        />
        I acknowledge that floor-only evidence carries elevated pricing risk.
      </label>
      <button
        type="submit"
        data-testid="override-submit"
        disabled={status === 'saving'}
      >
        {status === 'saving' ? 'Recording override' : 'Override and proceed'}
      </button>
      {error ? (
        <p role="alert" data-testid="override-error">
          {error}
        </p>
      ) : null}
      {status === 'saved' ? (
        <p role="status" data-testid="override-status">
          Quote-role override recorded.
        </p>
      ) : null}
    </form>
  );
}
