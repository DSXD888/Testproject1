# Pre-Prep: L5+L6 Commercial Intelligence & Cross-Layer Coherence

## Domain: L5+L6 + Cross-Layer Coherence
## Files This Agent Can Edit

| File | Scope | Notes |
|------|-------|-------|
| `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` | Full file | Customs, FX, freight, marketplace |
| `docs/strategy/layers/layer-6-request-pipeline-automation.md` | Full file | Bid workflow, portal skills |
| `docs/strategy/layer-strategy.md` | Full file | Cross-layer governance, schema stability, testing evolution |
| `docs/strategy/cross-reference-index.md` | Full file | Navigation updates |
| `docs/strategy/README.md` | Full file | Overview and navigation |

**Do NOT directly edit L0–L4 layer docs.** If the cross-layer consistency audit reveals issues in those docs, add cross-domain notes at the end of the file you're editing, clearly labeled `<!-- CROSS-DOMAIN NOTE FOR L{N}: ... -->`.

**Exception:** If the reviewer identifies a cross-layer inconsistency that requires a ONE-LINE fix in an L0–L4 doc (e.g., wrong section reference, typo in an ADR number), you may make that fix directly. For anything larger, use cross-domain notes.

---

## Step 1: Pre-Prep Research

Read these documents in order:

1. **`AGENTS.md`** — Agent guidelines, rules, project overview
2. **`docs/strategy/layers/layer-5-expanded-commercial-intelligence.md`** — Read cover to cover (~600 lines). Pay special attention to:
   - Multi-source HS classification recall (5 sources)
   - Multi-family expansion to UK10 codes
   - Exporter/manufacturer resolution
   - Measure interaction engine (AD/CVD/safeguard/quota)
   - Conservative defaults (MFN, residual, standard VAT)
   - Classification freshness system
   - Marketplace skills (Alibaba, 1688, Amazon, PCS)
   - Dual FX engine and freight engine
   - Evidence-tiered accuracy measurement
   - Error types (10 customs-specific)
   - "No new tables, no new columns" guarantee
3. **`docs/strategy/layers/layer-6-request-pipeline-automation.md`** — Read cover to cover (~400 lines). Pay special attention to:
   - Bid workflow state machine
   - Constitutional prerequisites (operator approval, fresh decision)
   - Bid scope payload
   - Portal-specific skill architecture (Proactis workflow)
   - Audit trail requirements
   - RESUME_UNSAFE_AFTER_SUBMIT side-effect boundary
   - Workflow registry extensibility pattern
4. **`docs/strategy/layer-strategy.md`** — Read FULL doc (~2100 lines). This is critical for cross-layer coherence. Pay special attention to:
   - §8: Non-negotiable constraints
   - §9: Schema stability matrix (column rules, authority classification)
   - §10: Stage scope matrix, interface contracts
   - §11: Testing evolution, performance targets
5. **`docs/strategy/cross-reference-index.md`** — Understand navigation structure
6. **`docs/strategy/README.md`** — Overview and doc-set description
7. **ALL other layer docs (L0–L4)** — Skim each to verify cross-references are accurate. Don't deep-read, but check:
   - ADR references match the ADR definitions in the research doc
   - Table names in layer narratives match L0 DDL
   - Interface contracts in each layer match layer-strategy.md §10
   - Test file counts in each layer match testing evolution table
   - Forward-compatibility stubs reference the correct later layer
8. **`docs/planmaxer/active/handoffs/handoff-l5l6-commercial-coherence.md`** — Read the handoff that produced the feedback

Build a deep mental model of:
- The customs classification algorithm end-to-end
- How conservative defaults affect every downstream calculation
- The bid workflow safety model
- The cross-layer dependency chain (L0 schema → L1 runtime → L2 desktop → L3 hardening → L4 sync → L5 intelligence → L6 automation)
- Where each interface contract is defined and where it's referenced

---

## Step 2: Feedback Integration

The architecture is fundamentally sound. L5 is the right customs design, L6 is the right last-mile automation layer, and the sequencing still makes sense. The real risk is not that the system is too ambitious. The real risk is that several L5/L6 behaviors are only described in prose, while the cross-layer contracts, state model, and document governance are not tight enough yet.

