import type { PipelineBadgeProps } from '../pipeline-badge';

export const pipelineBadgeFixture: PipelineBadgeProps = {
  state: 'qualified',
  validationState: 'unverified',
  size: 'md',
};

export const flaggedPipelineBadgeFixture: PipelineBadgeProps = {
  state: 'review',
  validationState: 'flagged',
  size: 'sm',
};
