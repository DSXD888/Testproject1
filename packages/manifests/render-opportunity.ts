import {
  OPPORTUNITY_SECTIONS,
  renderOpportunityMarkdown
} from "./opportunity.md.ts";

type ConfidenceBand = "high" | "usable" | "manual";

type RecommendationDisposition =
  | "auto-qualified"
  | "manual-review"
  | "not-qualified";

type MoneyLike = number | null | undefined;

export interface ProcurementProcess {
  id: string;
  canonicalProcessId?: string | null;
  title: string;
  buyerName?: string | null;
  latestPublishedAt?: string | null;
  latestUpdatedAt?: string | null;
  latestDeadlineAt?: string | null;
  accessRoute?: string | null;
  estimatedValueGbpPence?: number | null;
}

export interface ContractNotice {
  id: string;
  title: string;
  buyerName?: string | null;
  publishedAt?: string | null;
  deadlineAt?: string | null;
  accessRoute?: string | null;
}

export interface NoticeLot {
  id: string;
  lotNumber?: number | null;
  title?: string | null;
  description?: string | null;
  estimatedValuePence?: number | null;
  qualificationStatus?: "qualified" | "review" | "rejected" | "pending" | null;
  addressableValuePence?: number | null;
  goodsClassification?: string | null;
}

export interface ProductCandidate {
  id: string;
  lotId?: string | null;
  name: string;
  archetype?: string | null;
  identityConfidenceBand?: ConfidenceBand | null;
  identityConfidenceBps?: number | null;
  quoteabilityBand?: ConfidenceBand | null;
  quoteabilityScoreBps?: number | null;
}

export interface SupplierQuote {
  id: string;
  candidateId?: string | null;
  supplierName: string;
  unitPricePence?: number | null;
  notes?: string | null;
}

export interface CandidatePricingAssessment {
  id: string;
  candidateId?: string | null;
  source?: string | null;
  priceSignalRole?: string | null;
  matchConfidenceBand?: ConfidenceBand | null;
  matchConfidenceBps?: number | null;
  usabilityConfidenceBand?: ConfidenceBand | null;
  usabilityConfidenceBps?: number | null;
}

export interface LandedCostEstimate {
  id: string;
  candidateId?: string | null;
  incoterm?: string | null;
  freightPence?: number | null;
  dutyPence?: number | null;
  totalPence?: number | null;
}

export interface RiskFlag {
  label: string;
  detail?: string | null;
  severity?: "blocker" | "warning" | "info" | null;
}

export interface PipelineAssessment {
  decisionBasisHash: string;
  artifactHash?: string | null;
  stageRunId?: string | null;
  generatedAt?: string | null;
  recommendation?: {
    disposition: RecommendationDisposition;
    reason: string;
  } | null;
  grossMargin?: {
    outcome: "pass" | "review" | "fail";
    basisPoints?: number | null;
  } | null;
  netMarginBand?: {
    lowBps?: number | null;
    baseBps?: number | null;
    highBps?: number | null;
    label?: string | null;
  } | null;
  riskFlags?: RiskFlag[] | null;
}

export interface ManifestInput {
  process: ProcurementProcess;
  notices: ContractNotice[];
  lots: NoticeLot[];
  candidates: ProductCandidate[];
  quotes: SupplierQuote[];
  pricingAssessments: CandidatePricingAssessment[];
  landedCosts: LandedCostEstimate[];
  pipelineAssessment: PipelineAssessment;
  coverageMetrics: {
    pricedCoverageBps: number;
    landedCostCoverageBps: number;
    netQualifiedCoverageBps: number;
    unresolvedMandatoryLots: number;
  };
}

export interface ManifestOutput {
  markdown: string;
  basisHash: string;
  sections: string[];
  blockerCount: number;
  generatedAt: string;
}

type NormalizedRiskFlag = {
  severity: "blocker" | "warning" | "info";
  label: string;
  detail?: string;
};

const CONFIDENCE_LABELS: Record<ConfidenceBand, string> = {
  high: "high",
  usable: "usable",
  manual: "manual"
};

const RECOMMENDATION_LABELS: Record<RecommendationDisposition, string> = {
  "auto-qualified": "Auto-qualified",
  "manual-review": "Manual review required",
  "not-qualified": "Not qualified"
};

export const renderOpportunityManifest = (
  input: ManifestInput
): ManifestOutput => {
  const sections = OPPORTUNITY_SECTIONS.map((section) => section.id);
  const generatedAt =
    input.pipelineAssessment.generatedAt ??
    input.process.latestUpdatedAt ??
    input.process.latestPublishedAt ??
    new Date(0).toISOString();
  const riskFlags = normalizeRiskFlags(input.pipelineAssessment.riskFlags ?? []);
  const blockerCount = riskFlags.filter(
    (flag) => flag.severity === "blocker"
  ).length;
  const markdown = renderOpportunityMarkdown(
    new Map<string, string>([
      ["summary", renderSummarySection(input)],
      ["lot-analysis", renderLotAnalysisSection(input.lots)],
      ["product-candidates", renderCandidatesSection(input.candidates)],
      [
        "pricing-evidence",
        renderPricingEvidenceSection(
          input.quotes,
          input.pricingAssessments,
          input.candidates
        )
      ],
      [
        "margin-estimate",
        renderMarginEstimateSection(
          input.pipelineAssessment,
          input.landedCosts,
          input.candidates
        )
      ],
      ["coverage-metrics", renderCoverageMetricsSection(input.coverageMetrics)],
      ["risk-flags", renderRiskFlagsSection(riskFlags)],
      ["recommendation", renderRecommendationSection(input.pipelineAssessment)]
    ])
  );

  return {
    markdown,
    basisHash: input.pipelineAssessment.decisionBasisHash,
    sections,
    blockerCount,
    generatedAt
  };
};