One hard limit on my audit: the handoff includes `layer-strategy.md`, `layer-5`, `layer-6`, `README.md`, and `cross-reference-index.md` in full, but not the full L0-L4 layer docs. So I can confirm contradictions inside the provided docs, and I can identify high-probability schema/contract gaps against L0, but I cannot do a literal line-by-line DDL verification of L0-L4 from this handoff alone.

Three external facts need patching in the docs now. First, HMRC’s official developer portal calls FPO the **Fast Parcel Operator API**, not “First Point of Origin.” Second, UK CBAM is now set for **aluminium, cement, fertiliser, hydrogen, and iron & steel** from **1 January 2027**, with a **trial sector rate in Q4 2026**; glass and ceramics are **not** in scope for 2027, and the detailed rules are still moving through 2026 legislation. Third, HMRC is issuing stop-press updates mid-cycle for commodity-code structure, VAT measures, preferences, MFN corrections, and trade-remedy registrations, so freshness cannot be treated as “mostly annual plus TRA only.” ([UK Trade Tariff][1])

## What is right

Your four core instincts are right.

Conservative defaults as a constitutional rule are the correct asymmetry. Multi-family expansion is the correct antidote to silent classification failure. Human approval before external portal submission is necessary. The multi-doc decomposition is better than one mega-doc.

HMRC’s tariff APIs do expose the right official primitives for this architecture — commodity data, duties, VAT, quotas, and trade measures — and HMRC’s exchange-rate service gives you the official customs FX stream. Browser-first Amazon is also the right choice because Amazon’s own PA-API 5.0 documentation now says deprecation is coming on 30 April 2026. Freightos and SeaRates both have active developer/API surfaces, so the dual-source freight design is grounded in real integrations, not wishful thinking. ([UK Trade Tariff][1])

## Confirmed contradictions and high-risk coherence gaps

These are the ones I would fix before anyone starts building L5/L6 against the text.

* **Doc-set authority drift.** `layer-strategy.md` says the maintained planning set is **exactly 8 files** and explicitly excludes `README.md` and `cross-reference-index.md` from planning authority. `README.md` says the maintained doc set is **10 files**. `cross-reference-index.md` also frames the maintained set as the 10-doc set. This is already a governance split-brain.

* **Portal target drift in L6.** `layer-strategy.md` §8 says L6 can prepare and submit on “Find a Tender or Contracts Finder.” `layer-6` §1.1 explicitly says that is the wrong abstraction and that real submission happens on downstream eSourcing portals such as Proactis/Jaggaer/Atamis/PCS-Tender. The local L6 doc is right. The cross-layer strategy summary is wrong.

* **Recall-source count drift in L5.** `layer-strategy.md` §7 describes multi-source recall as four sources. `layer-5` §1.4 defines five and includes **ATR/codebook reuse**. That fifth source matters a lot because it is the highest-value legal evidence tier.

* **State-model hole for saved drafts.** `layer-6` §1.3 gives `not_started → ready_to_bid → bid_in_progress → bid_submitted | bid_failed`. But `layer-6` §1.7 and §2.2 explicitly define a `reviewRequired = true` path where the skill **saves draft instead of submitting**. There is no state for that. Right now the state machine literally cannot represent one of its own happy paths.

* **Side-effect boundary is misnamed and too late.** `layer-6` §1.7 uses `RESUME_UNSAFE_AFTER_SUBMIT`, but the same step table admits draft autosave risk in step 6 and puts hard side-effect boundaries at step 7 and step 8 before final submit. The boundary is not “after submit.” It is “after remote mutation.” This is a real safety gap, not naming nitpicking.

* **Optional portal ref vs required locator.** `layer-6` §1.4 makes `portalOpportunityRef` optional, but `workflow-registry.ts` in §1.6 defines `locateOpportunity(session, ref: string)`. A mutating workflow needs a deterministic target. Optional is not good enough here.

* **Undefined L6 contracts.** `BidDocument[]` and `SubmissionResult` appear in the workflow interface in `layer-6` §1.6, but they are not defined anywhere in the provided contract surface. This is exactly the kind of “hidden contract” that causes cross-layer drift later.

* **Numeric typing drift against the constitutional schema rules.** `layer-strategy.md` §1 says confidence/percentage fields use integer basis points with `_bps` suffix. But `layer-5` uses `fxBasisDeltaPct`, `vatRecoveryRate`, and in §1.6 documents `exporterIdentityConfidence` as `0.0-1.0`, while §2.4 later documents `exporter_identity_confidence_bps`. That is both a cross-layer rule break and an internal L5 inconsistency.

