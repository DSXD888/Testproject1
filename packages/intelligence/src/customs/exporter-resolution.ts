export const EXPORTER_UNRESOLVED = "EXPORTER_UNRESOLVED" as const;

export type ExporterRateBasis =
  | "named_exporter"
  | "annex_exporter"
  | "residual_default";

export type EvidenceArtifactKind =
  | "invoice"
  | "certificate"
  | "supplier_profile"
  | "listing";

export interface SupplierQuote {
  supplierName?: string;
  marketplaceSellerName?: string;
  manufacturerName?: string;
  exporterName?: string;
  supplierRole?:
    | "manufacturer"
    | "exporter"
    | "distributor"
    | "marketplace_seller"
    | "unknown";
  factoryDirectClaim?: boolean;
}

export interface EvidenceArtifact {
  kind: EvidenceArtifactKind;
  refPath: string;
  manufacturerName?: string;
  exporterName?: string;
  productMatched?: boolean;
}

export interface TradeRemedyExporterRate {
  exporterName: string;
  manufacturerName?: string;
  additionalCode?: string;
  ratePctBps: number;
}

export interface TariffMeasure {
  id: string;
  kind:
    | "anti_dumping"
    | "countervailing"
    | "safeguard"
    | "trade_remedy";
  active: boolean;
  countryOfOrigin?: string;
  residualRatePctBps: number;
  namedExporterRates?: TradeRemedyExporterRate[];
  annexExporterRates?: TradeRemedyExporterRate[];
}

export interface ExporterResolutionInput {
  commodityCode: string;
  countryOfOrigin: string;
  customsValueGbpPence?: number;
  supplierQuotes: SupplierQuote[];
  evidenceArtifacts?: EvidenceArtifact[];
  measuresAtCommodity: TariffMeasure[];
}

export interface EvidenceLeverage {
  kind: "named_exporter_proof";
  missingEvidence: string[];
  unlockableMarginBps: number;
}

export interface ExporterResolutionResult {
  commodityCode: string;
  countryOfOrigin: string;
  exporterRateBasis: ExporterRateBasis;
  exporterIdentityConfidenceBps: number;
  selectedRatePctBps?: number;
  appliedTradeRemedyDutyGbpPence?: number;
  unlockableMarginBps?: number;
  errorCode?: typeof EXPORTER_UNRESOLVED;
  missingEvidence: string[];
  evidenceLeverage: EvidenceLeverage[];
}

interface MatchedRate {
  basis: Exclude<ExporterRateBasis, "residual_default">;
  confidenceBps: number;
  rate: TradeRemedyExporterRate;
}

const LOW_CONFIDENCE_BPS = 2500;

export function resolveExporter(
  input: ExporterResolutionInput,
): ExporterResolutionResult {
  const activeMeasure = selectActiveTradeRemedy(input);
  if (activeMeasure === undefined) {
    return {
      commodityCode: input.commodityCode,
      countryOfOrigin: input.countryOfOrigin,
      exporterRateBasis: "residual_default",
      exporterIdentityConfidenceBps: 0,
      missingEvidence: [],
      evidenceLeverage: [],
    };
  }

  const matchedRate = matchVerifiedExporterRate(
    input.supplierQuotes,
    input.evidenceArtifacts ?? [],
    activeMeasure,
  );

  if (matchedRate !== undefined) {
    return {
      commodityCode: input.commodityCode,
      countryOfOrigin: input.countryOfOrigin,
      exporterRateBasis: matchedRate.basis,
      exporterIdentityConfidenceBps: matchedRate.confidenceBps,
      selectedRatePctBps: matchedRate.rate.ratePctBps,
      appliedTradeRemedyDutyGbpPence: dutyPence(
        input.customsValueGbpPence,
        matchedRate.rate.ratePctBps,
      ),
      missingEvidence: [],
      evidenceLeverage: [],
    };
  }

  const missingEvidence = missingExporterEvidence(
    input.supplierQuotes,
    input.evidenceArtifacts ?? [],
    activeMeasure,
  );
  const unlockableMarginBps = unlockableNamedExporterMarginBps(activeMeasure);

  return {
    commodityCode: input.commodityCode,
    countryOfOrigin: input.countryOfOrigin,
    exporterRateBasis: "residual_default",
    exporterIdentityConfidenceBps: unresolvedConfidenceBps(input.supplierQuotes),
    selectedRatePctBps: activeMeasure.residualRatePctBps,
    appliedTradeRemedyDutyGbpPence: dutyPence(
      input.customsValueGbpPence,
      activeMeasure.residualRatePctBps,
    ),
    unlockableMarginBps,
    errorCode: EXPORTER_UNRESOLVED,
    missingEvidence,
    evidenceLeverage:
      unlockableMarginBps > 0
        ? [
            {
              kind: "named_exporter_proof",
              missingEvidence,
              unlockableMarginBps,
            },
          ]
        : [],
  };
}

function selectActiveTradeRemedy(
  input: Pick<ExporterResolutionInput, "countryOfOrigin" | "measuresAtCommodity">,
): TariffMeasure | undefined {
  return input.measuresAtCommodity.find(
    (measure) =>
      measure.active &&
      (measure.countryOfOrigin === undefined ||
        measure.countryOfOrigin === input.countryOfOrigin),
  );
}

