import { useId, useState, type FormEvent, type ReactElement } from 'react';

import type { BidWorkflowOverrideRecord } from './QuoteRoleOverrideForm';

export interface ValidationOverrideFormProps {
  processId: string;
  recordedBy: string;
  onSubmit: (record: BidWorkflowOverrideRecord) => Promise<void>;
}

export function ValidationOverrideForm({
  processId,
  recordedBy,
  onSubmit,
}: ValidationOverrideFormProps): ReactElement {
  const reasonId = useId();
  const ackId = useId();
  const [reason, setReason] = useState('');
  const [acknowledged, setAcknowledged] = useState(false);
  const [status, setStatus] = useState<'idle' | 'saving' | 'submitted'>('idle');
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
        kind: 'validation_gate',
        reason: trimmedReason,
        recorded_by: recordedBy,
        recorded_at: new Date().toISOString(),
      });
      setStatus('submitted');
      setReason('');
      setAcknowledged(false);
    } catch (submitError) {
      setStatus('idle');
      setError(
        submitError instanceof Error
          ? submitError.message
          : 'Failed to record validation override.',
      );
    }
  }

  return (
    <form
      className="bid-workflow-override-form"
      data-process-id={processId}
      data-testid="validation-override-form"
      onSubmit={(event) => void handleSubmit(event)}
    >
      <h4>Validation override</h4>
      <p>
        Final submission is blocked by validator rejection or outage. Override
        only after a manual review has been recorded.
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
        I acknowledge that this final submission bypasses the validator gate.
      </label>
      <button
        type="submit"
        data-testid="override-submit-final"
        disabled={status === 'saving'}
      >
        {status === 'saving' ? 'Submitting final bid' : 'Override and submit final'}
      </button>
      {error ? (
        <p role="alert" data-testid="override-error">
          {error}
        </p>
      ) : null}
      {status === 'submitted' ? (
        <p role="status" data-testid="override-status">
          Validation override recorded; final submission queued.
        </p>
      ) : null}
    </form>
  );
}
