# SPIKE-Z-002 Installer-Quote Ingestion

## Scope

Requirement: `FR-Z.2 Installer-Quote Ingestion`.

Scope statement from the spec: ingest a UK installer subcontract quote and use it as evidence that can override or calibrate table-based labour estimates.

Extension point from the spec: `SupplierQuoteRow.quote_class = "installer_quote"` and selected-quote/value-share synthesis.

Research reference: research section 10, "Sub-contractor quote ingestion as `quote_class = installer_quote`."

This is a Tier-B research/design output. It does not implement production behavior, add tests, or change Tier-A row semantics.

## Current Tier-A Surface

`SupplierQuoteRow.quote_class` already accepts `installer_quote` as a reserved extension-point value. Marketplace quote persistence already carries `quote_class` from source metadata into `supplier_quotes`, and the gross-margin stage already separates quote identity, usability, realism, selected quote, revenue basis, and selected supply quote fields.

The landed-cost engine now prices installation labour independently from the selected goods supply quote. The decision-grade path builds a landed-cost candidate, applies the Phase-2 goods-dominance gate, and prefers a contract-value goods share when present before falling back to a cost proxy. The cost proxy denominator is goods landed cost plus installation labour plus permits/traffic management. Maintenance remains opex outside the capital-event dominance denominator.

## Research Finding

A real UK installer subcontract quote is stronger evidence than a generic regional rate table only when it is traceable to the same site, period, scope, and responsibility boundary as the tender. The quote needs enough structure to prove what it covers:

- site or region, including whether travel, call-out, or mobilisation is included
- installation scope, required trades, unit count, and access-equipment assumptions
- permit and traffic-management responsibility, if any
- whether VAT is included, excluded, or unknown
- validity period and quote date
- exclusions such as making good, electrical certification, civil works, out-of-hours work, de-install, disposal, permits, or lift/MEWP hire
- evidence source, supplier identity, and enough raw text or attachment reference for audit

If that evidence is missing, the quote is still useful as calibration data, but it should not silently replace the table-based labour estimate. The existing screening-grade rule still applies: the platform is not issuing firm installation prices.

## Decision

Use verified `installer_quote` rows as per-estimate override evidence for the table-based installation labour component. Also record every accepted or rejected installer quote as calibration evidence for future rate-table tuning.

Override is the per-estimate behavior: when a quote passes evidence gates and maps to the same candidate/site/scope, its ex-VAT pence value becomes the authoritative labour cost input for the covered scope. The table estimate remains in the basis as a comparator and anomaly signal, not as the value used for that covered labour scope.

Calibration is the longitudinal behavior: quote-to-table deltas are accumulated by region, scope, trade, access-equipment band, and quarter, then used by a future rate-refresh or review process to tune reference tables. Calibration does not change a live estimate unless a specific quote also passes the override gates.

## Rationale

Installer quotes are concrete commercial evidence, so using them only as soft calibration would keep stale or over-broad labour tables in the decision path even after better estimate-specific evidence exists. At the same time, blindly overriding on any quote-shaped text would make the economics brittle because many installer quotations exclude permits, access equipment, VAT, certification, or de-install work.

The split keeps the pipeline conservative: strong scoped evidence can improve a specific estimate, weak evidence routes to review and calibration only, and all accepted evidence stays auditable.

## Rejected Alternatives

Table-only calibration: rejected because it ignores the main value of FR-Z.2, which is to use a real subcontract quote as estimate-specific evidence.

Unconditional override: rejected because incomplete or lump-sum quotes can understate labour, permits, access equipment, or VAT and would create false pass decisions.

Treat installer quotes as goods supply quotes: rejected because it would pollute product supply selection and gross-margin supply-cost logic. `installer_quote` is labour evidence, not a replacement for the selected goods quote.

Cap-table reconciliation: rejected for the first implementation because blending table and quote values hides which evidence was authoritative. A future implementation can show both values, but the selected basis should be explicit.

## Extension-Point Design

### Ingestion

An installer quote is stored as a `SupplierQuoteRow` with `quote_class = "installer_quote"`. The row should retain existing money discipline: pence integers for money and bps integers for confidence. VAT treatment must be explicit in `vat_inclusive` or evidence metadata. If the source quote is not GBP, conversion follows the same minor-unit and FX discipline as other quote rows.