* **Stage-model gap for L5 classification lattice.** `layer-strategy.md` §10 locks `process_stage_status.scopeKind` to six grains. L5 introduces a non-trivial **candidate × commodity-code** lattice, but there is no cross-layer contract saying whether that lattice is an internal sub-grain of the `candidate` stage, or whether its persistence has set-replacement semantics. Right now the most important new L5 computation has no clearly owned stage contract.

* **Testing-count drift.** `layer-strategy.md` §11 says L1 adds 7 test files, but the L1 section lists 8 named files. L2’s test list reuses filenames that already appeared in L1, so the cumulative count is not auditable as written. L5 adding only one new file and L6 adding zero is also not believable relative to scope.

* **Possible missing table introduction.** `layer-strategy.md` §9 includes a `sources` table in the schema matrix, but the L1 “Data Introduced” section in the provided strategy text does not introduce that table. That may be an omission in prose, but right now it is a matrix-to-layer mismatch.

## Belief 1: conservative defaults are constitutional

Keep them. Do not weaken them.

But they will create **systematic false negatives** in predictable places:

* exporter-specific remedy categories
* preference/suspension/quota-sensitive categories
* reduced/zero-VAT categories
* freight-boundary-sensitive bulky goods
* any product where supplementary units are required but not yet resolved

UK duty suspensions and ATQs are MFN-basis instruments available while in force, and HMRC explicitly says importers should use the most advantageous tariff concession when more than one applies. HMRC also treats DE 6/17 national additional codes as legal declarations for VAT relief/reduction claims. So a constitutional MFN / residual / out-of-quota / standard-VAT anchor is commercially safe, but it is **structurally pessimistic** wherever documentary or procedural claims are a real lever. ([GOV.UK][2])

The right answer is **not** to show “optimistic.” The right answer is to show:

* **Conservative landed cost** — your constitutional number
* **Unlockable upside** — how much margin is hidden behind missing proof
* **Evidence required** — exporter ID, origin proof, supplementary unit, quota allocation, VAT additional code, etc.

That gives you opportunity discovery without lying.

I would add two explicit L5 outputs on `landed_cost_estimates` or its JSON contract:

* `unlockableMarginBps`
* `evidenceLeverageSummaryJson`

That lets the system surface a case like “Conservative margin 5%, but named-exporter proof is worth +35pp” without relaxing the default.

The categories most likely to get unfairly buried are exactly the ones you already worry about: steel-family goods, aluminium-family goods, ceramics/tiles/tableware, some packaging subfamilies, and goods benefiting from suspensions, ATQs, or VATZ/VATR treatment. The bias is not random. It will correlate with importer sophistication and documentation quality.

A second-order risk: if the UI collapses VAT rate and VAT recovery into one cost number, the platform will over-penalise VAT-registered importers. Your architecture is right to separate them; now make sure the UI and downstream formulas never re-merge them.

## Belief 2: expand all plausible families, never greedy descent

Also right.

This is computationally tractable **if and only if** you separate the classification lattice from the commercial scenario lattice.

The raw classification side is manageable. Your own worst-case example — 5 plausible families × 20 CN8 branches × 10 UK10 descendants = 1,000 UK10 candidates per product — is fine **locally**. Fifty products at 1,000 descendants each is 50,000 local scoring operations. That is not scary.

What is scary is the full cartesian product once you mix in commercial scenarios:

`codes × quotes × packing scenarios × incoterms × exporter bases × quota states × FX snapshots`

A modest 200-code candidate set with 3 quotes, 2 packings, 3 incoterms, 3 exporter bases, 2 quota states, and 2 FX snapshots is already **43,200 scenario tuples**. That is where engines die.

So the rule should be:

1. **Expand all codes locally**
2. **Score all codes locally**
3. **Validate only top-K non-dominated branches externally**
4. **Late-bind commercial scenarios only after code-family pruning**

That preserves your non-greedy rule without melting the system.

### What makes it tractable

Use HMRC tariff data as a **locally indexed graph**, not as a per-code online traversal. Runtime should not be “call HMRC once per descendant.” Runtime should be “read descendants, measures, supplementary-unit requirements, and additional-code requirements from local cache; use live HMRC verification only for shortlisted candidates.” HMRC’s official tariff APIs are exactly the kind of structured feed you can cache this way. ([UK Trade Tariff][1])

