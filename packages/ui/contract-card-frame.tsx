import type { JSX, ReactNode } from "react";

import { PipelineBadge, type PipelineBadgeProps } from "./pipeline-badge";

export interface ContractCardFrameProps {
  summary: {
    processId: string;
    title: string;
    buyerName: string;
    publishedDate: string;
    deadlineDate: string | null;
    estimatedValuePence: number | null;
    lotCount: number;
    accessRoute: string;
    legalCategory: "goods" | "services" | "works" | "mixed" | "unknown";
  };
  metrics: {
    pipelineState: PipelineBadgeProps["state"];
    pricedCoverageBps: number | null;
    netQualifiedCoverageBps: number | null;
    netQualifiedValueCoverageBps: number | null;
    grossMarginBps: number | null;
    netMarginBaseBps: number | null;
    candidateCount: number;
    quotedCandidateCount: number;
  };
  decision?: {
    disposition: "approve" | "reject" | "watch" | null;
    isStale: boolean;
    decidedAt: string | null;
  };
  rightRail?: ReactNode;
  actions?: ReactNode;
}

export function ContractCardFrame({
  summary,
  metrics,
  decision,
  rightRail,
  actions,
}: ContractCardFrameProps): JSX.Element {
  return (
    <article
      className="contract-card-frame"
      data-process-id={summary.processId}
      data-pipeline-state={metrics.pipelineState}
    >
      <header className="contract-card-frame__header">
        <div className="contract-card-frame__identity">
          <h2>{summary.title}</h2>
          <p>{summary.buyerName}</p>
        </div>
        {rightRail ? <aside className="contract-card-frame__right-rail">{rightRail}</aside> : null}
      </header>

      <section className="contract-card-frame__summary">
        <dl>
          <div>
            <dt>Published</dt>
            <dd>{summary.publishedDate}</dd>
          </div>
          <div>
            <dt>Deadline</dt>
            <dd>{summary.deadlineDate ?? "No deadline published"}</dd>
          </div>
          <div>
            <dt>Estimated value</dt>
            <dd>{formatMoney(summary.estimatedValuePence)}</dd>
          </div>
          <div>
            <dt>Lots</dt>
            <dd>{summary.lotCount}</dd>
          </div>
          <div>
            <dt>Access route</dt>
            <dd>{summary.accessRoute}</dd>
          </div>
          <div>
            <dt>Category</dt>
            <dd>{summary.legalCategory}</dd>
          </div>
        </dl>
      </section>

      <section className="contract-card-frame__metrics">
        <PipelineBadge state={metrics.pipelineState} size="md" />
        <dl>
          <div>
            <dt>Priced coverage</dt>
            <dd>{formatPercent(metrics.pricedCoverageBps)}</dd>
          </div>
          <div>
            <dt>Net-qualified coverage</dt>
            <dd>{formatPercent(metrics.netQualifiedCoverageBps)}</dd>
          </div>
          <div>
            <dt>Value-weighted gate</dt>
            <dd>{formatPercent(metrics.netQualifiedValueCoverageBps)}</dd>
          </div>
          <div>
            <dt>Gross margin</dt>
            <dd>{formatPercent(metrics.grossMarginBps)}</dd>
          </div>
          <div>
            <dt>Net margin</dt>
            <dd>{formatPercent(metrics.netMarginBaseBps)}</dd>
          </div>
          <div>
            <dt>Candidates</dt>
            <dd>
              {metrics.quotedCandidateCount} quoted / {metrics.candidateCount} total
            </dd>
          </div>
        </dl>
      </section>

      {decision ? (
        <section className="contract-card-frame__decision">
          <p>
            Decision: <strong>{decision.disposition ?? "No decision recorded"}</strong>
          </p>
          <p>{decision.decidedAt ?? "Decision not yet captured"}</p>
          {decision.isStale ? (
            <p className="contract-card-frame__decision-warning">
              Decision may be outdated -- pipeline data changed since review
            </p>
          ) : null}
        </section>
      ) : null}

      {actions ? <footer className="contract-card-frame__actions">{actions}</footer> : null}
    </article>
  );
}

function formatMoney(valuePence: number | null): string {
  if (valuePence === null) {
    return "N/A";
  }

  const sign = valuePence < 0 ? "-" : "";
  const absoluteValue = Math.abs(valuePence);
  const pounds = Math.trunc(absoluteValue / 100);
  const pence = absoluteValue % 100;
  const poundsText = new Intl.NumberFormat("en-GB").format(pounds);
  return `${sign}GBP ${poundsText}.${pence.toString().padStart(2, "0")}`;
}

function formatPercent(valueBps: number | null): string {
  if (valueBps === null) {
    return "N/A";
  }

  const sign = valueBps < 0 ? "-" : "";
  const absoluteValue = Math.abs(valueBps);
  const whole = Math.trunc(absoluteValue / 100);
  const fraction = absoluteValue % 100;
  return `${sign}${whole}.${fraction.toString().padStart(2, "0")}%`;
}