const renderSummarySection = (input: ManifestInput): string => {
  const firstNotice = input.notices[0];
  const buyerName = input.process.buyerName ?? firstNotice?.buyerName ?? "Unknown buyer";
  const publishedAt =
    input.process.latestPublishedAt ?? firstNotice?.publishedAt ?? "Unknown";
  const deadlineAt =
    input.process.latestDeadlineAt ?? firstNotice?.deadlineAt ?? "No deadline recorded";
  const accessRoute =
    input.process.accessRoute ?? firstNotice?.accessRoute ?? "Not provided";

  return [
    `- Process: ${input.process.title}`,
    `- Buyer: ${buyerName}`,
    `- Value: ${formatMoney(input.process.estimatedValueGbpPence)}`,
    `- Lot count: ${input.lots.length}`,
    `- Published: ${publishedAt}`,
    `- Deadline: ${deadlineAt}`,
    `- Access route: ${accessRoute}`
  ].join("\n");
};

const renderLotAnalysisSection = (lots: ManifestInput["lots"]): string => {
  if (lots.length === 0) {
    return "No lot analysis is available yet. The manifest remains reviewable while lot extraction catches up.";
  }

  const lines = [
    "| Lot | Qualification | Addressable Value | Goods Classification |",
    "| --- | --- | --- | --- |"
  ];

  for (const lot of lots) {
    const label = lot.title ?? `Lot ${lot.lotNumber ?? lot.id}`;
    lines.push(
      `| ${escapeCell(label)} | ${escapeCell(
        lot.qualificationStatus ?? "pending"
      )} | ${escapeCell(
        formatMoney(lot.addressableValuePence ?? lot.estimatedValuePence)
      )} | ${escapeCell(lot.goodsClassification ?? "not assessed")} |`
    );
  }

  return lines.join("\n");
};

const renderCandidatesSection = (
  candidates: ManifestInput["candidates"]
): string => {
  if (candidates.length === 0) {
    return "No product candidates identified yet. This section stays visible so reviewers can distinguish missing data from omitted rendering.";
  }

  const lines = [
    "| Candidate | Archetype | Identity Confidence | Quoteability |",
    "| --- | --- | --- | --- |"
  ];

  for (const candidate of candidates) {
    lines.push(
      `| ${escapeCell(candidate.name)} | ${escapeCell(
        candidate.archetype ?? "unclassified"
      )} | ${escapeCell(
        renderConfidenceBand(
          candidate.identityConfidenceBand,
          candidate.identityConfidenceBps
        )
      )} | ${escapeCell(
        renderConfidenceBand(
          candidate.quoteabilityBand,
          candidate.quoteabilityScoreBps
        )
      )} |`
    );
  }

  return lines.join("\n");
};

const renderPricingEvidenceSection = (
  quotes: ManifestInput["quotes"],
  pricingAssessments: ManifestInput["pricingAssessments"],
  candidates: ManifestInput["candidates"]
): string => {
  if (quotes.length === 0 && pricingAssessments.length === 0) {
    return "No pricing evidence is available yet. Quote collection can remain incomplete without removing the pricing section.";
  }

  const lines = [
    "| Candidate | Supplier / Source | Unit Price | Confidence | Price Signal Role |",
    "| --- | --- | --- | --- | --- |"
  ];

  for (const quote of quotes) {
    const candidateName =
      candidates.find((candidate) => candidate.id === quote.candidateId)?.name ??
      "Unknown candidate";
    const assessment = pricingAssessments.find(
      (item) => item.candidateId === quote.candidateId
    );
    lines.push(
      `| ${escapeCell(candidateName)} | ${escapeCell(
        quote.supplierName
      )} | ${escapeCell(formatMoney(quote.unitPricePence))} | ${escapeCell(
        renderConfidenceBand(
          assessment?.matchConfidenceBand,
          assessment?.matchConfidenceBps
        )
      )} | ${escapeCell(assessment?.priceSignalRole ?? "unspecified")} |`
    );
  }

  if (quotes.length === 0) {
    for (const assessment of pricingAssessments) {
      const candidateName =
        candidates.find((candidate) => candidate.id === assessment.candidateId)?.name ??
        "Unknown candidate";
      lines.push(
        `| ${escapeCell(candidateName)} | ${escapeCell(
          assessment.source ?? "assessment"
        )} | ${escapeCell("No quote captured")} | ${escapeCell(
          renderConfidenceBand(
            assessment.matchConfidenceBand,
            assessment.matchConfidenceBps
          )
        )} | ${escapeCell(assessment.priceSignalRole ?? "unspecified")} |`
      );
    }
  }

  return lines.join("\n");
};