My recommended execution split:

* **Recall pass:** FPO + CPV prior + rule hints + tariff keyword + ATR/codebook
* **Local expansion:** all descendants from cached HMRC hierarchy
* **Local scoring:** slot compatibility, exclusions, unit requirements, measure family fit, source independence
* **Shortlist:** top 3–5 branches per non-dominated family
* **External validation:** HMRC/Taric Support/HS Ping only on shortlist
* **Commercial scenarioing:** only after shortlist

### The stage-contract problem you need to fix

Right now the stage model does not say where the candidate-code lattice lives. You need one of two explicit choices in `layer-strategy.md` §10:

* **Choice A:** classification lattice is an internal sub-grain of the `candidate` stage; `product_commodity_codes` is set-replacement for the whole candidate
* **Choice B:** define and document a non-stage “sub-grain” for candidate-code persistence, even if `process_stage_status` still stays on six canonical grains

Without this, stage atomicity for L5 is fuzzy.

### How to present 50+ codes to an operator

Never flat-list them.

Default view should be **family clusters**, not codes:

* family label and count of surviving UK10 descendants
* conservative duty outcome for that family
* most-supported branch inside the family
* blocker reason
* top missing facts that would collapse the branch set

What the operator wants is not “83 codes.” What they want is:

* “Fasteners family: 63 codes, remedy-active, exporter identity matters”
* “Tube fittings family: 12 codes, no remedy, need end-use/function”
* “Other articles family: 8 codes, residual basket, low duty spread”

Then show a short list of **entropy-killing questions**:
“Is it a threaded fastener?”
“Is it a tube fitting?”
“Do we know exporter/manufacturer?”
“Can we express net kg?”

That is how you make multi-family expansion usable.

## Belief 3: no real-money bids without human approval

Necessary. Not sufficient.

Your current L6 safety model fences **money**, but not **portal-side effects**.

The right safety principle is:

**No externally visible portal mutation beyond the approved side-effect class without human approval.**

That is the real rule.

### Current L6 gaps

The prerequisites in `layer-6` §1.3 only require operator approval and staleness checks. They do **not** require:

* no `CLASSIFICATION_AMBIGUOUS`
* no `TARIFF_BRANCH_DIVERGENCE`
* no `SUPPLEMENTARY_UNIT_MISSING` on required duty components
* no `decision_grade = broker_required`
* acceptable quote roles for bidding
* current document bundle hashes
* current company profile documents/certificates
* classification freshness still valid for the expected import window

That means L6 can currently become `ready_to_bid` even when L5 is still telling you “customs not actually settled.”

That cross-layer gate needs to be tightened.

### Side-effect classes I would add

| Class                  | Example                                                                  | Retry policy                     |
| ---------------------- | ------------------------------------------------------------------------ | -------------------------------- |
| `READ_ONLY`            | login, navigate, verify tender                                           | safe to retry                    |
| `REMOTE_DRAFT`         | opening response, autosave, save draft, document upload                  | verify remote state before retry |
| `NOTIFICATION_OR_LOCK` | buyer-visible status change, addenda acknowledgement, slot/response lock | human reconfirm before retry     |
| `LEGAL_DECLARATION`    | accepting portal declarations/attestations                               | human reconfirm before retry     |
| `FORMAL_SUBMISSION`    | final submit                                                             | never blind retry                |
| `FUNDS_COMMITTED`      | deposits/fees if any future portal requires them                         | never blind retry                |

For the current Proactis flow, steps 6–8 are already beyond `READ_ONLY`, and step 10 is not the first dangerous boundary.

### State-machine fix

At minimum, add representable states or sub-statuses for:

* `draft_saved`
* `portal_state_unknown`
* `verify_only_required`
* `submission_unconfirmed`

You do **not** necessarily need new tables for this if `bid_workflow_states` already has JSON metadata and a flexible text status field. But you absolutely need the state model.

### Workflow contract fix

`workflow-registry.ts` needs more than the current methods. Add something like:

* `verifyPortalState(...)`
* `saveDraft(...)`
* `submitFinal(...)`

Do not overload `submit(reviewRequired: boolean)` to mean both “save draft” and “submit.” Those are different side effects and different states.

### Validation-policy conflict you need to resolve