The future ingestion skill or operator flow should populate normal quote fields where they already fit:

- `product_candidate_id`: candidate the labour quote supports
- `product_name`: tender/candidate label or installer quote line label
- `region`: UK region or site-derived region
- `source_name`, `source_url`, `source_listing_id`: supplier and evidence reference
- `unit_price_original_minor`, `currency`, `unit_price_gbp_pence`: normalized ex-VAT labour price when known
- `offer_total_gbp_pence`: project-level quote total when scope is project-level
- `quote_cost_boundary`: `project`, `visit`, `unit`, or another explicit boundary if later specified
- `quoted_quantity` and `price_basis_unit`: units, visits, day rate, or project scope where available
- confidence fields: identity, usability, shipping-basis if not applicable left neutral or unused, and realism confidence
- `evidence`: structured JSON string or text that includes scope, exclusions, VAT state, validity, and raw evidence reference

Do not change the existing `SupplierQuoteRow` semantics in this spike. If future implementation needs first-class labour quote columns, that must be a new spec/TDD update.

### Evidence Gates

An installer quote is override-eligible only when all of the following are true:

- `quote_class = "installer_quote"`
- attached to the same `product_candidate_id` as the installation requirement
- price is positive and normalized to GBP pence
- quote date or validity period is present and not stale under the future configured threshold
- scope maps to the installation requirement's trade/profile/equipment assumptions
- VAT state is explicit enough to derive an ex-VAT screening value
- exclusions do not remove a required cost component without a separate table fallback
- match/usability/realism confidence clears the same broad gates used for other quotes, with a labour-specific realism check against the table band

If the quote covers only part of the labour scope, it overrides only that covered component and the table model fills the remainder. If coverage cannot be split, route the estimate to review and keep the table estimate as the base value.

### Selected-Quote Synthesis

The selected goods supply quote remains the `candidate_pricing_assessments.selected_quote_id` and `selected_supply_quote_id`. An installer quote must not become the selected goods quote and must not affect supplier-spread calculations.

The future landed-cost decision basis should carry a separate selected installer quote marker, most likely in `confidence_breakdown` or another landed-cost evidence payload until a new row field is specified:

- `selected_installer_quote_id`
- `selected_installer_quote_row_version`
- `installer_quote_basis`: `override`, `partial_override`, `calibration_only`, or `rejected_for_override`
- `table_labour_base_gbp_pence`
- `installer_quote_labour_gbp_pence`
- `quote_vs_table_delta_bps`
- `override_scope`: covered trades, unit count, access equipment, permit coverage, and exclusions

The decision-basis hash must include the installer quote id, row version, normalized labour pence, validity marker, and evidence hash whenever the quote affects the estimate.

### Value-Share Synthesis

Contract-value goods share remains preferred when available. Installer quotes do not change that precedence.

For the cost-proxy basis, an override-eligible installer quote replaces the table-derived installation labour cost for the covered capital-event scope. Permits and traffic management stay separate line items and are not folded into labour. Maintenance remains opex and is not capitalised into the goods-dominance denominator.

If an installer quote is calibration-only or rejected for override, it must not reduce the goods-share denominator or increase the margin. It can lower confidence or route to review if it indicates the table estimate may be materially wrong.

Near-floor goods-share cases still route to review rather than auto-reject, matching the Tier-A dominance policy.

## Future Implementation Shape

The next implementation bead should replace this spike coverage with real requirement IDs and named tests before shipping behavior. Suggested implementation surfaces:

- ingestion path for `quote_class = "installer_quote"` evidence, either through a dedicated operator/import flow or a scoped source registry entry
- labour quote selection helper that filters `supplier_quotes` by `quote_class`, candidate, scope, validity, and confidence
- landed-cost integration that overrides only the installation labour component and records table-vs-quote basis data
- decision-basis hash extension for selected installer quote evidence
- manifest/UI surfacing of table estimate, installer quote, delta, exclusions, and selected basis
- calibration capture for quote-to-table deltas without mutating reference tables in place

No code or code tests were added for this spike.