function matchVerifiedExporterRate(
  quotes: readonly SupplierQuote[],
  artifacts: readonly EvidenceArtifact[],
  measure: TariffMeasure,
): MatchedRate | undefined {
  const namedRate = findMatchedRate(measure.namedExporterRates ?? [], quotes, artifacts);
  if (
    namedRate !== undefined &&
    hasArtifact(artifacts, ["invoice", "certificate"], namedRate) &&
    hasArtifact(artifacts, ["supplier_profile"], namedRate)
  ) {
    return {
      basis: "named_exporter",
      confidenceBps: 8000,
      rate: namedRate,
    };
  }

  const annexRate = findMatchedRate(measure.annexExporterRates ?? [], quotes, artifacts);
  if (
    annexRate !== undefined &&
    hasArtifact(artifacts, ["listing"], annexRate) &&
    hasArtifact(artifacts, ["supplier_profile"], annexRate)
  ) {
    return {
      basis: "annex_exporter",
      confidenceBps: 6500,
      rate: annexRate,
    };
  }

  return undefined;
}

function findMatchedRate(
  rates: readonly TradeRemedyExporterRate[],
  quotes: readonly SupplierQuote[],
  artifacts: readonly EvidenceArtifact[],
): TradeRemedyExporterRate | undefined {
  return rates.find((rate) => {
    const artifactMatch = artifacts.some(
      (artifact) => artifact.productMatched !== false && namesMatch(artifact, rate),
    );
    const quoteMatch = quotes.some(
      (quote) =>
        isManufacturerOrExporterQuote(quote) &&
        (sameName(quote.manufacturerName, rate.manufacturerName) ||
          sameName(quote.exporterName, rate.exporterName)),
    );
    return artifactMatch || quoteMatch;
  });
}

function namesMatch(
  artifact: Pick<EvidenceArtifact, "manufacturerName" | "exporterName">,
  rate: TradeRemedyExporterRate,
): boolean {
  return (
    sameName(artifact.manufacturerName, rate.manufacturerName) ||
    sameName(artifact.exporterName, rate.exporterName)
  );
}

function hasArtifact(
  artifacts: readonly EvidenceArtifact[],
  kinds: readonly EvidenceArtifactKind[],
  rate: TradeRemedyExporterRate,
): boolean {
  return artifacts.some(
    (artifact) =>
      kinds.includes(artifact.kind) &&
      artifact.productMatched !== false &&
      namesMatch(artifact, rate),
  );
}

function isManufacturerOrExporterQuote(quote: SupplierQuote): boolean {
  return quote.supplierRole === "manufacturer" || quote.supplierRole === "exporter";
}

function sameName(left: string | undefined, right: string | undefined): boolean {
  return left !== undefined && right !== undefined && normalizeName(left) === normalizeName(right);
}

function normalizeName(value: string): string {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, " ")
    .trim();
}

function missingExporterEvidence(
  quotes: readonly SupplierQuote[],
  artifacts: readonly EvidenceArtifact[],
  measure: TariffMeasure,
): string[] {
  const requirements = new Set<string>();
  const eligibleRates = [
    ...(measure.namedExporterRates ?? []),
    ...(measure.annexExporterRates ?? []),
  ];
  const hasAnyEligibleName =
    eligibleRates.length > 0 &&
    eligibleRates.some((rate) =>
      artifacts.some((artifact) => namesMatch(artifact, rate)),
    );

  if (!hasAnyEligibleName) {
    requirements.add("eligible_manufacturer_or_exporter_name");
  }

  if (!artifacts.some((artifact) => artifact.kind === "supplier_profile")) {
    requirements.add("supplier_profile");
  }

  if (
    !artifacts.some(
      (artifact) => artifact.kind === "invoice" || artifact.kind === "certificate",
    )
  ) {
    requirements.add("invoice_or_certificate");
  }

  if (!artifacts.some((artifact) => artifact.kind === "listing")) {
    requirements.add("product_listing");
  }

  if (quotes.some((quote) => quote.supplierRole === "distributor")) {
    requirements.add("distributor_is_not_exporter_proof");
  }

  if (quotes.some((quote) => quote.supplierRole === "marketplace_seller")) {
    requirements.add("marketplace_seller_is_not_exporter_proof");
  }

  if (quotes.some((quote) => quote.factoryDirectClaim === true)) {
    requirements.add("factory_direct_claim_requires_documentary_proof");
  }

  return [...requirements].sort();
}

function unresolvedConfidenceBps(quotes: readonly SupplierQuote[]): number {
  if (quotes.length === 0) {
    return 0;
  }

  if (
    quotes.some(
      (quote) =>
        quote.supplierRole === "distributor" ||
        quote.supplierRole === "marketplace_seller" ||
        quote.factoryDirectClaim === true,
    )
  ) {
    return 1500;
  }

  return LOW_CONFIDENCE_BPS;
}

function unlockableNamedExporterMarginBps(measure: TariffMeasure): number {
  const lowerVerifiedRates = [
    ...(measure.namedExporterRates ?? []),
    ...(measure.annexExporterRates ?? []),
  ]
    .map((rate) => rate.ratePctBps)
    .filter((rateBps) => rateBps < measure.residualRatePctBps);

  if (lowerVerifiedRates.length === 0) {
    return 0;
  }

  return measure.residualRatePctBps - Math.min(...lowerVerifiedRates);
}

function dutyPence(
  customsValueGbpPence: number | undefined,
  ratePctBps: number,
): number | undefined {
  if (customsValueGbpPence === undefined) {
    return undefined;
  }
  return Math.round((customsValueGbpPence * ratePctBps) / 10_000);
}