This one is serious.

`layer-strategy.md` makes **fail-open validation** constitutional in L3/L4. `layer-6` says bid automation skills use `validation_policy: always` because portal actions matter. Those two rules conflict.

If validator failures still fail open during L6 final submission, then validator outage does **not** actually protect the portal. That defeats the point of “always”.

My recommendation:

* **fail-open remains fine for ordinary pipeline production**
* **final portal submission should be fail-closed on validator rejection or validator outage**
* if validation is unavailable, the workflow may still reach `draft_saved`, but not `bid_submitted`

That preserves your Layer 3 philosophy without letting a validator outage greenlight a mutating external action.

## Belief 4: the 10-doc split should stay coherent

The split is better than a mega-doc.

But only if cross-layer facts are **single-sourced** and the navigation docs stop carrying normative content.

Right now, the drift is already real. So yes, the decomposition helps, but only with a docs-as-data discipline.

### What I would do

Define three authority tiers explicitly:

* **Normative planning docs:** 8 files
* **Maintained navigation docs:** `README.md`, `cross-reference-index.md`
* **Non-authoritative review artifacts:** handoffs, prompts, notes

Then make the wording identical in all files.

### Add machine-readable registries

Create a small registry that generates the cross-layer tables:

* `tables.yaml`
* `contracts.yaml`
* `adrs.yaml`
* `tests.yaml`
* `glossary.yaml`
* `sections.yaml`

Generate from those:

* schema stability matrix
* interface contract inventory
* maintained-doc count
* ADR table
* test count tables
* cross-reference index anchors

That is how you keep 10 docs coherent without merging them.

### CI lint rules that matter

I would block merges on these:

* every L5/L6 “field now populated” exists in the L0 DDL
* every later-layer JSON payload has a schema in `packages/contracts-schema`
* every status enum used in prose exists in a typed registry
* every test-count table matches actual files
* every ADR number maps to one title only
* every cross-reference anchor resolves
* every glossary term has one canonical expansion

### The hidden contracts that should be promoted now

These are not optional anymore:

* `CommodityCandidateSet`
* `DutyBreakdown`
* `MeasureInteractionResult`
* `DecisionBasisHashPayload`
* `BidScopePayload`
* `BidDocument`
* `SubmissionResult`
* `PortalMutationCheckpoint`

Until these move into `packages/contracts-schema`, L5/L6 are not really “execution-grade.” They are narrative-grade with typed holes.

## L5 testing and sustainability

The current testing story is too light for the stated scope.

L5 adds 19 intelligence files, four skills, multi-source classification, exporter-rate basis logic, supplementary-unit blockers, dual FX, dual freight, freshness/watchers, and ten error types. One new test file is not credible.

L6 adds a whole stateful portal automation layer. Zero new test files is also not credible.

### Minimum extra tests I would require

For L5:

* measure interaction golden cases by family
* supplementary-unit required vs resolved vs missing
* named exporter vs annex vs residual
* tariff-news refresh and reclassification triggers
* candidate family clustering and blocker logic
* `CLASSIFICATION_AMBIGUOUS` vs numeric spread-only tolerances
* evidence-tier scoring by gold/silver/bronze
* conservative-to-actual uplift calibration by category

For L6:

* `reviewRequired=true` draft-save happy path
* crash after step 7, 8, and 10 with portal reconciliation
* duplicate-detection / already-submitted detection
* validator rejection before final submit
* validator outage before final submit
* wrong-portal / wrong-tender prevention
* operator approval scope vs side-effect cap

### Is the testing strategy achievable?

Yes, but not if you treat the full gate as a per-commit ritual.

The only sustainable cadence is:

* **per commit:** fast inner loop + replay fixtures
* **nightly:** live smoke + drift monitors
* **weekly:** benchmark slice / category accuracy runs
* **promotion or release:** full gate

If “no mocks” is interpreted literally everywhere, the strategy will collapse. The L3 replay suite already tells you the real sustainable interpretation: no mocked business truth in acceptance gates, but heavy use of replays and captured artifacts for deterministic regression.

### Annual maintenance cost

Rough order of magnitude:

* **human maintenance:** about **0.5–1.0 FTE** once you have 3+ categories, because gold sets, validator corpora, and live-source drift triage are labour-heavy
* **variable run cost:** usually **low five figures or less annually** if you keep full gates to nightly/weekly/promotion cadence rather than every change

