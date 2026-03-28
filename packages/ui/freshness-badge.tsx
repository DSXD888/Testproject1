import type { JSX } from "react";

export interface FreshnessBadgeVM {
  label: string;
  tone: "good" | "warn" | "bad" | "muted";
  tooltip: string;
}

export interface FreshnessBadgeProps {
  vm: FreshnessBadgeVM;
}

const FRESHNESS_ICONS: Record<FreshnessBadgeVM["tone"], string> = {
  good: "OK",
  warn: "!",
  bad: "X",
  muted: "...",
};

export function FreshnessBadge({ vm }: FreshnessBadgeProps): JSX.Element {
  return (
    <span
      className={`freshness-badge freshness-badge--${vm.tone}`}
      data-tone={vm.tone}
      title={vm.tooltip}
    >
      <span aria-hidden="true" className="freshness-badge__icon">
        {FRESHNESS_ICONS[vm.tone]}
      </span>
      <span className="freshness-badge__label">{vm.label}</span>
    </span>
  );
}