const renderMarginEstimateSection = (
  pipelineAssessment: ManifestInput["pipelineAssessment"],
  landedCosts: ManifestInput["landedCosts"],
  candidates: ManifestInput["candidates"]
): string => {
  const grossOutcome = pipelineAssessment.grossMargin?.outcome ?? "review";
  const grossMargin = formatPercent(pipelineAssessment.grossMargin?.basisPoints);
  const band =
    pipelineAssessment.netMarginBand?.label ??
    [
      formatPercent(pipelineAssessment.netMarginBand?.lowBps),
      formatPercent(pipelineAssessment.netMarginBand?.baseBps),
      formatPercent(pipelineAssessment.netMarginBand?.highBps)
    ].join(" / ");

  const header = [
    `- Gross margin gate: ${grossOutcome} (${grossMargin})`,
    `- Net margin band: ${band}`
  ];

  if (landedCosts.length === 0) {
    header.push(
      "- No landed-cost estimates available yet. Margin review remains provisional."
    );
    return header.join("\n");
  }

  const lines = [
    ...header,
    "",
    "| Candidate | Incoterm | Freight | Duty | Total |",
    "| --- | --- | --- | --- | --- |"
  ];

  for (const estimate of landedCosts) {
    const candidateName =
      candidates.find((candidate) => candidate.id === estimate.candidateId)?.name ??
      "Unknown candidate";
    lines.push(
      `| ${escapeCell(candidateName)} | ${escapeCell(
        estimate.incoterm ?? "n/a"
      )} | ${escapeCell(formatMoney(estimate.freightPence))} | ${escapeCell(
        formatMoney(estimate.dutyPence)
      )} | ${escapeCell(formatMoney(estimate.totalPence))} |`
    );
  }

  return lines.join("\n");
};

const renderCoverageMetricsSection = (
  coverage: ManifestInput["coverageMetrics"]
): string => {
  return [
    `- Priced coverage: ${formatPercent(coverage.pricedCoverageBps)}`,
    `- Landed-cost coverage: ${formatPercent(coverage.landedCostCoverageBps)}`,
    `- Net-qualified coverage: ${formatPercent(coverage.netQualifiedCoverageBps)}`,
    `- Unresolved mandatory lots: ${coverage.unresolvedMandatoryLots}`
  ].join("\n");
};

const renderRiskFlagsSection = (flags: NormalizedRiskFlag[]): string => {
  if (flags.length === 0) {
    return "No critical blockers are currently flagged.";
  }

  return flags
    .map((flag) => {
      const prefix = flag.severity === "blocker" ? "**BLOCKER**" : flag.severity.toUpperCase();
      return `- ${prefix}: ${flag.label}${flag.detail ? ` — ${flag.detail}` : ""}`;
    })
    .join("\n");
};

const renderRecommendationSection = (
  pipelineAssessment: ManifestInput["pipelineAssessment"]
): string => {
  const recommendation = pipelineAssessment.recommendation;
  if (!recommendation) {
    return "Manual review required — recommendation is not available yet.";
  }

  const label = formatRecommendation(recommendation.disposition);
  return `- ${label}\n- Reason: ${recommendation.reason}`;
};

const normalizeRiskFlags = (
  flags: readonly RiskFlag[]
): NormalizedRiskFlag[] => {
  return flags.map((flag) => ({
    severity: flag.severity ?? "warning",
    label: flag.label,
    detail: flag.detail ?? undefined
  }));
};

const renderConfidenceBand = (
  band: ConfidenceBand | null | undefined,
  basisPoints: number | null | undefined
): string => {
  const resolvedBand =
    band ??
    (basisPoints === null || basisPoints === undefined
      ? null
      : basisPoints >= 9000
        ? "high"
        : basisPoints >= 7000
          ? "usable"
          : "manual");

  return resolvedBand ? CONFIDENCE_LABELS[resolvedBand] : "not rated";
};

const formatRecommendation = (
  disposition: RecommendationDisposition
): string => RECOMMENDATION_LABELS[disposition];

const formatMoney = (value: MoneyLike): string => {
  if (value === null || value === undefined) {
    return "Not available";
  }

  const sign = value < 0 ? "-" : "";
  const absoluteValue = Math.abs(value);
  const pounds = Math.floor(absoluteValue / 100);
  const pence = absoluteValue % 100;

  return `${sign}£${new Intl.NumberFormat("en-GB").format(pounds)}.${pence
    .toString()
    .padStart(2, "0")}`;
};

const formatPercent = (value: number | null | undefined): string => {
  if (value === null || value === undefined) {
    return "Not available";
  }

  const sign = value < 0 ? "-" : "";
  const absoluteValue = Math.abs(value);
  const whole = Math.floor(absoluteValue / 100);
  const fractional = absoluteValue % 100;
  return `${sign}${whole}.${fractional.toString().padStart(2, "0")}%`;
};

const escapeCell = (value: string): string => {
  return value.replaceAll("|", "\\|");
};