Labour will dominate spend. Not APIs.

## The concerns you did not ask about, but should

1. **Additional-code satisfiability is as important as rate discovery.** HMRC makes clear that DE 6/17 codes are legal declarations and can trigger repayment demands and sanctions if claimed incorrectly. Your engine must model not just “this measure exists” but “we can truthfully satisfy the code and document prerequisites.” ([GOV.UK][3])

2. **Your freshness model is too narrow.** TRA watcher is necessary, but HMRC tariff news is changing code structure, VAT measures, tariff preferences, MFN data, and remedy registrations mid-cycle. Freshness needs at least four vectors: structure, measures, remedies, and FX. ([GOV.UK][4])

3. **CBAM is no longer just an iron/steel/aluminium placeholder problem.** For tenders that run into 2027, cement, fertiliser, and hydrogen now matter too, while glass/ceramics do not for 2027. If the tender delivery window crosses 1 January 2027, you need a future-regulatory-exposure flag even if `cbamEstimateGbp` stays null for now. ([GOV.UK][5])

4. **Bundle splitting can be legally wrong.** Some retail sets or kits should be classified as sets under GIR 3, not decomposed item by item. Your bundle splitter needs a legal “set-candidate” escape hatch, or it will silently misclassify the hard cases.

5. **Taric Support / HS Ping are useful, but UK-specific divergence matters.** They are fine as cross-checks, but they should never overrule HMRC on UK national measures, VAT additional codes, quotas, or UK trade-remedy implementation.

6. **`decisionBasisHash` is now mission-critical and underspecified.** It needs to cover L5 freshness inputs, selected quotes, company profile version, document hashes, side-effect class cap, and portal target reference. If it does not, your L6 staleness protection is cosmetic.

7. **Ready-to-bid should be blocked by quote-role quality, not just approval.** An opportunity backed only by `domestic_wholesale_floor` or `optimistic_floor` evidence should not be bid-ready unless your policy explicitly says so.

## Bottom line

Keep the architecture. Do not back off the conservative defaults. Do not back off multi-family expansion. Do not collapse the docs into one file.

But fix the contract and state-model debt before build:

* tighten L5/L6 into formal contracts
* repair the L6 state machine
* replace the single submit-boundary flag with graded side-effect classes
* add a real L5→L6 readiness gate
* correct the external factual drift
* generate the cross-layer tables from one source of truth

If you do that, the strategy is strong. If you do not, the codebase will become internally inconsistent long before customs logic becomes the bottleneck.

[1]: https://hub.trade-tariff.service.gov.uk/ "https://hub.trade-tariff.service.gov.uk/"
[2]: https://www.gov.uk/guidance/duty-suspensions-and-tariff-quotas "https://www.gov.uk/guidance/duty-suspensions-and-tariff-quotas"
[3]: https://www.gov.uk/government/publications/cds-uk-trade-tariff-volume-3-import-declaration-completion-guide/group-6-goods-identification "https://www.gov.uk/government/publications/cds-uk-trade-tariff-volume-3-import-declaration-completion-guide/group-6-goods-identification"
[4]: https://www.trade-tariff.service.gov.uk/news/stories/update-to-commodity-code-structure-to-maintain-dynamic-alignment-with-the-eu-stop-press--18-march-2026?utm_source=chatgpt.com "Update to Commodity Code Structure to maintain dynamic ..."
[5]: https://www.gov.uk/government/publications/carbon-border-adjustment-mechanism-cbam-policy-summary/carbon-border-adjustment-mechanism-cbam-policy-summary "https://www.gov.uk/government/publications/carbon-border-adjustment-mechanism-cbam-policy-summary/carbon-border-adjustment-mechanism-cbam-policy-summary"


### Integration Rules

For each proposed change from the reviewer, evaluate:

1. **Agree strongly** → Apply directly to the appropriate file
2. **Agree partially** → Apply the useful parts, note what you modified and why
3. **Disagree** → Do NOT apply. Note the disagreement and reasoning at the end of your response

### Specific Guidance

