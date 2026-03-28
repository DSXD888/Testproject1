export type OpportunitySection = {
  id: string;
  title: string;
};

export const OPPORTUNITY_SECTIONS: readonly OpportunitySection[] = [
  { id: "summary", title: "Summary" },
  { id: "lot-analysis", title: "Lot Analysis" },
  { id: "product-candidates", title: "Product Candidates" },
  { id: "pricing-evidence", title: "Pricing Evidence" },
  { id: "margin-estimate", title: "Margin Estimate" },
  { id: "coverage-metrics", title: "Coverage Metrics" },
  { id: "risk-flags", title: "Risk Flags" },
  { id: "recommendation", title: "Recommendation" }
] as const;

export const renderOpportunityMarkdown = (
  sectionBodies: ReadonlyMap<string, string>
): string => {
  return OPPORTUNITY_SECTIONS.map(({ id, title }) => {
    const body = sectionBodies.get(id)?.trim() ?? "_Section unavailable._";
    return `## ${title}\n\n${body}`;
  }).join("\n\n");
};
