import type { JSX } from "react";

export interface PipelineBadgeProps {
  state:
    | "pending"
    | "qualifying"
    | "enriching"
    | "qualified"
    | "review"
    | "rejected"
    | "error";
  validationState?: "unverified" | "validated" | "flagged";
  size?: "sm" | "md" | "lg";
}

interface BadgeVisual {
  label: string;
  icon: string;
  tone: "muted" | "info" | "good" | "warn" | "bad";
}

const BADGE_VISUALS: Record<PipelineBadgeProps["state"], BadgeVisual> = {
  pending: { label: "Pending", icon: "...", tone: "muted" },
  qualifying: { label: "Qualifying", icon: "Q", tone: "info" },
  enriching: { label: "Enriching", icon: "E", tone: "info" },
  qualified: { label: "Qualified", icon: "OK", tone: "good" },
  review: { label: "Review", icon: "!", tone: "warn" },
  rejected: { label: "Rejected", icon: "X", tone: "bad" },
  error: { label: "Error", icon: "!!", tone: "bad" },
};

const VALIDATION_LABELS: Record<NonNullable<PipelineBadgeProps["validationState"]>, string> = {
  unverified: "Unverified",
  validated: "Validated",
  flagged: "Flagged",
};

export function PipelineBadge({
  state,
  validationState = "unverified",
  size = "md",
}: PipelineBadgeProps): JSX.Element {
  const visual = BADGE_VISUALS[state];
  const validationLabel =
    validationState === "unverified" ? null : VALIDATION_LABELS[validationState];

  return (
    <span
      className={`pipeline-badge pipeline-badge--${visual.tone} pipeline-badge--${size}`}
      data-state={state}
      data-tone={visual.tone}
      title={`Pipeline state: ${visual.label}`}
    >
      <span aria-hidden="true" className="pipeline-badge__icon">
        {visual.icon}
      </span>
      <span className="pipeline-badge__label">{visual.label}</span>
      {validationLabel ? (
        <span
          className={`pipeline-badge__validation pipeline-badge__validation--${validationState}`}
          data-validation-state={validationState}
        >
          {validationLabel}
        </span>
      ) : null}
    </span>
  );
}