- **Customs engine changes:** Acceptable for L5. Changes to the classification algorithm, measure interaction rules, or conservative defaults should be precise and cite UK customs regulations where possible.
- **Multi-family expansion tractability fixes:** If the reviewer proposes caching, pruning, or complexity bounds, evaluate against the "never greedy descent" principle. You can add optimization (memoization, early pruning of incompatible codes) but NOT change the principle of expanding all plausible families.
- **Bid safety model changes:** Acceptable for L6. New side-effect classifications, additional safety checks, or recovery mechanisms. Preserve: "no real-money bids without human approval" and "explicit bid scope."
- **Cross-layer consistency fixes:** This domain has the unique ability to fix cross-layer issues. For L5/L6/layer-strategy.md: apply directly. For L0–L4: one-line fixes directly, larger fixes via cross-domain notes.
- **Schema stability matrix updates:** Changes to layer-strategy.md §9 affect ALL layers. Be very careful. Only add rules; don't remove or weaken existing rules.
- **Testing evolution updates:** Changes to layer-strategy.md §11 (test counts, testing lanes) must be verified against each layer's test specification.
- **Interface contract updates:** Changes to layer-strategy.md §10 must be verified against the defining layer doc.

### Preservation Rules

- Preserve conservative defaults as constitutional
- Preserve "no real-money bids without human approval" as constitutional
- Preserve "no new tables, no new columns in L5/L6" guarantee
- Preserve all ADR numbers and references
- Preserve authority classifications
- Preserve multi-family expansion principle (never greedy descent)
- Preserve the 10-file maintained doc set structure

---

## Step 3: Fresh-Eyes Optimization

After completing the edits, re-read L5, L6, and layer-strategy.md with fresh eyes. Then skim ALL other layer docs. Check for:

### L5 Specific

1. **Customs algorithm completeness:** Walk through a complete classification for a concrete product (e.g., "stainless steel hex bolts from China"). Does every step in the algorithm have enough specification to implement?
2. **"No new columns" guarantee:** Verify EVERY column referenced in L5 narratives exists in L0 DDL (check layer-0-gold-set-evaluation.md).
3. **Error type completeness:** Are the 10 customs error types sufficient? Walk through each classification failure mode and verify there's an error type for it.
4. **Evidence tiers:** Gold/silver/bronze labels are defined but how are they assigned? Is the assignment procedure specified?

### L6 Specific

5. **State machine completeness:** Walk through every transition in the bid workflow state machine. Are there missing transitions (e.g., cancel-in-progress, timeout)?
6. **Audit trail completeness:** For every step in the 11-step Proactis playbook, verify a corresponding audit artifact is specified.
7. **Prerequisite check robustness:** The prerequisites (operator approval + fresh decision) are checked at bid start. What if the decision becomes stale DURING the bid? Is there a mid-flow check?

### Cross-Layer Coherence

8. **ADR consistency:** For each ADR (001-008), verify the number, title, and decision are referenced consistently across ALL layer docs.
9. **Interface contract consistency:** For each of the 14 interface contracts, verify the definition (in the defining layer doc) matches the reference (in layer-strategy.md §10) and any usage (in consuming layer docs).
10. **Schema stability completeness:** Verify the schema stability rules in layer-strategy.md §9 cover ALL tables created in L0. No table should be missing from the stability classification.
11. **Test count accuracy:** Verify the cumulative test counts (4 → 11 → 16 → 32 → 35 → 36 → 36) match the actual test specifications in each layer doc.
12. **Stage scope matrix coverage:** Verify every pipeline stage referenced in any layer doc appears in the stage scope matrix (layer-strategy.md §10).
13. **Terminology consistency:** Spot-check key terms across all docs: "session lifecycle" vs "execution lifecycle," "skill directory" vs "skill standard," "run manifest" vs "execution manifest." Flag any drift.

---

## Step 4: Sync Pass

1. Update `docs/strategy/cross-reference-index.md`:
   - Add any new customs/trade concepts
   - Add any new interface contracts
   - Verify all existing entries still point to correct sections
2. Update `docs/strategy/README.md`:
   - Verify doc-set description is current
   - Update any line counts if docs grew substantially
3. Update `docs/strategy/layer-strategy.md`:
   - §8: Add any new non-negotiable constraints from reviewer feedback
   - §9: Update schema stability matrix if new rules added
   - §10: Update interface contracts or stage scope matrix if changed
   - §11: Update testing evolution if test counts changed
4. Compile all cross-domain notes from this and other preprep domains into a clear list at the end of your response, organized by target layer.

Write all updates using as-if-always language (not changelog style).
