# Sorellon — UK Public Procurement Bidding Readiness — FINAL Master Document

> **Single source of truth.** Combines comprehensive research + tailored workflow + operator clarifications into one document. Table of contents below.
> **Last updated**: 2026-05-26 (sourcing flexibility correction + pipeline alignment + customs analysis).
> **Prior versions** (still on disk for audit; not the authoritative reference): `2026-05-25-sorellon-bidding-readiness-research.md`, `2026-05-25-sorellon-mvs-tailored-workflow.md`.

---

## Table of Contents

| § | Section | What it covers |
|---|---|---|
| 0 | Executive summary | The overall picture in 1 page |
| 1 | **Step-by-step action plan** | Numbered 1-50: every concrete action to get from today → first bid |
| 2 | Locked-in decisions | What you committed to in the 5-round clarification |
| 3 | PRE-BID vs AT-BID-TIME vs POST-WIN | The single most important reframe |
| 4 | Year-1 budget — three tiers | Rock Bottom / Safer / Cautious |
| 5 | Multi-country sourcing — revised regulatory analysis | Corrects the China-only assumption from earlier |
| 6 | Customs declarations + DDP supplier terms | Sorellon often isn't the importer of record |
| 7 | Pipeline alignment | Which platform-qualified opportunities are bidable with MVS |
| 8 | EL insurance clarification | The 50/50 restructure interaction |
| 9 | 60-day weekly plan | Week-by-week execution |
| 10 | Statutory compliance + data protection | VAT, IV, Confirmation Statement, ICO, Modern Slavery |
| 11 | Insurance deep-dive | Brokers, cover levels, year-1 path |
| 12 | Certifications & supplier schemes | Cyber Essentials, ISO, Constructionline, SMAS, Living Wage |
| 13 | Procurement platform registrations | FTS, CDP, devolved nations, NHS Atamis, CCS |
| 14 | Product-specific regulatory | UKCA/CE, MHRA, REACH/CLP, BPR, PPE Reg, GPSR, EPR/PPT |
| 15 | Financial & operational maturity | Banking, credit rating, capital adequacy, references |
| 16 | Standalone policies | Social Value, QMS, DPA, Safeguarding, Logistics, etc. |
| 17 | Open questions | Decisions deferred to operator |
| 18 | Appendix: sources cited | All external URLs |

---

## 0. Executive Summary

**Sorellon Limited** (Companies House #16575318), incorporated 2025-07-10, pre-revenue B2B wholesale supplier of PPE / hygiene / cleaning / healthcare consumables / janitorial / education stationery / construction-adjacent goods. The company sources from multiple countries (UK, EU, China, global — whichever cheapest per product), targets UK public procurement at £10k-£100k contract size, intends to bid to LA + LA-affiliated + NHS (Trusts + NHS Supply Chain) + central government via CCS/GCA. MoD is explicitly out of scope for year-1.

### The big numbers

- **Rock-bottom PRE-BID cash**: **~£1,500-£2,000 year-1**. Enough to start bidding on LA + NHS Trust direct purchases at £10-£25k contract size.
- **Operator time**: ~15 hours/week for 60 days. Front-loaded weeks 1-4.
- **First bid target**: 30-60 days from today.
- **Capital injection** (director's loan or share allotment): £20k-£50k recommended, decided before week 5.

### Single highest-urgency item

🚨 **Companies House ECCTA identity verification for Dieter** — deadline 14 Dec 2025 lapsed ~5 months ago. Company strike-off risk. Resolve THIS WEEK via GOV.UK One Login (free, 15 min). Zac due 23 Jul 2026.

### Three critical reframes

1. **Most regulatory work is POST-WIN per-contract, not blanket year-1.** Bid responses describe what you WILL supply; audits and per-supplier verifications happen before first delivery, not before first bid. The "scary £4,600-£6,200 year-1 number" I quoted earlier conflated post-win costs with pre-bid costs.

2. **Multi-country sourcing reduces regulatory burden significantly.** UK suppliers + EU suppliers with UK Only Representative + DDP-Incoterm supplier shipments all keep Sorellon as a *distributor* (light burden) rather than *importer of record* (heavy burden). Sourcing from China-direct without DDP is the most regulatory-heavy path; sorellon.md says Sorellon sources "UK, EU, China, and trusted global locations" — the operator should preferentially route the regulated products (medical devices, biocidal products) through UK-supplier or EU-with-OR routes when commercially viable.

3. **The 50/50 ownership restructure helps EL exemption only if Dieter stays non-executive.** Procurement portals require EL certificate upload regardless of exemption status — so the EL purchase is a procurement decision, not a statutory one. £20-£60 marginal inside the combined business policy.

---

## 1. Step-by-step Action Plan

Numbered concrete actions from "today" to "first bid submitted." Every action has owner + estimated time + cost.

### Phase A — Critical-path emergency (this week)

```
[ ] 1. Companies House identity verification for Dieter
       — gov.uk/guidance/verify-your-identity-for-companies-house
       — 15 min · £0 · OPERATOR (Dieter)
       — Use GOV.UK One Login route
       — Risk: 5-month-overdue deadline; company strike-off possible

[ ] 2. Companies House identity verification for Zac
       — Same URL, same process
       — 15 min · £0 · OPERATOR (Zac)
       — Deadline 23 Jul 2026 — do now to bundle with Dieter

[ ] 3. Execute the 1% share transfer (Zac → Dieter, making 50/50)
       — Complete J30 stock transfer form (free download from gov.uk)
       — File SH01 if also changing total share capital
       — Update PSC register
       — 1 hour · £0-£15 · OPERATOR (Zac + Dieter)

[ ] 4. Decide on Dieter's employment status post-restructure
       — Non-executive PSC (no employment contract, no salary) → preserves EL exemption
       — Working employee → triggers mandatory EL
       — Recommendation: non-executive unless commercially needed
       — 15 min decision · OPERATOR

[ ] 5. Set up DIY accounting software
       — Pick: Xero (~£16/mo), FreeAgent (free with NatWest Mettle), or QuickBooks (~£14/mo)
       — Skip the £150/mo accountant retainer year-1
       — Engage accountant only at year-end (~£500-£1,500 one-off, ~3 mo before 2027-04-10)
       — 1-2 hours · £14-16/mo (£170-200/yr) · OPERATOR

[ ] 6. Pay ICO Data Protection Fee
       — ico.org.uk/registration · Tier 1 with direct debit
       — 15 min · £47/yr · OPERATOR

[ ] 7. File HMRC active-status notification
       — Inside HMRC business tax account
       — Only if any RFQ activity has begun (per Round 2, you're pre-trading, may not be needed yet)
       — 10 min · £0 · OPERATOR
```

### Phase B — Foundations (weeks 2-3)

```
[ ] 8. Submit Cyber Essentials application
       — iasme.co.uk/cyber-essentials/ · IASME direct (cheapest)
       — Prerequisites to verify first:
         · MFA enforced on Google Workspace admin
         · MFA enforced on Netlify admin
         · Full-disk encryption (BitLocker / FileVault) on all work devices
         · Supported OS versions only (no Windows 10 unattended)
         · Automatic security updates enabled
         · Password manager in use (Bitwarden, 1Password — not browser autofill for shared accounts)
       — Questionnaire ~12 hours of work
       — 1-3 weeks total elapsed · £396 · OPERATOR + AI assistance

[ ] 9. Insurance broker outreach
       — Call in order:
         a) Premierline 0333 320 6009
         b) Towergate 0330 123 5922 — ask for wholesale/distribution team
         c) Simply Business 0333 014 6683 — online quote first for benchmark
         d) Hiscox Broker channel 0800 042 0345
         e) Markel UK (via BIBA directory)
       — Script: "Sorellon Limited, Companies House 16575318, incorporated July 2025,
         pre-revenue, B2B wholesale distributor of PPE, hygiene wipes (EN1276/EN14476),
         healthcare consumables, cleaning chemicals, janitorial supplies, painting kits,
         stationery. Multi-country supply (UK, EU, China, global). Sole director, no
         employees beyond director, projected first-year turnover £100k-£250k. Registered
         office is residential (administrative only, no stock storage). Need combined
         business policy: £5M EL, £5M PL, £5M PrL aggregate. Can you quote?"
       — Mid-tier at £10M PL/PrL: optional upgrade if NHS Supply Chain frameworks are targeted
       — 3 hours operator time spread across calls · £0 upfront · OPERATOR

[ ] 10. HMRC GB EORI registration (do BEFORE first import)
       — gov.uk/eori — free, ~3 working days
       — Defer if no imports planned within 30 days
       — 30 min · £0 · OPERATOR

[ ] 11. Draft Documented QMS procedure
       — ISO 9001:2015 clause-shape structure (clauses 4-10)
       — Sections: Quality Policy, Scope, Process maps, Supplier qualification,
         Goods inspection & NCR handling, Customer-complaint procedure, Document
         control, Internal audit cadence, Continual improvement
       — ~16 hours · £0 · AI-assisted with operator review

[ ] 12. Draft Modern Slavery Statement (voluntary)
       — 2-3 pages, references pre-revenue first-year status honestly
       — Board approval (Zac signs directors' resolution)
       — Publish on sorellon.com/modern-slavery-statement
       — Submit to gov.uk Modern Slavery Registry
       — ~6 hours · £0 · AI-assisted

[ ] 13. Bind insurance policy
       — Select best broker quote (target £800-£1,200 for basic £5M tier, ~£1,500 for mid-tier)
       — Sign + pay deposit
       — Receive certificate within 1-3 business days
       — 30 min · £800-£2,000/yr depending on tier · OPERATOR
```

### Phase C — Tax + procurement registrations (weeks 3-4)

```
[ ] 14. HMRC VAT voluntary registration
       — gov.uk/register-for-vat
       — Have ready: company evidence pack (sample RFQ screenshots, supplier MOUs, website screenshots, bank statements)
       — Choose Cash Accounting Scheme (turnover ≤ £1.35M)
       — Plan for 30-60 working day HMRC processing
       — 1 hour application · £0 · OPERATOR

[ ] 15. File first Companies House Confirmation Statement (early)
       — gov.uk/file-your-confirmation-statement-with-companies-house
       — Reflects new 50/50 PSC structure + ECCTA IV completed
       — Update SIC codes if relevant (currently 46900; consider adding 46491 stationery, 46740 hardware, 46499 other)
       — 30 min · £50 online · OPERATOR

[ ] 16. Find a Tender + Central Digital Platform registration
       — Both Dieter and Zac create personal GOV.UK One Login first
         (gov.uk/log-in-or-register-gov-uk-one-login)
       — Designate Dieter as Sorellon's administrator
       — Register Sorellon Limited on find-tender.service.gov.uk
       — Populate ALL CDP fields (buyers filter on "fully populated")
       — Save CDP unique identifier for downstream registrations
       — 2 hours · £0 · OPERATOR

[ ] 17. Configure FTS saved searches with CPV codes
       — 33700000 Personal care products
       — 33741000 Hand care products
       — 33772000 Disposable paper products
       — 39800000 Cleaning and polishing products
       — 39830000 Cleaning products
       — 33140000 Medical consumables
       — 33141420 Surgical gloves
       — 18424000 Gloves (work)
       — 35113400 Protective and safety clothing
       — 30192700 Stationery (education)
       — Set email frequency to daily
       — 30 min · £0 · OPERATOR

[ ] 18. Devolved nation portal registrations
       — Sell2Wales: sell2wales.gov.wales/Register_V2/Supplier_Register.aspx
         (requires CDP unique ID from step 16)
       — PCS Scotland: publiccontractsscotland.gov.uk/Register/Register_Supplier.aspx
       — eTendersNI: etendersni.gov.uk (D-U-N-S 233967763 already held)
       — 30 min each · £0 each · OPERATOR

[ ] 19. NHS Supply Chain Atamis registration
       — health-family.force.com/s/Welcome
       — Required: Companies House #16575318, D-U-N-S 233967763, organisation info,
         sustainability disclosures (Evergreen assessment)
       — Complete NHS Evergreen Sustainable Supplier Assessment
         (pillars: Workforce, Net Zero, Modern Slavery, Information Governance)
       — 2 hours · £0 · OPERATOR

[ ] 20. Contracts Finder saved searches (redundancy channel)
       — gov.uk/contracts-finder
       — 30 min · £0 · OPERATOR
```

### Phase D — Optional certification gates (weeks 4-5; only if targeting Constructionline-required tenders)

```
[ ] 21. Living Wage Foundation accreditation
       — livingwage.org.uk/accreditation
       — Sole-director Sorellon trivially satisfies (only director paid)
       — Year-1 essentially-free social-value win
       — 1 hour · £100/yr · OPERATOR

[ ] 22. Constructionline Bronze (DEFER unless a target tender requires it)
       — constructionline.co.uk/products-services/memberships/sub-contractor-memberships/
       — Required by many LA cleaning/janitorial DPS tenders
       — Decision point: check FTS saved-search results week 2-4 for whether your
         pipeline is Constructionline-gated
       — 4 hours · £249/yr · OPERATOR (DEFER UNLESS NEEDED)

[ ] 23. SMAS Worksafe SSIP (DEFER unless a target tender requires it)
       — Same decision logic
       — 3 hours · £234/yr · OPERATOR (DEFER UNLESS NEEDED)

[ ] 24. JOSCAR SME Portal (free entry, optional)
       — hellios.com/joscar
       — Zero-cost defence visibility (MoD is out of year-1 scope, but free entry has no downside)
       — Skip unless specifically wanted
       — 30 min · £0 · OPERATOR (OPTIONAL)

[ ] 25. Disability Confident Level 1 (reconsider — free self-declaration)
       — Free, 30-min self-declaration
       — Adds scoring on some NHS + London-borough bids
       — Zero downside; recommend reconsidering even though Round 5 was no
       — 30 min · £0 · OPERATOR (RECOMMEND RECONSIDER)
```

### Phase E — Bid asset drafting (weeks 4-5, parallel)

```
[ ] 26. Draft Social Value Statement
       — 3-5 pages
       — Themes: economic inequality, climate, equal opportunity, wellbeing, COVID-19 recovery
       — Reference Cabinet Office PPN 06/20 Social Value Model
       — ~6 hours · £0 · AI-assisted + operator review

[ ] 27. Draft GDPR Article 28 DPA template
       — Use ICO model contractual clauses as base
       — List Sorellon's sub-processors (Google Workspace, Netlify, payment processor)
       — ~4 hours · £0 · AI-assisted

[ ] 28. Draft GDPR Article 30 RoPA (Record of Processing Activities)
       — Pre-bid asset; what personal data Sorellon processes (RFQ form data primarily)
       — ~2 hours · £0 · AI-assisted

[ ] 29. Draft Safeguarding Policy
       — Mandatory for any education / care / NHS bid
       — 2-4 pages; named safeguarding lead = Zac
       — ~5 hours · £0 · AI-assisted

[ ] 30. Draft Complaints Procedure (standalone)
       — 1-2 pages
       — Acknowledgement timeline 1 business day; resolution timeline 5 business days
       — ~3 hours · £0 · AI-assisted

[ ] 31. Draft Ethical Sourcing Statement (standalone)
       — 2-3 pages; covers supplier code of conduct, audit programme, country risk
       — Cross-references Modern Slavery Statement + Environmental Policy
       — ~5 hours · £0 · AI-assisted

[ ] 32. Draft Logistics Capability Statement
       — 2-4 pages
       — Honest about no fixed warehouse; multi-country sourcing; UK delivery partners
       — ~4 hours · £0 · AI-assisted

[ ] 33. Complete Carbon Reduction Plan baseline numbers
       — Replace TBC figures in existing CRP draft (already on sorellon.com)
       — Use SME Climate Hub free template
       — Sorellon's Scope 1 ≈ 0, Scope 2 ≈ 0 (residential office), Scope 3 = material categories
       — ~8 hours · £0 · OPERATOR (RECOMMEND RECONSIDER from Round 5 no-vote)

[ ] 34. Annual-review and re-date all 10 existing policy documents
       — Sorellon's policy suite was last reviewed March 2026; re-confirm if any update needed
       — Re-date all 10 policies + republish to refresh validity
       — ~3 hours · £0 · OPERATOR
```

### Phase F — Working capital + first-bid prep (weeks 5-6)

```
[ ] 35. Make capital injection decision
       — Director's loan (Zac/Dieter inject £20-£50k as loan, documented + minute) OR
       — SH01 share allotment (increase share capital from £1 to e.g. £20k)
       — Required before bids ≥ £50k contract value
       — Brokers + bid evaluators may ask about working capital by week 5
       — 2 hours decision + 1 hour filing · £15-50,000 (loan not expense) · OPERATOR

[ ] 36. Claim Sorellon profile on credit-reference agencies
       — Experian Business: experian.co.uk/business
       — Equifax Business: equifax.co.uk/business
       — Creditsafe: creditsafe.com/gb (most procurement-friendly)
       — Dun & Bradstreet: dnb.com/en-gb (DUNS 233967763 already exists; verify profile)
       — 1 hour total · £0 · OPERATOR

[ ] 37. Identify first bid opportunity
       — Review FTS / Atamis / Procontract / Sell2Wales / PCS / eTendersNI saved searches
       — Prefer for FIRST bid:
         · Sub-£25k LA DPS lot or small NHS Trust direct purchase
         · Non-biocidal product category (no BPR triggered)
         · Manufacturer-branded (no own-brand liability)
         · Deadline 14+ days in future
         · No imminent commitment risk
       — 2-4 hours research · £0 · OPERATOR

[ ] 38. Prepare first bid response
       — Use bid asset library built in Phase E
       — Tailor scope, pricing, lead times to the specific opportunity
       — ~10-20 hours depending on complexity · £0 · OPERATOR + AI

[ ] 39. Submit first bid 🎯
       — Via the relevant portal (Atamis / FTS / Procontract / etc.)
       — Save submission confirmation
       — Calendar reminder for opportunity status check
       — 1-2 hours · £0 · OPERATOR
```

### Phase G — Pipeline maturity (weeks 7+)

```
[ ] 40. Receive VAT number from HMRC (typically week 8-10)
       — Update sorellon.com footer
       — Add VAT number to bid asset library
       — Re-issue any quotes given pre-VAT-number

[ ] 41. Build first 3 contract references via small-contract strategy
       — Target sub-£10k LA DPS lots that ask only for testimonial-style references
       — Successful delivery → reference #1, repeat for #2 and #3

[ ] 42. Review bid-pipeline progress weekly
       — FTS / Atamis alerts → 3+ bids per week scanned
       — 1+ bid submitted per fortnight

[ ] 43. Per-bid: assess Constructionline Bronze + SMAS gating
       — If 3 of 10 bids require Constructionline → buy (£249/yr) on the next applicable bid
       — If only 1 of 10 → continue deferring
```

### Phase H — Triggered-only actions (only when contracts demand them)

These items belong here, NOT in year-1 blanket prep:

```
[ ] 44. POST-WIN: Engage accountant for first management accounts (~3 mo before year-end 2027-04-10)
       — £500-£1,500 one-off
       — Triggered: approaching first accounts deadline

[ ] 45. POST-WIN: GB BPR per-SKU audit
       — Only when winning a biocidal-product contract AND about to deliver
       — In-house + AI for most products; paid legal review (~£500-£1,500) for high-risk products
       — Triggered: contract win with biocidal product in scope

[ ] 46. POST-WIN: MHRA UKRP verification per Chinese supplier
       — Only when delivering a Chinese manufacturer's medical device under a won contract
       — In-house verification: request UKRP appointment + MHRA registration evidence
       — Triggered: contract win with specific Chinese supplier's medical device

[ ] 47. POST-WIN: UK REACH importer registration
       — Only if any substance crosses 1 t/yr import threshold
       — Unlikely to trigger year-1 given Sorellon's scale
       — Action: maintain substance-by-substance tonnage tracker from day 1

[ ] 48. POST-WIN: CDS customs declarations per import shipment
       — Only triggered when actually importing
       — DIY option OR negotiate DDP Incoterms with supplier (~£0)
       — Per-shipment cost via broker: £50-£100

[ ] 49. POST-WIN: Insurance limit upgrade £5M → £10M PL/PrL
       — Only when a specific tender requires £10M cover
       — Mid-bid endorsement: ~£200-£400 per bid OR £400 annual upgrade

[ ] 50. POST-WIN: Bid-specific declarations
       — Each bid response uses the policy + statement pack drafted in Phase E
       — No additional drafting needed unless a buyer asks for a bespoke document
```

### Total Phase A-G operator time + cost

- **Phase A** (this week): ~3-4 hours operator time; £62 cash (ICO + share filing)
- **Phase B-E** (weeks 2-5): ~60-80 hours operator time; £1,400-£2,400 cash
- **Phase F** (weeks 5-6): ~15-25 hours operator time; £0 cash (capital injection is a loan not expense)
- **Phase G** (weeks 7+): ongoing
- **Total to first bid**: ~80-110 hours over 5-6 weeks; £1,500-£2,500 cash

Plus optional capital injection (£20-£50k as director's loan, recoverable).

---


## 2. Locked-in decisions (from the 5-round clarification + refinement)

| Dimension | Decision |
|---|---|
| **Buyers** | LA + LA-affiliated (district / borough / county councils, schools, MATs, housing); NHS (Trusts direct purchases + NHS Supply Chain frameworks); central government via CCS/GCA. **MoD explicitly out** for year-1. |
| **Contract size** | £10k-£100k dominant tier |
| **Geography** | All 4 UK nations (England + Wales + Scotland + NI). All devolved portals are free; ~1.5 hours one-off registration time. |
| **Bid posture** | Mix per opportunity — prime / sub-contractor / consortium evaluated case-by-case |
| **Bank account** | ✅ Already set up |
| **Companies House IV** | ❌ NEITHER PSC verified yet. Dieter ~5 months overdue (deadline 14 Dec 2025). Zac due 23 Jul 2026. |
| **Advisors** | ❌ None engaged. Will engage accountant only at year-end (one-off, not retainer); broker outreach via this plan; solicitor on-demand. |
| **Trading state** | Pre-trading — no live RFQ activity yet. Clean window for VAT registration. |
| **Product scope** | All 4 categories (healthcare + cleaning/janitorial + education/office + construction-adjacent) — comprehensive offer matching sorellon.md |
| **Sourcing** | **Multi-country — UK, EU, China, global, whichever cheapest per product**. NOT China-only. See §5 for regulatory implications. |
| **Biocidal claims** | YES — antimicrobial wipes / sanitisers / disinfectants are core. GB BPR compliance handled PER-CONTRACT POST-WIN, not blanket year-1. |
| **Re-branding** | NO — manufacturer-branded only. Sorellon stays as distributor, not legal manufacturer. |
| **Structure** | Zac transfers 1% to Dieter → 50/50 PSC. Dieter remains non-executive PSC (no employment contract) to preserve EL exemption. Contractors only otherwise (accountant, etc.). |
| **Restructure timing** | This week / month 1 alongside IV |
| **First-bid target** | 30-60 days from today |
| **Regulatory work** | In-house + AI-supplemented |
| **Capital** | Both director's loan and SH01 share allotment on table; decide before week 5 (before first bid submission) |
| **Insurance posture** | Bid-led not budget-led; basic £5M tier as default, upgrade per-bid when required |
| **Social commits** | Living Wage YES; Disability Confident L1 / no-Xinjiang / CRP baseline — reconsider per §16 below |
| **Time per week** | 10-20 hours operator time; mid-band assumed for planning |

---

## 3. The single most important reframe — PRE-BID vs AT-BID-TIME vs POST-WIN

The original cost figures conflated three distinct spend tiers. Separating them:

### 3.1 PRE-BID — items you must have before submitting any bid

These are pre-qualification gates. Every bid will check. Total: **~£1,500-£2,000 year-1 cash**.

| Item | Cost | Why pre-bid |
|---|---|---|
| Companies House active status + both PSCs IV-verified | £0 | Procurement portals do live Companies House lookups |
| Confirmation Statement filed | £50/yr | Companies House good standing required |
| HMRC active-status notification | £0 | Required within 3 months of trading |
| ICO Data Protection Fee | £47/yr | Many SQs ask "ICO registered? Number?" |
| Cyber Essentials (basic) | £396 | PPN 014 mandate; pre-qual gate for central gov + NHS Trust direct + many LA |
| Basic combined insurance £5M EL/PL/PrL | £800-£1,200/yr | Mandatory upload field on every framework / portal |
| Living Wage Foundation | £100/yr | Social-value scoring at bid stage |
| Documented QMS procedure | £0 (16h work) | SQ Module 3 |
| Modern Slavery Statement (voluntary) | £0 (6h work) | Often asked at bid stage |
| HMRC VAT registration | £0 + ~£170/yr accounting software | VAT number routinely requested on bid forms |
| Find a Tender / CDP / devolved portals | £0 | Discovery + unique supplier ID |
| NHS Supply Chain Atamis + Evergreen | £0 | Required for NHS framework participation |
| Policy suite | £0 (already held) | SQ pre-bid documents |
| Bid asset library | £0 (in-house) | SQ-stage documents |

### 3.2 AT-BID-TIME — per-opportunity decisions, allocate to that bid

Not paid for in advance. Add only when a specific bid requires them.

| Item | Cost | When needed |
|---|---|---|
| Insurance limit upgrade £5M → £10M | +£400/yr OR per-bid endorsement | When a specific bid requires £10M (NHS Supply Chain frameworks, some CCS) |
| Constructionline Bronze | £249/yr | Only if a target tender requires it (many LA cleaning tenders do; many don't) |
| SMAS Worksafe SSIP | £234/yr | Same as Constructionline — buyer-discretionary |
| Sub-contractor / consortium agreement | £500-£1,500 per opportunity | Only when going sub or consortium route |
| Disability Confident L1 self-declaration | £0 (FREE — reconsider declining) | When a target tender scores it positively |

### 3.3 POST-WIN — triggered only if you win a contract requiring delivery

**The bulk of "scary regulatory work" lives here. None of it is required to BID.**

| Item | Cost | When needed |
|---|---|---|
| GB BPR per-product audit | £200-£500/SKU in-house+AI, or £500-£1,500 paid review | Only when selling biocidal product under a won contract |
| MHRA UKRP verification per supplier | £0 in-house | Only when selling medical device under a won contract |
| UK REACH importer registration | £0 (or £2-5k if substance ≥ 1 t/yr) | Only if substance crosses threshold |
| HMRC GB EORI | £0 | Only before first import shipment |
| CDS customs declarations | £0 DIY / £50-£100/shipment via broker / £0 if supplier DDP | Per shipment |
| GB CLP labelling audit | £0 in-house | Per product before placing on UK market |
| UKCA/CE distributor verification | £0 in-house | Per SKU before placing on market |
| Plastic Packaging Tax | £0 + per-quarter returns | Only at 10-tonne threshold (year 2+) |
| EPR Packaging fees | Tiered | Only at £1M turnover (year 2+) |
| Accountant for first management accounts | £500-£1,500 one-off | ~3 months before year-end (Apr 2027) |
| Customs broker | £50-£250/shipment OR £0 with DDP | Per shipment |

### 3.4 Why this distinction matters

A bid response describes what Sorellon WILL supply if awarded. It does NOT require pre-completed BPR audits for every aspirational SKU. The audit happens before first delivery under the contract — that's post-win.

In a typical 60-day pre-bid sprint, Sorellon spends ~£1,500-£2,000 to acquire the PRE-BID stack and reach first bid. AT-BID-TIME costs (Constructionline, SMAS, insurance upgrade) get added only when a specific tender requires them — and the cost is justified by the specific opportunity. POST-WIN costs are paid out of contract margin from winning bids.

---

## 4. Year-1 budget — three tiers

### 4.1 ROCK BOTTOM (~£1,500-£2,000 year-1 cash)

Strictest interpretation: only items pre-bid mandatory.

| Item | Cost |
|---|---|
| Companies House IV (One Login) | £0 |
| SH01 / J30 share transfer | £0-£15 |
| Confirmation Statement | £50 |
| ICO Data Protection Fee (DD) | £47 |
| HMRC VAT registration + accounting software | £0 + ~£170/yr |
| Cyber Essentials (basic) | £396 |
| Insurance — BASIC £5M EL/PL/PrL combined | £800-£1,200 |
| Living Wage Foundation | £100 |
| All other items | £0 (in-house) |
| **ROCK BOTTOM TOTAL** | **~£1,563-£1,963/yr** |

**Trade-offs accepted at Rock Bottom:**
- LA tenders requiring Constructionline Bronze → miss until added (~£249 when needed)
- LA tenders requiring SSIP → miss until added (~£234 when needed)
- NHS Supply Chain framework lots requiring £10M PL/PrL → miss until insurance upgraded
- All POST-WIN regulatory items deferred to per-contract triggers

### 4.2 SAFER (~£2,000-£2,500 year-1 cash) — RECOMMENDED

Adds the two main LA gate certs.

| Item | Cost |
|---|---|
| Rock bottom items | £1,563-£1,963 |
| + Constructionline Bronze | £249/yr |
| + SMAS Worksafe SSIP | £234/yr |
| **SAFER TOTAL** | **~£2,046-£2,446/yr** |

This is the recommended starting tier for your buyer mix (LA + NHS + central gov at £10-£100k).

### 4.3 CAUTIOUS (~£3,500-£4,500 year-1 cash) — NHS-Supply-Chain-ready

Only worth it if specifically targeting NHS Supply Chain framework re-tenders.

| Item | Cost |
|---|---|
| Safer items | £2,046-£2,446 |
| + Insurance upgrade £5M → £10M | +£400-£800 |
| + Product Recall extension £100k | +£100-£400 |
| + Paid BPR legal review (one-off, before first biocidal sale) | +£500-£1,500 |
| + Customs broker for first 6 imports | +£500-£1,200 (one-off) |
| **CAUTIOUS TOTAL** | **~£3,546-£6,346/yr** |

### 4.4 Recommendation

**Start at SAFER (~£2,046-£2,446).** Upgrade to CAUTIOUS items only when a specific NHS Supply Chain framework bid (Hand Hygiene Jun 2026 / PPE Sep 2026) is in your sights — month 3-5, not pre-bid.

Capital injection (£20-£50k director's loan) is separate from this budget — it's a loan recoverable, not an expense.

---

## 5. Multi-country sourcing — revised regulatory analysis

**This is a correction to an earlier framing.** Sorellon's `sorellon.md` line 116 states: "Access to factories and suppliers in the UK, EU, China, and trusted global locations." Sourcing is **multi-country**, not China-only. The regulatory burden depends on the supplier-country + Incoterm combination for each product, not on Sorellon's overall company profile.

### 5.1 The supplier-country / Incoterm matrix

| Supplier country | Incoterm | Importer of record | Sorellon's regulatory burden |
|---|---|---|---|
| **UK manufacturer** | Any | N/A (no import) | **Lightest** — distributor only. SDS pass-through, label retention, UKCA/CE verification |
| **EU manufacturer WITH UK Only Representative (OR)** | Any | The OR | **Light** — distributor only. Same as UK supplier above; the OR carries the importer-side REACH/MHRA burden |
| **EU manufacturer WITHOUT UK OR — supplier DDP** | DDP (Delivered Duty Paid) | Supplier (via their UK customs agent) | **Light** — distributor only. Customs handled by supplier; Sorellon is consignee but not importer of record |
| **EU manufacturer WITHOUT UK OR — supplier NOT DDP** | FOB / EXW / others | Sorellon | **Heavy** — full importer burden under UK REACH, MHRA distributor obligations expand, CDS declarations, EORI required |
| **Non-EU (China, US, etc.) — supplier DDP** | DDP | Supplier (via their UK customs agent) | **Light** — distributor only, same as above. Supplier's DDP price typically includes UK import VAT + duty, so Sorellon's landed cost is fully known |
| **Non-EU — supplier NOT DDP** | FOB / EXW / CIF / others | Sorellon | **Heaviest** — full importer burden + customs declaration responsibility + potential anti-dumping duty exposure |

### 5.2 Strategic implication

**Sorellon can engineer its regulatory burden DOWN by preferring suppliers in this order:**

1. **UK manufacturers** for any product where UK supply is commercially viable (cleaning chemicals — there are many UK manufacturers; janitorial paper — UK production exists; stationery — UK distributors abundant)
2. **EU manufacturers with UK OR appointed** for regulated products where UK doesn't have competitive supply (some PPE categories, certain medical devices)
3. **DDP suppliers from anywhere** for products where direct overseas supply offers margin advantage AND the supplier accepts DDP terms
4. **FOB / EXW direct import** ONLY when the cheaper landed cost outweighs the importer regulatory burden — typically high-volume, low-regulatory-risk goods (basic stationery, simple PPE Cat I)

This is a **per-product** strategy. Sorellon doesn't choose one regime for the whole company; each product line can have a different supplier-country / Incoterm combination.

### 5.3 What this changes vs the earlier analysis

The earlier "China-only direct import" framing implied:
- 🚨 Full UK REACH importer obligations for every cleaning chemical Sorellon sells
- 🚨 MHRA distributor + UKRP verification burden for every Chinese-sourced medical device
- 🚨 CDS declarations per shipment
- 🚨 EORI registration mandatory before any import
- 🚨 Anti-dumping duty exposure on every Chinese consignment

**The revised analysis says these obligations attach PER PRODUCT, not blanket:**
- A product sourced from a UK supplier triggers NONE of the above
- A product sourced from EU-with-OR triggers NONE of the above
- A product sourced DDP from China triggers the customs+VAT handling at supplier cost, not Sorellon's
- Only products sourced FOB / EXW from outside-UK trigger the full importer burden, and only for THOSE products

### 5.4 Practical sourcing playbook for Sorellon

**For each product Sorellon plans to bid with**, ask:
1. Is there a UK manufacturer / UK distributor offering this product? (If yes, use them — lightest regulatory path; verify their UKCA/CE compliance)
2. Is there an EU manufacturer with an appointed UK Only Representative? (If yes, second-best; same light regulatory burden)
3. Is the cheapest source non-EU, but supplier offers DDP? (If yes, accept DDP; verify customs handling is genuine and certificate-of-conformity is provided)
4. Is the cheapest source non-EU, FOB only? (Then Sorellon takes the importer hit; only do this when commercial advantage > regulatory cost)

**For regulated products specifically** (medical devices, biocidal products, Cat III PPE), strongly prefer routes 1 and 2 — the regulatory verification burden on importer-of-record products in these categories is substantial.

### 5.5 Bid implication

When responding to a bid, Sorellon's "supply chain narrative" should describe its multi-source approach: "Sorellon sources from UK manufacturers, EU manufacturers with appointed UK Responsible Persons, and non-EU manufacturers using DDP Incoterms — our supply chain due-diligence model includes verification of UKCA/CE marking, REACH compliance, MHRA registration where applicable, and supplier code-of-conduct adherence." This is more credible than "we source from one country" and reassures evaluators about supply security.

---

## 6. Customs declarations + DDP supplier terms — what the codebase actually says

Subagent inspection of the codebase finds the operator's claim about "customs and declarations being included in shipping quotes" is **partly true, partly oversimplifies**. Here's what's true and what isn't:

### 6.1. What the Freightos skill actually captures

The `skills/landed-cost/freightos-freight-estimate/` skill exposes only an `include_customs_brokerage: bool` toggle (`SKILL.md:47`). It does **NOT** capture Incoterm (DDP / FOB / EXW / CIF). The `declaredCustoms.entry` flag controls whether Freightos quotes include the customs **brokerage handling** fee — NOT the duty + VAT amounts themselves.

Evidence in the screenshots (`docs/pipeline/l5-expanded-commercial-intelligence/operator-built-skills/screenshots/freightos/`):
- `freightos-step-08.png` — Freightos "Recommended Services" UI defaults Customs brokerage toggle to ON
- `freightos-step-09.png` — 28-quote results, port-to-port, with NO duty/VAT line items
- `TEST_REPORT.md:38, 71` — confirms heterogeneity: "some forwarders bundle customs into their listed price... others don't"
- One canary case: base €3,819.29 vs with-services €3,327.97 — i.e. with-customs was LOWER, only makes sense if it's clearance fees not duty disbursement

### 6.2. Where Incoterm IS modelled (downstream, not in the freight skill)

The freight-engine (`packages/intelligence/src/logistics/freight-engine.ts:52`) defines `FreightIncoterm` as `"EXW" | "FOB" | "CIF" | "DDP" | "unknown"`. When the Incoterm is `unknown`, **the engine runs only `EXW/FOB/CIF` scenarios (line 312) and selects the highest-cost branch — explicitly treating Sorellon as the importer of record**.

The Incoterm field is on `supplier_quotes.incoterm` (`crates/daemon/src/main.rs:4870`), populated from the marketplace listing (Alibaba, Amazon, eBay, etc.), NOT from the Freightos skill.

### 6.3. What DDP actually shifts vs doesn't

| Obligation | DDP shifts? | Why |
|---|---|---|
| UK customs duty + VAT at import (CDS declaration) | ✅ YES | DDP = "Delivered Duty Paid" — supplier handles all import costs |
| Importer of record for CDS purposes | ✅ YES | Supplier's customs agent files the declaration |
| EORI requirement on Sorellon | ✅ YES (for that shipment) | Supplier uses their own EORI/customs representation |
| UK REACH "placer on market" status | ❌ NO | This attaches to the commercial party placing the substance on the UK market — Sorellon, regardless of who handled customs |
| MHRA distributor obligations | ❌ NO | Distributor obligations attach to who sells the device in the UK — Sorellon |
| Conformity verification (UKCA/CE) | ❌ NO | Sorellon as the on-market distributor must verify per-product, regardless of Incoterm |
| Vigilance / adverse-event reporting | ❌ NO | Distributor duty under UK MDR 2002 |
| Anti-dumping duty exposure | ✅ YES | Supplier absorbs in DDP pricing |

**Bottom line**: DDP genuinely lowers Sorellon's customs/duty workload to ~zero per shipment, AND DDP-bundled customs is real evidence Sorellon often isn't the legal importer of record for customs purposes. **BUT** the regulatory obligations under REACH (substance placement), MHRA (medical device distribution), UKCA (marking verification), BPR (biocidal placement) attach to whoever sells the product onward in the UK market — which is Sorellon — regardless of who handled customs at port.

### 6.4. Practical implication for sourcing strategy

This actually reinforces §5's recommendation:

- **For low-regulatory products** (basic stationery, simple PPE Cat I, hand tools, paper goods): DDP from China is fine. Sorellon's regulatory burden is essentially the same as buying from a UK distributor. Cheapest-source-with-DDP wins.
- **For regulated products** (medical devices, biocidal products, Cat III PPE): DDP doesn't help with the regulatory placer-on-market obligations. Still prefer UK supplier or EU-with-OR routes. The customs saving from DDP is small (~£50-£250/shipment) compared to the per-SKU regulatory verification work (~£200-£1,500/SKU including paid legal review for BPR).

### 6.5. Codebase gap surfaced by the analysis

The Freightos skill currently doesn't forward Incoterm to the freight-engine. If Sorellon's procurement pipeline wants to accurately track which supplier shipments are DDP (= light burden) vs FOB (= heavy burden), this should become a first-class field. **Suggested follow-up bead under the L5 implementation epic**: extend `FreightosSkillInvocation` to accept supplier-side Incoterm input from `supplier_quotes.incoterm`, propagate to freight-engine, and switch the "unknown Incoterm = highest-cost branch" default to "unknown = require manual operator review."

This isn't blocking your first bid; it's an internal-tooling improvement worth raising with the development team.

---


## 7. Pipeline alignment — major scope mismatch surfaced

Subagent inspection of the platform's own qualification logic (`docs/strategy/layers/layer-0-gold-set-evaluation.md:96-122` and the foundational research at `docs/research/2026-03-08-public-procurement-intelligence-platform-research.md`) finds a **significant mismatch between what the pipeline qualifies as bidable and what Sorellon's stated commercial targets are**.

### 7.1. What the pipeline actually qualifies as bidable

The pipeline applies seven orthogonal filters before surfacing an opportunity:

1. **Category whitelist (HARD)** at `layer-0-gold-set-evaluation.md:96-122`. Only 6 commodity categories pass:
   - Office supplies (pens, paper, stationery)
   - Cleaning products **buckets / brushes / mops only — NOT wipes or chemically-treated**
   - Basic PPE (hi-vis, basic gloves — Cat I/II only)
   - Hand tools
   - Basic furniture (wood-only — metal excluded due to trade-remedy risk)
   - Packaging (cartons safe — foil / glass / tin excluded)
2. **Explicit no-list** at `:113-120`: medical devices, pharmaceuticals, IT hardware, vehicles, construction materials, food, **chemicals**, bespoke
3. **Access route**: `direct_award` / `closed_framework_award` / `not_bidable` → reject; `framework_call_off` surfaced as actionable
4. **Expiry**: 30-day cutoff (DPS exempt)
5. **Goods qualification**: multi-dimensional (legalCategory, goodsDensityScore, goodsSeparability)
6. **Quoteability score**: < 0.45 → manual/RFQ only
7. **Margin gates**: net_margin_low ≥ 20% + no critical blockers + netQualifiedValueCoverageBps ≥ 5000

### 7.2. 🚨 The major mismatch

**The pipeline's L0 whitelist EXCLUDES Sorellon's stated #1 commercial target.**

- Sorellon's Round 3 answer: "biocidal products core to offer"
- Sorellon's executive-summary commercial windows: NHS Supply Chain Hand Hygiene re-tender (Jun 2026) + NHS Supply Chain PPE re-tender (Sep 2026)
- The pipeline whitelist: EXCLUDES chemically-treated wipes, EXCLUDES chemicals, EXCLUDES medical devices

**Translation**: opportunities for antimicrobial wipes / hand sanitisers / biocidal cleaners / medical-grade PPE will NOT be surfaced as `final_qualified` by the platform regardless of MVS tier. The platform's qualification gate filters them out at the goods-classification stage.

### 7.3. What the pipeline DOES surface that Sorellon can bid on

Per-category MVS tier mapping:

| Category | Pipeline surfaces it? | Bidable at ROCK BOTTOM? | Bidable at SAFER? | Bidable at CAUTIOUS? |
|---|---|---|---|---|
| **Office supplies / stationery** | ✅ Yes | ~70-80% (LA + academy + small NHS Trust) | ~80% | ~85% |
| **Non-chemical cleaning (buckets/brushes/mops)** | ✅ Yes | ~50% | ~60% | ~75% (NHS Supply Chain framework lots need CAUTIOUS) |
| **Basic PPE Cat I/II** | ✅ Yes | ~40% | ~50% | ~60% (NHS Supply Chain PPE framework needs CAUTIOUS+ISO 13485 — out of MVS) |
| **Hand tools** | ✅ Yes | ~70% | ~80% | ~80% |
| **Basic wooden furniture** | ✅ Yes | ~70% | ~80% | ~80% |
| **Packaging (cartons)** | ✅ Yes | ~70% | ~80% | ~80% |
| **Biocidal wipes / hand sanitisers** | ❌ Pipeline EXCLUDES — not in whitelist | N/A | N/A | N/A |
| **Hand-Hygiene chemicals / disinfectants** | ❌ Pipeline EXCLUDES (chemicals on no-list) | N/A | N/A | N/A |
| **Class II+ medical devices (syringes, catheters)** | ❌ Pipeline EXCLUDES (medical devices on no-list) | N/A | N/A | N/A |
| **Class III PPE (chemical-resistant gloves, FFP masks)** | ⚠️ Borderline — basic PPE only | ~30% | ~40% | ~50% |
| **Painting/maintenance kits** | ⚠️ Construction-adjacent; mixed | ~30% | ~40% | ~50% |
| **Education stationery** | ✅ Yes (office supplies) | ~70% | ~80% | ~85% |

### 7.4. Net pipeline-bidable percentage by tier

Aggregating across categories, weighted by pipeline volume:

| Tier | Year-1 cash | Bidable % of pipeline-qualified opportunities | Pipeline waste |
|---|---|---|---|
| ROCK BOTTOM | £1,500-£2,000 | ~30-40% | ~60-70% |
| **SAFER** | **£2,000-£2,500** | **~55-70%** | **~30-45%** |
| CAUTIOUS | £3,500-£4,500 | ~60-75% | ~25-40% |

**SAFER is the inflection point.** CAUTIOUS only buys 5-15% more bidability, most of which is NHS Supply Chain framework participation that ALSO needs out-of-MVS items (ISO 9001-UKAS ~£6k, ISO 13485 if medical, 3 contract references, capital adequacy >£20k).

### 7.5. What this means for Sorellon's commercial strategy

Two paths the operator should consider:

**Path A — Pipeline-aligned (recommended for year-1)**: Accept that the platform's pipeline will surface the "boring SKU lanes" only — office supplies, non-chemical cleaning equipment, basic PPE Cat I/II, hand tools, basic furniture, packaging — sold to LA + academy + small NHS Trust direct buyers under £50k. This matches Sorellon's stated buyer mix + contract size, and is exactly what the SAFER MVS tier was built for. Pursue NHS Hand Hygiene and NHS PPE framework opportunities MANUALLY outside the pipeline, not via the platform's automatic feed.

**Path B — Expand pipeline scope**: Drive L5 category expansion (per `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md:40-41`) to bring biocidal / medical-device categories into the gold-set whitelist. This requires per-category gold-set validation, additional landed-cost engine work for biocidal products' REACH/BPR cost-stack, and probably a quarter of platform-development effort. **Decision: this is platform-development work for the implementation team, NOT pre-bid work for the operator.** Worth flagging as a roadmap item; not a year-1 blocker.

### 7.6. Two further codebase gaps surfaced

1. **No `BidderEligibilityGate` at L6** alongside the existing five L5→L6 readiness gates. The pipeline currently checks classification freshness + company-profile expiry but does NOT validate framework membership or capital adequacy. **Result**: framework call-off opportunities surface as "bidable" but Sorellon may be ineligible to call off (not on framework, capital below threshold). Suggested as a fix bead under the L6 implementation epic.

2. **Pipeline doesn't segment by buyer type or geography.** Per `layer-strategy.md:34-37`, the pipeline ingests across all UK nations and all buyer types but doesn't filter by Sorellon's chosen MoD-out + size band. Suggested fix: per-bidder-profile filter at L5 qualification stage. Smaller change; lower priority.

### 7.7. Bottom line — what to do with this finding

For the FIRST BID specifically (week 5-6 of the plan): **accept the pipeline scope mismatch and bid into office supplies + non-chemical cleaning equipment + basic PPE Cat I/II opportunities**. These are real, high-volume, easy entry points. Your SAFER MVS tier covers ~55-70% of what the pipeline will surface — that's plenty for early-pipeline learning.

For NHS Hand Hygiene + NHS PPE framework opportunities (Jun + Sep 2026): **prep these manually, outside the pipeline's automatic feed**. Set up calendar reminders to monitor Atamis directly. Build the BPR-compliance evidence pack for the biocidal products in your catalogue independently of the platform's qualification engine.

---


## 8. EL insurance + 50/50 restructure interaction

The HSE statutory exemption for sole-director Ltd's is narrow:

> A limited company is NOT required to hold Employers' Liability insurance if **the only employee is a person who owns 50% or more of the issued share capital** of the company.
> — Employers' Liability (Compulsory Insurance) Act 1969 + HSE guidance HSE40

**Three scenarios for Sorellon post-restructure:**

| Scenario | EL legally required? | EL cost in practice |
|---|---|---|
| Dieter non-executive PSC (no employment contract, no salary, no active work); Zac sole working director owning 50% | **NO** (exemption applies) | £0 if no target tender demands EL certificate. ~£20-£60 if any does — and in practice, every framework portal does. |
| Both Dieter AND Zac are working employees regardless of share split | YES (multi-employee rule) | £20-£60 bundled inside combined policy |
| Zac sole worker; Dieter non-executive PSC but you bid on tenders requiring EL upload | NO statutorily, but procurement portals demand it | £20-£60 bundled |

**Net practical answer**: buy EL inside the combined policy regardless. The £20-£60 marginal cost is part of the £800-£1,200 basic-insurance figure already; the EL purchase is a procurement decision, not a statutory one.

The 50/50 restructure DOES achieve a separate good outcome — cleaner PSC structure, both directors verified at Companies House under the new shareholding, simpler bid responses about ownership — but it does NOT meaningfully reduce insurance cost.

---

## 9. 60-day weekly plan

For more compact week-by-week tracking. Maps to the step-by-step actions in §1.

### Week 1 (this week) — emergency triage

Steps 1-7 from §1.

- 🚨 Companies House IV for Dieter (~15 min, £0)
- 🚨 Same for Zac
- Execute 1% share transfer (1 hr, £0-£15)
- Decide Dieter's post-restructure employment status (non-executive recommended)
- Set up DIY accounting software (Xero / FreeAgent / QuickBooks; ~£14-16/mo, NOT a full accountant retainer)
- Pay ICO Data Protection Fee (£47)
- HMRC active-status notification (if applicable)

**Week 1 cash out: ~£62 + £15-16/mo software.**

### Week 2 — Cyber Essentials + insurance outreach

Steps 8-9 + start of 11-12.

- Submit Cyber Essentials application (£396)
- Insurance broker outreach calls (Premierline, Towergate, Simply Business, Hiscox)
- Begin Documented QMS draft (AI-assisted)
- Begin Modern Slavery Statement draft

### Week 3 — Insurance bind + VAT + Confirmation Statement

Steps 10, 13, 14, 15.

- Bind insurance policy (£800-£1,500 first premium)
- HMRC VAT voluntary registration (online, free; 30-60 day processing)
- File Confirmation Statement (£50, reflects new 50/50 + IV completed)
- Cyber Essentials certificate received (if IASME marking fast)

### Week 4 — Procurement registrations + remaining policies

Steps 16-20, 26-30.

- Find a Tender + CDP registration
- Sell2Wales + PCS Scotland + eTendersNI
- NHS Supply Chain Atamis + Evergreen
- Living Wage Foundation accreditation
- Publish Modern Slavery Statement
- Begin Social Value Statement, DPA template, Safeguarding Policy

### Week 5 — Bid assets + capital decision + first opportunity

Steps 31-37.

- Complete bid asset library (Ethical Sourcing, Logistics Capability, RoPA)
- Complete CRP baseline (~8 hr)
- Capital injection decision (director's loan vs SH01 vs defer)
- Claim profiles on Experian, Equifax, Creditsafe, D&B
- Identify first bid opportunity from saved searches

### Week 6 — First bid submission

Steps 38-39.

- Final preparation of first bid response
- 🎯 SUBMIT FIRST BID

### Weeks 7-8 — Pipeline + VAT activation

Step 40-43.

- Receive VAT number from HMRC
- Continue weekly pipeline scan
- Submit second bid

---


---

# Part II — Detailed reference sections

The following sections are the detailed per-domain research that supports Part I above. Cross-referenced by section number in §1-§9. Read these when you need the full evidence behind a specific item; skip if you're executing the action plan and just need the action list from §1.

---

## 10. Tax, Statutory Compliance & Data Protection Foundations

> **Subject:** Sorellon Limited (Companies House #16575318), incorporated 2025-07-10, ~10 months old as of 2026-05-25. Pre-revenue B2B wholesale supplier (SIC 46900). Director + 50%+ PSC: Zac Farrell. Co-PSC (25-50%): Dieter Xia Shuang Danicek (the operator). Registered office: Avantgarde Tower, Apartment 144, London E1 6GS. Domain: sorellon.com (double-L canonical spelling). Tech stack: Google Workspace + Netlify-hosted brochure site with RFQ form only (no commerce). Zero employees beyond the director. First accounts due 2027-04-10.
>
> **Scope:** Statutory tax, company law (post-ECCTA), modern slavery transparency and data-protection regulatory foundation required to legitimately bid on UK public-sector procurement under the Procurement Act 2023 regime.
>
> **Document conventions:**
> - `[Source: <URL>]` — primary or supporting source for the preceding claim.
> - `[VERIFY: <thing>]` — material uncertainty the operator should re-check before acting.
> - All prices are 2026 GBP unless flagged otherwise.

---

### 10.1. VAT Registration

**Field — What it is:** Value Added Tax (VAT) is the UK consumption tax administered under the Value Added Tax Act 1994. Registration creates a legal obligation to charge VAT (standard rate 20%, reduced 5%, zero 0%, exempt) on taxable supplies, to file periodic returns (normally quarterly) through Making Tax Digital (MTD) compatible software, and to issue VAT invoices. Registration is mandatory when *taxable turnover* (rolling 12-month basis) exceeds the threshold, or when there are reasonable grounds to believe it will exceed the threshold in the next 30 days alone [Source: https://www.gov.uk/register-for-vat]. The current threshold is **£90,000** and has been static since 1 April 2024; HMRC confirmed it remains at £90,000 for the 2025/26 and 2026/27 tax years, with the deregistration threshold at £88,000 [Source: https://www.thp.co.uk/uk-vat-threshold-2026-a-complete-guide-for-growing-businesses/] [Source: https://mbridgetechnologies.cloud/blog/vat-threshold-increase-april-2026].

**Field — Mandatory?** *Conditional.* Not mandatory for Sorellon today (pre-revenue, no taxable turnover). It becomes mandatory the moment any of: (a) rolling 12-month taxable turnover crosses £90,000, (b) Sorellon reasonably expects taxable turnover to exceed £90,000 in the next 30 days alone, or (c) Sorellon imports goods from outside the UK in scenarios that trigger compulsory registration. Public buyers do **not** by themselves make VAT registration mandatory — but many tender forms include a VAT-number field, and central-government frameworks frequently filter or de-prioritise non-VAT-registered suppliers in B2B-wholesale categories where VAT-registered competitors recover input VAT and quote net.

**Field — Eligibility for Sorellon:** Fully eligible to register voluntarily today. HMRC permits voluntary registration at any turnover level provided the business is making (or intends to make) taxable supplies in the course of business [Source: https://www.starlingbank.com/resources/business-guides/benefits-of-being-vat-registered/]. Pre-revenue status does *not* block voluntary registration — HMRC needs evidence of *intention* to trade (registered office, banking, supplier contracts, marketing). Sorellon's incorporation, active website, SIC 46900 and RFQ pipeline are sufficient evidence of intent.

**Field — Cost:** £0 application fee (HMRC charges nothing for VAT registration). Indirect costs: MTD-compatible accounting software (Xero ~£16-£59/month, FreeAgent ~£19/month, QuickBooks ~£14-£42/month, or free MTD bridging software such as 'My Tax Digital') [Source: https://mtd.digital/mtd-income-tax/mtd-vat-complete-guide/]; bookkeeper or accountant time for first VAT return setup (~£200-£600 one-off if outsourced). [VERIFY: current Xero/QuickBooks list prices at the time of subscription — pricing reshuffles annually].

**Field — Timeline:** Best ~10 working days; **expected 30 working days (HMRC's published service-level expectation for online VAT1)**; worst 60+ working days when HMRC's anti-fraud checks flag the application (very common for new shell-like companies with no trading history, no employees, and a residential-style registered office — Sorellon fits this profile). Effective date of registration can be backdated by up to 4 years in voluntary cases. [VERIFY: HMRC's 'VAT registration: how long it takes' page for current published average — has been ~30 working days for ~18 months but creeps].

**Field — Issuing body:** HM Revenue & Customs (HMRC). Online registration at https://www.gov.uk/register-for-vat (Government Gateway login required). VAT helpline: 0300 200 3700 (Mon-Fri 8am-6pm). Correspondence: HM Revenue and Customs, VAT Registration Service, Crown House, Birch Street, Wolverhampton, WV1 4JX. There is no published direct email; queries flow through the helpline or the secure messaging inbox inside the HMRC online account.

**Field — Application path:** (1) Create or sign in to the company's HMRC business tax account using its Government Gateway ID; (2) select "Add a tax" → VAT; (3) complete VAT1 online — required information: company name, Companies House number (16575318), UTR, registered office, nature of business (free text + SIC 46900), expected annual taxable turnover (state low/£0 if voluntary), date business started/will start trading, bank details for VAT refunds, contact details, evidence of intention to trade if pre-revenue (HMRC will request: invoices in/out, supplier contracts, lease, marketing collateral); (4) HMRC verification (may include phone callback or written queries — Sorellon should have RFQ pipeline records and any supplier MOUs ready); (5) receive VAT registration number (9 digits, format GB123456789) and effective date letter; (6) sign up for MTD for VAT (auto-enrolled for new registrations since April 2022) [Source: https://www.xero.com/uk/programme/making-tax-digital/]; (7) authorise MTD-compatible software with HMRC; (8) start charging VAT from the effective date.

**Field — Prerequisites:** Active company with Companies House number, UTR (HMRC issues automatically post-incorporation — see §3.3), Government Gateway business tax account, UK business bank account (HSBC, Lloyds, Barclays, Tide, Starling, Wise Business etc.), evidence of trading intent. If voluntary pre-revenue: prepare an "evidence pack" (1-pager + 2-3 invoices/quotes/RFQs/supplier MOUs) to head off HMRC's anti-fraud queries.

**Field — Renewal cadence:** No annual renewal. Once registered, registration is continuous until cancelled. VAT returns due quarterly (most common), monthly (refund-heavy businesses on application), or annually (Annual Accounting Scheme — turnover ≤ £1.35M). MTD digital submission via approved software is mandatory for **all** VAT-registered businesses regardless of turnover since April 2022 [Source: https://www.xero.com/uk/programme/making-tax-digital/].

**Field — Ongoing obligations:** (a) charge VAT correctly on taxable supplies; (b) issue VAT invoices to other VAT-registered businesses within 30 days of supply; (c) maintain digital records (sales/purchases/VAT) per MTD rules; (d) file the VAT return + pay any VAT due by 1 month + 7 days after end of VAT period; (e) keep records 6 years; (f) update HMRC within 30 days of business detail changes; (g) deregister when turnover falls below £88,000 and is expected to remain so. Late return = £200 penalty point + £200 each subsequent missed return until thresholds reset. Late payment interest at HMRC's published rate (currently 7%+ above Bank of England base).

**Field — Risk if absent:** Sorellon is below threshold, so no statutory risk. But: (a) **commercial cost** — non-VAT-registered suppliers cannot reclaim input VAT on PPE, hygiene, stationery purchases, meaning Sorellon either absorbs ~20% margin compression or quotes higher than VAT-registered competitors; (b) **credibility signal** — public buyers and large B2B customers often treat presence of a VAT number on quotes as a positive signal of seriousness and operational maturity [Source: https://www.taxually.com/blog/is-voluntary-vat-registration-right-for-your-business]; (c) some procurement portals on Find a Tender have a VAT-number field — leaving it blank doesn't disqualify but reduces trust. **No central-government buyer outright refuses non-VAT-registered SMEs**, and the Procurement Act 2023 explicitly requires proportionate treatment of SMEs [Source: https://www.gov.uk/government/publications/procurement-act-2023-short-guides/the-procurement-act-2023-a-short-guide-for-suppliers-html].

**Field — Sequencing recommendation:** **Do early — within the first 60 days of starting active bidding** (i.e. before submitting the first RFQ-driven bid). Voluntary registration should happen ahead of, or in parallel with, ICO data-protection fee assessment and identity verification. Rationale: (a) HMRC's 30-day-expected / 60-day-possible processing window means it is the single longest-lead-time item on this list, (b) procurement portals capture the VAT number once at supplier-registration time and reuse it on every bid, (c) registering after the first big quote risks having to re-issue invoices and confuses buyers.

**Field — Sorellon-specific notes:**
- **Pre-revenue evidence pack** is critical — HMRC routinely rejects pre-revenue voluntary registrations from new shell-style companies. Include: copy of incorporation certificate; signed lease/licence for Avantgarde Tower if available (a residential-style apartment as registered office is itself a flag); sample customer RFQ printouts; any supplier price lists / sample quotes; the sorellon.com website screenshots showing product categories; bank statements showing modest set-up activity.
- **B2B-only profile favours voluntary registration** — Sorellon's customers (other businesses, public bodies) reclaim VAT charged, so VAT adds zero net cost to them while letting Sorellon recover input VAT on PPE/stationery/cleaning-supply purchases.
- **Single-director, residential-style office** is a recognised HMRC risk profile for missing-trader fraud; expect a phone call or letter requesting additional documents. Director Zac should be reachable on a UK landline or mobile that matches Companies House records.
- **MTD lock-in:** once registered, Sorellon is mandatorily on MTD from day one — pick the accounting platform *before* applying (Xero is widely accepted by UK accountants; QuickBooks is common for small wholesalers; FreeAgent is free with NatWest/RBS/Mettle business banking).
- **Domain mismatch risk:** Sorellon's domain (sorellon.com — double L) does not match the historic single-L "sorelon" typo in internal docs. HMRC will cross-reference Companies House data (16575318) — ensure all VAT1 fields use the canonical Companies House name "SORELLON LIMITED" exactly.
- **Effective date trade-offs:** Sorellon can request voluntary registration with an effective date earlier than the application date (backdated). For a brand-new pre-revenue company there is little reason to backdate — backdating means VAT returns covering any nominal pre-registration sales (none for Sorellon) and triggers earlier MTD obligations. Choose the application date or a forward-dated effective date (up to ~30 days ahead).
- **Cash accounting scheme eligibility (turnover ≤ £1.35M):** Sorellon should opt in from day one — it lets Sorellon defer VAT remittance until customers actually pay. Critical for B2B wholesale where net-30 / net-60 payment terms are normal and a public-sector buyer can take 60+ days to settle invoices. Without cash accounting, Sorellon must pay VAT on the invoice date even if unpaid.
- **Flat Rate Scheme NOT recommended:** the FRS (turnover ≤ £150,000) lets businesses charge full VAT but pay HMRC a reduced flat percentage. Wholesale businesses have a sector rate (8.5-9.5%) that combined with the "limited cost trader" 16.5% override usually makes FRS *worse* than standard accounting for wholesalers with significant cost-of-goods input VAT to recover. Standard scheme + cash accounting is correct for Sorellon.
- **Reverse-charge VAT exposure:** PPE/hygiene imports from EU or non-EU suppliers trigger postponed VAT accounting at import. Once VAT-registered, Sorellon should opt in to Postponed Import VAT Accounting (PIVA) — automatic for VAT-registered importers but the VAT declaration on the C79 / monthly statement must be tracked. PIVA improves cash flow vs paying import VAT upfront.
- **EORI number prerequisite for imports:** Sorellon should also obtain an EORI number (free, ~3 working days, applied via HMRC online) before any first import or first export. EORI is distinct from VAT but linked once VAT-registered.
- **Bid-form VAT field — defensive value:** even if HMRC's pre-revenue review takes 60 days, Sorellon can answer "Application in progress, expected number end of [month]" on procurement portals. Most buyers accept this; some flag for follow-up but do not reject outright.

---

### 10.2. HMRC Employer Reference / PAYE Setup

**Field — What it is:** Pay As You Earn (PAYE) is HMRC's system for collecting Income Tax and National Insurance contributions from employee earnings at source. Registering as an employer with HMRC produces an Employer PAYE Reference (ERN, format `NNN/AANNNNN`, e.g. `123/AB45678`) and an Accounts Office Reference (`NNNPNNNNNNNN`, e.g. `123PF00012345`) used for all PAYE remittances. Governed by the Income Tax (Earnings and Pensions) Act 2003 and the PAYE Regulations 2003 (SI 2003/2682). Real-Time Information (RTI) submissions (Full Payment Submission / FPS) are required on or before each payday [Source: https://www.gov.uk/register-employer].

**Field — Mandatory?** *Conditional.* Mandatory **only if** Sorellon pays any individual (including a director) any of the following: (a) £125+ per week / £542+ per month / £6,500+ per year (the 2026/27 Lower Earnings Limit) [VERIFY: exact 2026/27 LEL — sources cite £123/£129/£6,708 across different years; HMRC sets this each tax year]; (b) expenses or benefits in kind to a director or employee; (c) employees who already have another job, get a pension, or receive taxable state benefits; (d) salary that uses up tax-free Personal Allowance. A purely dividend-paid director with **zero salary** can avoid PAYE entirely [Source: https://www.qualitycompanyformations.co.uk/blog/registering-as-an-employer/].

**Field — Eligibility for Sorellon:** Sorellon currently has no employees and no documented director salary. If Zac is paid only via dividends (or zero) PAYE is not required. The moment Sorellon hires anyone, OR Zac takes a salary above the LEL, OR Sorellon offers any taxable benefit (company car, private medical, accommodation), PAYE registration becomes mandatory before the first payday.

**Field — Cost:** £0 application fee. Indirect: payroll software (free options include HMRC Basic PAYE Tools for ≤10 employees; commercial: BrightPay £161/yr, Moneysoft Payroll Manager £85-£185/yr, or included in Xero/QuickBooks). Outsourced payroll bureau: ~£5-£15 per payslip per month. Employer NI on salary above secondary threshold (~£9,100 [VERIFY: 2026/27 secondary NI threshold]) is a substantive cost — 13.8% historically, **but the rate increased to 15.0% from 6 April 2025** [VERIFY: confirm current 2026/27 rate, especially after any Spring Budget 2026 changes].

**Field — Timeline:** Up to **2 weeks before** first payday allowed; HMRC's online registration takes ~5 working days to process and a further **up to 3 weeks** to receive the PAYE reference letter by post (cannot be emailed for security) [Source: https://ltd-companies.co.uk/resources/register-for-paye-uk/]. Plan **at least 4 weeks** lead time before the first salary payment. You cannot register more than 2 months in advance.

**Field — Issuing body:** HMRC (Employer Helpline 0300 200 3200). Online registration at https://www.gov.uk/register-employer through the Business Tax Account (Government Gateway). Correspondence: HMRC, PT Operations North East England, BX9 1AN.

**Field — Application path:** (1) Sign in to the HMRC Business Tax Account; (2) "Add a tax" → PAYE for Employers; (3) complete online form — required: Companies House number (16575318), UTR, registered office, expected first payday, expected number of employees in the next 12 months, type of business (SIC 46900); (4) HMRC posts the ERN + Accounts Office Reference to the registered office (Avantgarde Tower) — typically within 5 working days but allow up to 3 weeks; (5) enrol the PAYE scheme online inside HMRC Online Services; (6) authorise payroll software / agent; (7) run first payroll, submit FPS on or before payday; (8) pay PAYE/NI to HMRC by 22nd of the following month (electronic) or 19th (cheque).

**Field — Prerequisites:** Active company with UTR, Government Gateway business tax account, decision on payroll software, bank account for paying HMRC, employment contracts in place for any incoming hires (legally required — Employment Rights Act 1996 §1 written-statement obligation from day one).

**Field — Renewal cadence:** No renewal — once a PAYE scheme is open it stays open until you formally close it (e.g. company dormant or last employee leaves). Annual obligations: P60s by 31 May, P11Ds by 6 July (benefits in kind), Employer Payment Summary (EPS) when no FPS due in a month.

**Field — Ongoing obligations:** RTI Full Payment Submission on or before each payday; monthly remittance of PAYE+NI; year-end P60s; P11D for any benefits; auto-enrolment pension duties from first eligible employee (Pensions Act 2008 — separate registration with The Pensions Regulator); maintain records 3 years; notify HMRC of scheme closure when no longer needed.

**Field — Risk if absent:** Zero risk if Sorellon stays director-only on dividends. Once a salary above LEL is paid without PAYE registration, HMRC penalties accrue from the missed-FPS dates (£100/month for ≤9 employees if 1 day late, increasing scale). **Procurement-eligibility risk: none for SME bidders with director-only operations.** Public buyers do not require an ERN to qualify a bidder. Some PPN bid forms ask "do you have employees" but there is no minimum-employee requirement.

**Field — Sequencing recommendation:** **Defer until just-in-time.** Do not register PAYE until Sorellon decides to (a) hire its first non-director employee, or (b) move Zac from pure dividends to a director salary (commonly done to use the Personal Allowance / NI Primary Threshold). At that point register ~4 weeks before the first payday. PAYE setup should *not* block first bids.

**Field — Sorellon-specific notes:**
- **Director-only company strategy:** the conventional small-business approach is for a sole director to take a low PAYE salary (just above the NI secondary threshold) to preserve state-pension years + reclaim Employment Allowance, then top up via dividends. However: single-director companies *cannot claim Employment Allowance* (rules changed April 2016) — so the salary-vs-dividends optimisation differs from multi-employee Ltds. [VERIFY: 2026/27 single-director rule for Employment Allowance — still excluded as of latest HMRC guidance].
- **Bid-form impact:** procurement portals often ask "number of employees" — Sorellon should answer truthfully ("1: the director" or "0") and use the SME self-declaration field to flag micro-enterprise status. This *helps* on Procurement-Act-2023 SME considerations rather than hurting.
- **Public sector "employer covenant" signals:** some PPNs (e.g. PPN 06/20 social value) award points for being a "Real Living Wage" employer — only relevant once Sorellon employs people beyond Zac.
- **No urgent action today.** Revisit only if hiring plan emerges.
- **Auto-enrolment pension:** once Sorellon has any worker classed as "an employee" (not just a director — a director on a contract of employment counts), The Pensions Regulator imposes automatic-enrolment duties. Sole-director companies with no other employee and no employment contract for the director are typically exempt and can submit a "duties don't apply" notification to The Pensions Regulator. If Sorellon later hires a second worker, AE duties trigger immediately.
- **Director's NI considerations:** directors are taxed on an annual earnings basis (not weekly/monthly) for NI purposes — this gives flexibility but also means a one-off bonus at year-end is taxed correctly without retrospective adjustment.
- **PAYE scheme closure if dormant:** if PAYE is set up speculatively and then no salary is paid for an extended period, HMRC may close the scheme — best to delay opening until actually needed.

---

### 10.3. Corporation Tax Registration + UTR

**Field — What it is:** Corporation Tax (CT) is the UK tax on company profits, governed by the Corporation Tax Acts 2009/2010. A 10-digit Unique Taxpayer Reference (UTR) is issued to every newly incorporated UK company by HMRC automatically. The main rate from April 2023 is **25%** (profits > £250,000); small profits rate **19%** (profits ≤ £50,000); marginal relief between. Sorellon, being pre-revenue, has no CT liability yet but must still satisfy registration and dormant-status notification rules [Source: https://www.gov.uk/corporation-tax].

**Field — Mandatory?** *Yes — automatic and unavoidable.* Registration with Companies House triggers automatic Corporation Tax registration with HMRC; HMRC then posts the CT UTR to the registered office within ~14 days [Source: https://www.yourcompanyformations.co.uk/blog/company-utr-number-explained/]. Within **3 months of starting to trade** (defined broadly: trading, advertising for business, employing staff, renting premises, or earning any taxable income), the company must notify HMRC the company is "active" via the Government Gateway. Until then it can be treated as dormant.

**Field — Eligibility for Sorellon:** Already registered (automatic at incorporation, 2025-07-10). UTR should have been posted to Avantgarde Tower in late July / early August 2025. If Sorellon has been actively soliciting business via the sorellon.com website and the RFQ contact form, it is arguably *trading* now — the 3-month active-notification clock may already have started. [VERIFY: the precise date Sorellon began "trading" by HMRC's definition — soliciting RFQs counts. Notify HMRC active status if more than 3 months has elapsed since first RFQ.]

**Field — Cost:** £0 to register, £0 to receive UTR. Indirect: CT600 filing (annual after first accounts) typically requires accounting software or accountant — ~£300-£900 for a micro-entity CT return at typical UK SME-accountant rates. Late-filing penalty: £100 immediately, £100 more after 3 months, then 10% of unpaid tax after 6 months.

**Field — Timeline:** UTR issued automatically — typically arrives 2-3 weeks post-incorporation. Sorellon's UTR should have been received in late July / early August 2025 at the registered office. If not received: contact HMRC Corporation Tax helpline 0300 200 3410 to request a re-issue using the Companies House number (16575318) and registered-office details. Active-status notification: must be done within 3 months of trading commencement.

**Field — Issuing body:** HMRC. Helpline: 0300 200 3410. Online: HMRC Business Tax Account, Corporation Tax section. Postal: HM Revenue and Customs, Corporation Tax Services, BX9 1AX.

**Field — Application path:** Registration itself is automatic; the operator steps are: (1) locate the CT UTR — either on the original HMRC letter sent to Avantgarde Tower, or via the HMRC Business Tax Account (under "Self Assessment / Corporation Tax"); (2) sign into Government Gateway and use the UTR to enrol Corporation Tax; (3) within 3 months of trading commencement, notify HMRC the company is active (online form, takes ~10 minutes; required: UTR, date trading started, accounting reference date, registered office, principal place of business if different, nature of business + SIC 46900, parent company details if any); (4) first CT return (CT600) due 12 months after accounting period end; (5) first CT payment due 9 months + 1 day after accounting period end.

**Field — Prerequisites:** Companies House incorporation (done). UTR letter received (or recovered via helpline). Government Gateway business tax account.

**Field — Renewal cadence:** Annual CT600 return + payment cycle. First CT600 covers the period from incorporation to first accounting reference date (2026-07-31 by default for a 2025-07-10 incorporation; can be shortened or extended once per 5 years), then annual thereafter. Sorellon's first accounts are due at Companies House 2027-04-10 (per internal docs) [VERIFY: confirm against Companies House public record for #16575318 since the default is 21 months from incorporation = 2027-04-10, which matches].

**Field — Ongoing obligations:** Maintain accounting records (sales, purchases, assets, liabilities) 6 years; file CT600 annually; pay CT 9 months + 1 day after period end; file abbreviated/full accounts at Companies House; notify HMRC of accounting reference date changes; notify dormancy/reactivation.

**Field — Risk if absent:** UTR is required on virtually every UK procurement supplier form. Buyers cross-check the UTR against the Companies House number and the VAT number for fraud screening. **Absent UTR = bid stops at the financial-due-diligence stage.** Active-status non-notification within 3 months of trading: HMRC may issue an estimated assessment + late-filing penalties. Probability buyer rejects bid for missing UTR: near 100% for central-government tenders that ask for it.

**Field — Sequencing recommendation:** **Already done.** Just ensure the UTR is *located* and recorded in Sorellon's bid-prep dossier alongside the Companies House number and (when obtained) the VAT number. If Sorellon has been advertising/RFQ-soliciting more than 3 months, file the active-status notification this week.

**Field — Sorellon-specific notes:**
- **Pre-revenue does not mean dormant.** Soliciting RFQs, hosting a marketing site, and maintaining a business email may already constitute "trading" for HMRC purposes even with zero invoices issued. The safe path: file the active-status notification now and report £0 trading until invoices begin.
- **First CT return will be small/zero.** With pre-revenue status the CT600 will report nil profits, but the return is still mandatory once the company is active.
- **UTR recovery:** if the UTR letter never reached Avantgarde Tower (mail-handling apartments often miss HMRC post), Dieter or Zac can phone 0300 200 3410, identity-verify against Companies House data, and have the UTR posted again — typically 7-10 working days.
- **Bid-form check:** when registering on Find a Tender's Supplier Information Service (SIS), the UTR field is mandatory. Have it in the dossier before that registration step.
- **Dormant vs active accounting reference dates:** Sorellon's default first accounting reference date is 31 July 2026 (end of incorporation month + 1 year). Internal docs say first accounts are due 2027-04-10, which is consistent with the default 9-months-after-period-end rule (31 Jul 2026 + 9 months = 30 Apr 2027 — close to but not exact 2027-04-10; [VERIFY] Companies House public record for the exact accounting reference date — Sorellon may have shortened the first period or accepted the Companies House default of 21 months from incorporation = 31 Mar 2027 + 9 months = 31 Dec 2027 for *Companies House* filing; the 2027-04-10 date may be for HMRC purposes).
- **CT accounting period vs Companies House accounting period:** the two are usually identical for the first year, but if Sorellon's first CH accounting period exceeds 12 months (which the default 21-month rule produces), HMRC requires *two* CT returns: one for the first 12 months and one for the residual.
- **HMRC reactivation:** if Sorellon was flagged dormant and is now active, the active-status notification can be completed online inside the Business Tax Account in ~10 minutes.

---

### 10.4. Companies House Confirmation Statement

**Field — What it is:** An annual statement (Form CS01) confirming that the information Companies House holds about a UK company is accurate. Replaced the old "annual return" in 2016. Governed by the Companies Act 2006 (as amended by ECCTA 2023). Confirms: registered office; SAIL (Single Alternative Inspection Location) if any; officers (directors, secretary); People with Significant Control (PSC) register; statement of capital + shareholders for share-capital companies; principal SIC codes (1-4 codes) [Source: https://www.gov.uk/guidance/confirmation-statement-guidance]. From 5 March 2024 (ECCTA) it also confirms the registered email address and a lawful-purpose statement.

**Field — Mandatory?** *Yes, absolutely.* Every UK company — dormant, non-trading, or active — must file at least one confirmation statement every 12 months. Sorellon's first confirmation statement is due **2026-07-23** per internal docs (one year + 14-day filing window from incorporation 2025-07-10) [VERIFY: confirm 'confirmation date' on Companies House public record for #16575318 — typically same day as incorporation, with 14 days to file].

**Field — Eligibility for Sorellon:** Fully applicable — Sorellon is a UK private company limited by shares.

**Field — Cost:** **£50 online** (digital filing fee from 1 February 2026) — up from £34 in 2025; **£110 by post** [Source: https://www.gov.uk/government/news/companies-house-fees-are-changing-from-1-february-2026]. The same fee covers all confirmation statements in the 12-month "payment period" — additional filings within the year are free of additional fee, but each filing must still be made. The fee is collected at the **first** filing in each payment period; subsequent statements in the same period are free.

**Field — Timeline:** Online filing is processed within minutes-hours; the public register typically updates within 24 hours. Paper filings: 5-15 working days. The statement must be filed within **14 days** of the confirmation date (Sorellon's: 2026-07-10, so file by 2026-07-23).

**Field — Issuing body:** Companies House. Online: https://www.gov.uk/file-your-confirmation-statement-with-companies-house (WebFiling portal — needs the company's authentication code, a 6-character code mailed to the registered office shortly after incorporation). Helpline: 0303 1234 500 (Mon-Fri 8:30am-6pm). Email enquiries@companieshouse.gov.uk. Registered offices: Crown Way, Cardiff, CF14 3UZ (England & Wales companies).

**Field — Application path:** (1) Sign in to Companies House WebFiling with company authentication code; (2) start a new CS01; (3) review pre-filled data: registered office, officers, PSCs, shareholders, SIC codes; (4) make any updates needed (separate forms first for: officer changes AP01/CH01/TM01, registered office AD01, PSC changes PSC01/PSC07); (5) confirm registered email address (ECCTA requirement); (6) confirm lawful-purpose statement (ECCTA requirement: "the intended future activities of the company will be lawful"); (7) pay £50; (8) submit. Confirmation receipt issued immediately.

**Field — Prerequisites:** Authentication code (mailed to Avantgarde Tower at incorporation — if lost, request reissue online, takes ~5 working days by post). Up-to-date PSC register. Up-to-date shareholder register (companies with share capital). Up-to-date SIC code list (currently 46900). Registered email address (ECCTA, since March 2024). **Crucially from 18 Nov 2025 onwards:** all directors and PSCs must have their Companies House identity verification status reflected in the next confirmation statement — see §3.10.

**Field — Renewal cadence:** Annual. The confirmation date defaults to the anniversary of incorporation (or anniversary of the last confirmation statement). The company can voluntarily file early and "reset" the next-due date.

**Field — Ongoing obligations:** File annually; maintain the underlying registers (PSC, members, officers); file separate event-based forms within their own deadlines (e.g. director appointment within 14 days using AP01); update SIC code when business activities change.

**Field — Risk if absent:** Failure to file is a **criminal offence** by the directors (Companies Act 2006 §853A) — director-level fines up to £5,000 + personal criminal record. Companies House may **strike off** the company (publishing in the Gazette, 2-3 month grace period). For Sorellon as a bidder: **dissolved or struck-off company = automatic disqualification from every UK public tender**. Procurement systems pull live Companies House status; "Dissolved" or "Active - Proposal to Strike Off" is an immediate disqualifier. Buyers will also reject bids where the confirmation statement is overdue and the company shows as "Active - Overdue Filing".

**Field — Sequencing recommendation:** **Critical — file by 2026-07-23.** Combine with identity-verification deadline checks (Zac's IV deadline 2026-07-23 aligns with confirmation statement due date — they are designed to coincide).

**Field — Sorellon-specific notes:**
- **First confirmation statement is the natural deadline for Zac's identity verification.** ECCTA's transitional rule: existing directors must verify identity by the next confirmation statement after 18 Nov 2025. Sorellon's first confirmation statement (2026-07-23) is therefore the *effective* deadline for Zac's IV.
- **PSC register accuracy is critical.** With two PSCs (Zac >50%, Dieter 25-50%), the PSC categories must be correctly recorded (e.g. "Ownership of voting rights — 50% or less but more than 25%" for Dieter; "Ownership of voting rights — more than 50% but not more than 75%" for Zac, depending on the exact split). Any inconsistency between the share register and the PSC register will be flagged.
- **Registered email address:** ensure the company's registered email at Companies House is one that is actively monitored (suggest a shared mailbox: secretary@sorellon.com or directors@sorellon.com, not Zac's personal address).
- **SIC code review:** SIC 46900 (Non-specialised wholesale trade) is broad enough to cover Sorellon's PPE / hygiene / facilities / education / sustainability mix, but consider adding more specific secondary codes (e.g. 46491 Wholesale of paper & stationery, 46740 Wholesale of hardware/plumbing/heating equipment, 46499 Other wholesale of household goods) if specific bids ask buyers to filter by detailed SIC.
- **Authentication code:** if lost, request reissue at https://find-and-update.company-information.service.gov.uk/ — needs Companies House Online sign-in for the company.
- **Two-PSC dynamic:** with Zac (>50%, director + PSC) and Dieter (25-50%, PSC-only), Sorellon must keep the PSC register entries crisp — particularly the *nature of control* fields (Companies House offers 5 standard "natures" of control: ownership of shares, voting rights, right to appoint/remove directors, significant influence/control, and special category for trusts/firms). Most-likely correct entries: Zac — "more than 50% but not more than 75% of shares" + "more than 50% but not more than 75% of voting rights"; Dieter — "more than 25% but not more than 50% of shares" + "more than 25% but not more than 50% of voting rights". Cross-check against Sorellon's share register (number of shares issued and held by each PSC).
- **Lawful-purpose statement:** ECCTA requires confirming that Sorellon's intended activities are lawful. For a wholesaler selling regulated medical PPE / biocidal hygiene wipes (EN14476 etc.), ensure Sorellon has internally noted compliance with the relevant downstream regulations (UKCA marking for PPE; Biocidal Products Regulation for sanitiser/wipes claims) — these don't get filed but the director is signing the lawful-purpose declaration on the public record.
- **Confirmation statement timing relative to bid windows:** if Sorellon is mid-bid on 23 Jul 2026 and the confirmation statement is still pending, Companies House will mark the public record "Confirmation statement overdue" within ~14 days of the due date — this is visible to all buyers. File on the date due, not later.
- **Combined dependencies:** the 2026-07-23 confirmation statement is the natural milestone where (a) Zac's identity verification must be done; (b) the PSC register must be accurate; (c) the registered email must be confirmed; (d) SIC codes must be reviewed. Treat it as a quarterly mini-audit, not a one-click filing.

---

### 10.5. Modern Slavery Act §54 Statement

**Field — What it is:** Section 54 of the Modern Slavery Act 2015 requires every commercial organisation carrying on business in the UK with a **total turnover of £36 million or more** in its previous financial year to prepare and publish, every financial year, a "slavery and human trafficking statement" describing the steps it has taken to ensure modern slavery is not occurring in its business or supply chains [Source: https://www.legislation.gov.uk/ukpga/2015/30/section/54] [Source: https://www.gov.uk/guidance/publish-an-annual-modern-slavery-statement]. The statement must be approved by the board (or equivalent) and signed by a director, published in a prominent place on the organisation's UK website, and (strongly encouraged, not yet mandatory) added to the UK Government's Modern Slavery Statement Registry. The Home Office published updated Transparency in Supply Chains (TISC) statutory guidance on **25 March 2025** [Source: https://www.lw.com/en/insights/uk-government-issues-updated-modern-slavery-act-transparency-guidance-key-takeaways-for-businesses].

**Field — Mandatory?** *Conditional — not mandatory for Sorellon today.* The £36M threshold means Sorellon (pre-revenue, ~10 months old, single director) is several orders of magnitude below the statutory trigger. **However:** the Home Office has publicly committed (in the 2025 UK Trade Strategy) to *consider* lowering the threshold, expanding the reporting obligation, and introducing penalties for non-compliance [Source: https://cms.law/en/gbr/legal-updates/section-54-modern-slavery-act-where-do-we-go-from-here]. As of May 2026 no statutory change has been made to the £36M figure [VERIFY: check legislation.gov.uk and Home Office news pages for any 2026 statutory instrument amending §54].

**Field — Eligibility for Sorellon:** Eligible to publish a *voluntary* statement. The Home Office and the Modern Slavery Statement Registry both accept voluntary statements from organisations below the threshold and explicitly encourage them [Source: https://www.gov.uk/guidance/add-your-modern-slavery-statement-to-the-statement-registry]. For a pre-revenue B2B wholesaler bidding into public-sector supply chains, a voluntary statement is a low-cost credibility lever.

**Field — Cost:** £0 to publish on the company website. £0 to add to the gov.uk registry. Indirect: drafting time (the first statement typically takes 5-15 hours for a small organisation if done in-house; £500-£2,000 if outsourced to a specialist consultancy). Annual refresh: ~2-5 hours.

**Field — Timeline:** Drafting: 1-2 weeks part-time. Board approval: 1 directors' meeting (minute it). Publication on website: minutes. Registry upload: 1-2 days for Home Office moderation review.

**Field — Issuing body:** Home Office (policy owner). UK Government Modern Slavery Statement Registry: https://modern-slavery-statement-registry.service.gov.uk/. Contact: modernslaverystatements@homeoffice.gov.uk. Statutory guidance: "Transparency in Supply Chains: a Practical Guide" (Home Office, March 2025 update) [Source: https://assets.publishing.service.gov.uk/media/68873308cec9ccd515ae09b0/Transparency_in_supply_chains_a_practical_guide.pdf].

**Field — Application path:** (1) Draft a statement aligned to the six areas recommended by Home Office statutory guidance: (a) organisational structure & supply chains; (b) policies in relation to slavery and human trafficking; (c) due diligence processes; (d) risk assessment & management; (e) effective performance indicators; (f) training & capacity-building. For a pre-revenue B2B wholesaler, write it honestly — describe the *intended* due-diligence and supplier-vetting processes, even if no transactions have occurred yet. (2) Board approval — pass a directors' resolution; sign by Zac as the director. (3) Publish on sorellon.com — link from the website footer ("Modern Slavery Statement"). (4) Submit to the registry: create an account at https://modern-slavery-statement-registry.service.gov.uk/, fill in the organisation profile, paste/upload the statement, answer the optional structured questions (encouraged), submit. (5) Repeat annually.

**Field — Prerequisites:** Live company website with a stable URL for the statement; director sign-off (Zac); a credible description of supply-chain due-diligence (even if just the policy commitments and supplier-onboarding checklist Sorellon plans to use).

**Field — Renewal cadence:** Annual. Must be updated and republished every financial year, within 6 months of the financial year end per the Home Office's strongly-recommended timeframe [Source: https://www.gov.uk/guidance/publish-an-annual-modern-slavery-statement]. Sorellon's first financial year ends 2026-07-31 by default (extended to align with first accounts due 2027-04-10), so the first voluntary statement would naturally be published by Q1-Q2 2027.

**Field — Ongoing obligations:** Annual refresh; keep website link live; maintain underlying supplier due-diligence records; if the company grows past £36M turnover, the statement becomes statutory and the content/quality bar rises.

**Field — Risk if absent:** No statutory penalty (under threshold). However: (a) many central-government and large public-sector tender forms ask "do you have a modern slavery statement and if so where is it published?" — answering "no" is permitted under threshold but a "yes + URL" is a clear positive signal; (b) Crown Commercial Service guidance to public buyers (PPN 09 on tackling modern slavery in supply chains) actively encourages buyers to include modern-slavery-awareness questions even for sub-threshold suppliers [Source: https://www.gov.uk/government/publications/ppn-009-tackling-modern-slavery-in-government-supply-chains/ppn-009-guidance-on-tackling-modern-slavery-in-government-supply-chains-html]; (c) some buyers will deduct points in the social-value scoring (PPN 06/20) for absence of any statement; (d) being on the registry is publicly searchable and improves discoverability in supplier-due-diligence searches by larger buyers and primes.

**Field — Sequencing recommendation:** **Do early-to-mid priority — within the first 90 days of starting to bid.** Lower urgency than VAT/UTR/IV (no hard deadline; not legally required) but high marginal value for credibility in public-sector PPE/healthcare/education tenders where modern-slavery risk is salient. Publish before the first significant public-sector bid.

**Field — Sorellon-specific notes:**
- **Honesty about pre-revenue status:** the first statement should explicitly say "Sorellon Limited is in its first financial year and has not yet completed any commercial transactions. This statement sets out the policies and processes Sorellon will apply as it begins trading." This is a *known acceptable* framing for early-stage voluntary statements per Home Office guidance.
- **Supplier-vetting framework:** sketch a supplier-onboarding checklist (Sedex / SMETA membership for tier-1 suppliers, supplier code of conduct, modern-slavery declaration in supplier T&Cs, geographic-risk screening for upstream sources especially China/India/Bangladesh for PPE and stationery).
- **Healthcare/education focus:** Sorellon's product mix (PPE, hygiene wipes, education stationery) maps directly onto categories of *elevated* modern-slavery risk (manufacturing of medical PPE in regions with known forced-labour concerns; cotton-based hygiene wipes; stationery and craft supplies with known child-labour risk). A serious statement that *acknowledges* these category risks is far more credible than a boilerplate template.
- **Hosting:** publish at sorellon.com/modern-slavery-statement and link from the website footer. Use a stable PDF as well so it can be cited in tender responses.
- **Registry submission:** the Modern Slavery Statement Registry is gov.uk-hosted and will give Sorellon a public URL like `https://modern-slavery-statement-registry.service.gov.uk/statement-summary/<id>` — useful as a citable proof in bid forms.
- **Domain consistency:** ensure the statement, website footer link, and registry record all use `sorellon.com` (double-L) consistently. Any single-L "sorelon" typo undermines the audit trail.
- **[VERIFY]:** as of May 2026, monitor Home Office news — the 2025 Trade Strategy commitment to reform §54 may produce a statutory instrument or new Bill in 2026; threshold reduction (commonly mooted at £18M or even lower) would not directly affect Sorellon for several years but is worth tracking.
- **Approval evidence:** the Home Office statutory guidance is explicit that the statement must be approved by the board (or equivalent governing body) and signed by a director. For Sorellon with sole director Zac, a directors' written resolution stating "the Board approved the Modern Slavery Statement dated [date]" signed by Zac is sufficient. Save the signed resolution in the legal folder.
- **Statement length and quality bar:** voluntary statements at the small-business level are typically 2-4 pages. Anything shorter than 1 page risks looking tokenistic; anything longer than 5 pages risks over-promising. Aim for ~1500-2500 words.
- **Year-on-year content:** the *second* statement (next year) must show progress against the first — even small concrete steps (e.g. "we added a modern slavery clause to our standard supplier contract"; "we added a Sedex-membership request to our supplier onboarding form"). Keep a running log of evidence throughout the year.
- **Bid-form citing:** when a public-sector bid form asks for the modern slavery statement, cite both URLs: (1) sorellon.com/modern-slavery-statement (the live HTML page), and (2) the gov.uk registry permalink. Two sources improves trust.
- **Coordinated with Cyber Essentials etc:** if Sorellon later certifies Cyber Essentials, ISO 27001, ISO 9001, Sedex, or similar, the modern-slavery statement should evolve to reference those (signals of an actual management system).

---

### 10.6. ICO Data Protection Fee

**Field — What it is:** Under the Data Protection (Charges and Information) Regulations 2018 (made under the Digital Economy Act 2017), every UK organisation that determines the purposes of processing personal data (i.e. acts as a data controller) must pay an annual fee to the Information Commissioner's Office unless covered by an exemption. The fee is set in three tiers based on size and turnover [Source: https://ico.org.uk/for-organisations/data-protection-fee/data-protection-fee/]:

- **Tier 1 (micro-organisations):** £52/year — turnover ≤ £632,000 OR ≤ 10 staff.
- **Tier 2 (SMEs):** £78/year — turnover ≤ £36 million OR ≤ 250 staff (and not Tier 1).
- **Tier 3 (large organisations):** £3,763/year — all others.

Direct-debit discount: £5 off (so Tier 1 = £47, Tier 2 = £73, Tier 3 = £3,758). Fees were last increased in February 2025 [Source: https://acuitylaw.com/rise-in-ico-fees-data-protection-fees-february/].

**Field — Mandatory?** *Conditional — likely yes for Sorellon once trading.* The fee is mandatory for any organisation processing personal data as a controller unless an exemption applies. Exemptions include: (a) not-for-profit purposes (charities/clubs); (b) processing only for staff administration in a household; (c) processing only for advertising/marketing/PR of one's own business in a narrow sense (rarely applicable in practice); (d) maintaining a public register; (e) judicial functions; (f) personal/household activities; (g) **not yet trading.**

**Field — Eligibility for Sorellon:** *Currently exempt under "not yet trading."* The ICO's published self-assessment explicitly states: "Companies that are not yet trading don't have to pay a fee to the ICO. Once you start trading you should retake this self assessment to decide if you need to pay the fee." [Source: https://ico.org.uk/for-organisations/data-protection-fee/data-protection-fee-self-assessment/shared/results/result-companies-house-not-trading/]. However: Sorellon has an active website with a B2B RFQ contact form that collects names, work emails, work phone numbers, company names — this is **processing of personal data**. The "not yet trading" exemption is narrow: it covers companies that have not begun any commercial activity. Soliciting RFQs and collecting prospect details *may already* constitute "processing personal data in the course of business," even if no sales have happened yet. **The conservative view:** Sorellon should pay Tier 1 now.

**Field — Cost:** £52/year (Tier 1, online card payment) or £47/year (Tier 1, direct debit). Penalty for non-payment: ICO can fine **up to £4,350** (statutory maximum); fixed penalties are Tier 1 £400, Tier 2 £600, Tier 3 £4,000 [Source: https://ico.org.uk/for-organisations/data-protection-fee/data-protection-fee/penalties/]. The ICO issues a "Notice of Intent to Fine" 28 days after expiry of the payment grace period, with a further 28 days to pay or make representations.

**Field — Timeline:** Online registration + payment: ~15 minutes. The ICO confirms registration immediately on payment and publishes the controller on the public register at https://ico.org.uk/ESDWebPages/Search within 24-48 hours. Renewal reminder: 1 month before expiry.

**Field — Issuing body:** Information Commissioner's Office (ICO). Address: Wycliffe House, Water Lane, Wilmslow, Cheshire SK9 5AF. Phone: 0303 123 1113 (Mon-Fri 9am-5pm). Email: casework@ico.org.uk. Registration portal: https://ico.org.uk/registration. Self-assessment: https://ico.org.uk/for-organisations/data-protection-fee/self-assessment/ (a.k.a. ico.org.uk/fee-checker).

**Field — Application path:** (1) Run the self-assessment at ico.org.uk/fee-checker — answer honestly about trading status, processing categories, size, turnover; (2) on the "you need to pay" outcome, click through to registration; (3) provide organisation details: company name (SORELLON LIMITED), Companies House number 16575318, registered office Avantgarde Tower (the ICO uses this as the controller address), trading name(s), contact name (Dieter or Zac), contact email (e.g. dpo@sorellon.com or director@sorellon.com), business sector; (4) declare tier (Tier 1 based on size); (5) choose payment method (card or direct debit — DD recommended for the £5 discount + automatic renewal); (6) pay £52 (or £47 via DD); (7) receive confirmation + public-register entry.

**Field — Prerequisites:** Companies House number (16575318), registered office address, payment card or UK bank account for direct debit, primary contact details, an accurate self-assessment of trading status.

**Field — Renewal cadence:** Annual. Direct debit auto-renews; card-payment customers receive an email reminder ~1 month before expiry.

**Field — Ongoing obligations:** Keep the public-register entry up to date (changes to organisation name, address, contact). Pay annually. Note: paying the fee is *not* "GDPR registration" — there is no separate GDPR registration. The fee is administrative and does not give a compliance imprimatur. Sorellon must still independently comply with UK GDPR + DPA 2018.

**Field — Risk if absent:** **High enforcement risk + procurement reputational risk.** The ICO has published its list of monthly penalty notices for non-payment since 2019 and the list is public [Source: https://ico.org.uk/about-the-ico/our-information/penalty-notices-issued-for-non-payment-of-data-protection-fee/]; appearing on it is a poor signal to public buyers. Many UK public-sector tender forms (especially health/education) ask "Are you registered with the ICO? Please provide your registration number." Absence is a flagged response and often a scoring deduction.

**Field — Sequencing recommendation:** **Do early — within first 30 days of the very first RFQ form submission on sorellon.com.** Even if conservatively classified as "not yet trading", the £52 cost is trivial relative to the regulatory and procurement risk of getting the call wrong. Pay before the first public-sector bid.

**Field — Sorellon-specific notes:**
- **RFQ form is the trigger.** A live website that collects identifiable personal data (name, email, phone, job title) for the purpose of business outreach is processing personal data; the "not yet trading" exemption is intended for genuinely dormant Companies House registrations, not active marketing operations.
- **Tier 1 fits Sorellon.** Turnover £0 and staff 1, comfortably within Tier 1 (≤ £632k turnover OR ≤ 10 staff).
- **Use direct debit:** £47 vs £52 saving + auto-renewal removes the risk of forgetting and triggering the £400 fixed penalty.
- **Registered controller contact:** appoint Dieter or Zac (or a shared mailbox like `privacy@sorellon.com`) as the ICO-facing contact; whoever it is must monitor the mailbox for any ICO correspondence.
- **Public register signal:** having Sorellon listed at https://ico.org.uk/ESDWebPages/Search as registration ZA-prefixed-number is a citable proof in tender forms and a credibility signal.
- **No DPO required.** Sorellon does not meet any of the GDPR Article 37 mandatory-DPO triggers (public authority; large-scale systematic monitoring; large-scale special-category processing). Having a *voluntary* contact name labelled "data protection contact" is sufficient.
- **Privacy notice as a paired deliverable:** paying the ICO fee does not by itself meet Articles 13/14 transparency obligations. Sorellon needs a public privacy notice at sorellon.com/privacy linked from the RFQ form. Use the ICO's SME privacy-notice template generator at https://ico.org.uk/for-organisations/sme-web-hub/create-your-own-privacy-notice/ — output is ready in ~30 minutes for a single-purpose site.
- **Cookie notice:** if sorellon.com sets any non-essential cookies (analytics, marketing), PECR (Privacy and Electronic Communications Regulations 2003) requires a cookie consent banner. If sorellon.com is a static Netlify page with only essential cookies for the RFQ form, no consent banner is needed — but the privacy notice must still describe any cookies set.
- **Cross-reference with Sorellon's directors:** Dieter's German residence-history might trigger questions about EU data-subject reach if Sorellon ever processes EU personal data. Today, processing is UK-only.

---

### 10.7. GDPR Article 30 — Records of Processing Activities (ROPA)

**Field — What it is:** UK GDPR Article 30 requires controllers and processors to maintain written records of their processing activities. Required content (controller side) includes: name and contact details of the controller (and DPO if any); purposes of processing; categories of data subjects and personal data; recipients (including any in third countries); transfer safeguards for third-country transfers; envisaged retention periods; and a general description of technical and organisational security measures (Article 32) [Source: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/accountability-and-governance/documentation/what-do-we-need-to-document-under-article-30-of-the-gdpr/].

**Field — Mandatory?** *Conditional.* Article 30(5) provides a limited exemption for organisations with **fewer than 250 employees** — but the exemption does **not** apply where the processing is: (a) likely to result in a risk to the rights and freedoms of data subjects; (b) **not occasional** (i.e. systematic/regular); or (c) involves special-category data (Article 9) or criminal-convictions data (Article 10) [Source: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/accountability-and-governance/documentation/who-needs-to-document-their-processing-activities/]. The ICO and EDPB both note this exemption rarely applies in practice for businesses with any sustained customer-data activity — a CRM, mailing list, RFQ pipeline, or employee records nearly always constitutes regular/systematic processing.

**Field — Eligibility for Sorellon:** Sorellon has <250 employees (1), but its RFQ form, B2B prospect mailing list (if any), Google Workspace contact records, and supplier records are **regular and systematic** — not occasional. Therefore Sorellon is **required** to maintain a ROPA covering at least the RFQ/customer-prospect and supplier-onboarding processing streams.

**Field — Cost:** £0 to produce in-house using ICO templates. ICO publishes free downloadable templates at https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/accountability-and-governance/documentation/. Outsourced: ~£250-£800 for a small business privacy starter pack (ROPA + privacy notice + DPIA-lite template).

**Field — Timeline:** 4-12 hours of work for a single director / small business with simple processing flows. Best done as a one-time setup using the ICO controller template, then updated quarterly.

**Field — Issuing body:** ICO (guidance + templates). No registration; ROPA is internal documentation that must be produced on request.

**Field — Application path:** (1) Download ICO's controller ROPA template (Excel/Word) from the ICO website; (2) identify processing streams — for Sorellon expect roughly: RFQ form leads, supplier contacts, employees (currently just Zac for PAYE purposes once that's done), accounting (HMRC), marketing/newsletter (if any), bid-form submissions to public buyers (which contain Zac's/Dieter's personal data), website analytics (if any cookies set); (3) for each stream fill: purpose, lawful basis (Art 6), special-category basis if any (Art 9), categories of data, categories of recipients (Google Workspace, Netlify, HMRC, Companies House, ICO, accountant, banks), retention period, security measures (Google 2FA, password manager, encrypted disk, locked office); (4) store the ROPA in a versioned location (Google Drive folder restricted to directors); (5) review quarterly + on any change of processor or new processing activity.

**Field — Prerequisites:** A clear inventory of the data processing the business does (RFQ form, accounting, supplier records, etc.). Awareness of which third-party services are processors (Google Workspace, Netlify) — see §3.8.

**Field — Renewal cadence:** No formal renewal. Update on every material change (new system, new supplier, new processing activity) and at minimum review every 12 months. Best linked to the same calendar review as the ICO fee renewal.

**Field — Ongoing obligations:** Keep it current. Be able to produce it within a reasonable time on ICO request. ICO can issue an Information Notice demanding production of records — failure to comply is a separate offence.

**Field — Risk if absent:** Direct fine under UK GDPR up to £8.7M or 2% of global turnover (whichever is higher) for Article 30 non-compliance — though in practice the ICO uses ROPA absence as evidence of broader accountability failures rather than fining for ROPA alone. Procurement: a small but growing number of public-sector tender forms ask "Do you maintain records of processing activities (ROPA)?" — answering "no" with no qualifier looks bad.

**Field — Sequencing recommendation:** **Do in parallel with ICO fee registration (§3.6).** No external dependency; one focused afternoon produces a defensible baseline ROPA. Ideal sequence: ICO fee paid → ROPA drafted → privacy notice published on sorellon.com → DPAs with sub-processors confirmed.

**Field — Sorellon-specific notes:**
- **Cross-border note:** Sorellon's use of Google Workspace inherently involves data transfer to Google's global infrastructure. The Google Cloud Data Processing Addendum (see §3.8) provides the transfer mechanism. Document this in the ROPA "transfers" column with reference to the Google CDPA and the UK International Data Transfer Addendum (IDTA) / UK-US Data Bridge.
- **Employee data:** even with just Zac as a director, once any salary/expense/benefit is processed, an "HR" processing stream emerges; if PAYE/dividends are processed by an accountant, that accountant is a processor and needs Art 28 DPA.
- **Public-buyer disclosure stream:** when Sorellon submits a Find a Tender / SIS profile or bid, Sorellon discloses the directors' personal data (Zac, Dieter) to public buyers — that's a separate processing stream, lawful basis Article 6(1)(b) (necessary for contract) or (f) (legitimate interests).
- **Risk-of-rights threshold:** Sorellon does not process special-category data and is unlikely to trigger high-risk DPIA conditions — so the simplified ROPA template is appropriate.
- **Practical ROPA contents for Sorellon (worked outline):**
  - *Stream 1 — Prospect/lead capture via RFQ form:* purpose = respond to RFQ + follow-up sales; lawful basis = Art 6(1)(b) (pre-contract) or 6(1)(f) (legitimate interests); categories = work name, work email, work phone, company name, free-text enquiry; recipients = internal team, Netlify (form transit), Google Workspace (mailbox); transfers = via Google Workspace global infrastructure under Google CDPA + UK IDTA; retention = 24 months from last contact; security = TLS, 2FA on mailbox, restricted admin access.
  - *Stream 2 — Supplier onboarding:* purpose = supplier management; lawful basis = Art 6(1)(b) or (f); categories = supplier contact name, work contact details, bank/payment details (if any); recipients = accountant, bank; retention = 7 years after end of relationship (HMRC record keeping); security = same as Stream 1 + access-restricted shared drive.
  - *Stream 3 — Statutory and tax filings:* purpose = legal obligations; lawful basis = Art 6(1)(c); categories = director and PSC personal data; recipients = Companies House, HMRC, ICO; retention = company lifetime + 6 years post-dissolution.
  - *Stream 4 — Bid submissions to public buyers:* purpose = contract bidding; lawful basis = Art 6(1)(b) or (f); categories = director/PSC personal data, possibly supplier references including contact details; recipients = each public buyer (controller in its own right); retention = aligned to procurement file retention (commonly 7 years post-contract).
  - Each stream entry should also fill in: international transfers (yes/no, mechanism); data-subject rights handling pointer; breach-notification process.
- **Maintenance discipline:** simplest pattern is a single Google Sheet with one row per processing stream and seven columns matching ICO template; update in the same calendar slot as ICO fee renewal.

---

### 10.8. GDPR Article 28 — Data Processing Agreement (DPA)

**Field — What it is:** UK GDPR Article 28 requires that wherever a controller uses a processor (a third party processing personal data on the controller's behalf), there must be a written contract setting out: subject matter and duration; nature and purpose; types of personal data; categories of data subjects; rights and obligations of the controller; specific processor duties (process only on documented instructions; confidentiality; security; sub-processor authorisation; assistance with data-subject rights; assistance with breach notification; deletion/return; audit rights) [Source: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/accountability-and-governance/contracts-and-liabilities-between-controllers-and-processors-multi/what-needs-to-be-included-in-the-contract/].

**Field — Mandatory?** *Yes.* Article 28 is a strict legal requirement whenever a controller uses a processor. For Sorellon's stack: Google Workspace = processor; Netlify = processor (hosts the RFQ form submissions, possibly server-side logs); any accountant/bookkeeper = processor; any future CRM/marketing tool = processor.

**Field — Eligibility for Sorellon:** Fully applicable. Sorellon is a controller for all of its customer/prospect/supplier/employee data and uses (at minimum) Google Workspace + Netlify as processors.

**Field — Cost:** £0 — major processors publish their DPAs / Data Processing Addenda that can be accepted electronically:
- **Google Workspace:** Cloud Data Processing Addendum at https://cloud.google.com/terms/data-processing-addendum (accept in Admin Console → Account settings → Legal and compliance → Cloud Data Processing Addendum) [Source: https://cloud.google.com/terms/data-processing-addendum].
- **Netlify:** Data Processing Addendum at https://www.netlify.com/legal/data-processing-addendum/ (auto-incorporated into Netlify's Terms of Service; can be signed via DocuSign for an executed copy on request).
- **For accountant / bookkeeper / payroll bureau:** request their standard DPA template — most UK accountants provide one in their engagement letter.

ICO publishes free model contract clauses that can be inserted into bespoke contracts at no cost. Outsourced legal review: ~£100-£400 for a standard small-business sub-processor DPA review.

**Field — Timeline:** Per-processor, ~15-60 minutes to accept / sign electronically. Bespoke contract drafting: 1-3 days with a solicitor.

**Field — Issuing body:** N/A — bilateral contracts. ICO publishes guidance; ICO has authority to issue standard contractual clauses but to date the major processors' DPAs are the practical default.

**Field — Application path:** (1) List all processors Sorellon uses (the ROPA from §3.7 is the input); (2) for each: locate the processor's published DPA; (3) execute / accept it (Google: Admin Console click-through; Netlify: signed PDF or accept-by-use; accountant: engagement letter); (4) store the executed copy / acceptance evidence in a versioned folder (Google Drive); (5) for each new processor onboarded in future, repeat before any personal data is shared; (6) review annually.

**Field — Prerequisites:** Complete list of processors (from ROPA). Authority to bind the company (Zac as director can sign).

**Field — Renewal cadence:** Generally evergreen — DPAs renew automatically with the underlying service contract. Review annually and on any change to processing scope, processor's sub-processors, or service terms.

**Field — Ongoing obligations:** Monitor processor sub-processor changes (most major processors publish a sub-processor list and notify changes — Google's at https://workspace.google.com/intl/en/terms/subprocessors.html); update DPAs if Sorellon's processing scope changes; revoke and offboard processors cleanly (data return / deletion).

**Field — Risk if absent:** Direct GDPR fine risk same as Article 30 (up to £8.7M or 2% of global turnover). Practical risk for Sorellon: a customer or public buyer asks "do you have DPAs in place with all your sub-processors?" and Sorellon cannot produce one. Loss of public-sector bid credibility, particularly for any health or education tender where the buyer is downstream of any patient/pupil data.

**Field — Sequencing recommendation:** **Do in parallel with ROPA and ICO fee — within first 30-45 days of bid prep.** Accept Google CDPA and Netlify DPA the same day they are encountered in the ROPA work.

**Field — Sorellon-specific notes:**
- **Google Workspace acceptance:** sign in to admin.google.com as the Sorellon super-admin → Account settings → Legal and compliance → Cloud Data Processing Addendum → Review and Accept. Save a PDF copy with the acceptance timestamp.
- **Netlify acceptance:** Netlify's standard DPA is auto-incorporated; for an executed copy email privacy@netlify.com requesting a DocuSign of the standard DPA, which they will return signed by Netlify within ~5 working days. Keep that signed PDF in the Sorellon legal folder.
- **International transfer clauses:** both Google and Netlify rely on the UK International Data Transfer Addendum (IDTA) or the UK-US Data Bridge for transfers outside the UK. Sorellon's ROPA should reference the relevant transfer mechanism per processor.
- **Sub-processor change notifications:** subscribe to Google Workspace and Netlify sub-processor-change emails; review changes within 30 days; the right to object is contractual.
- **Accountant DPA:** the most-forgotten one. When Sorellon engages an accountant or bookkeeper, ensure the engagement letter contains GDPR-compliant Art 28 clauses or attach a separate DPA. Same for any future cloud-based bid-writing tool / CRM (e.g. HubSpot, Salesforce, Pipedrive).
- **No special-category data** in Sorellon's foreseeable processing — simplifies the analysis. If Sorellon later expands into health-and-care contracts where the buyer shares patient data, the DPA bar rises substantially.
- **Worked sub-processor inventory for Sorellon today:**
  - Google Workspace (mail, drive, docs) — Cloud DPA accepted via Admin Console; UK IDTA in place.
  - Netlify (static site hosting + form submissions) — Netlify DPA accepted by ToS; if Netlify Forms is used, form submissions transit Netlify infrastructure.
  - Domain registrar / DNS provider (whoever registered sorellon.com) — review their DPA; if they hold WHOIS contact data they are a processor.
  - Accountant / bookkeeper (when engaged) — DPA in engagement letter.
  - Payroll bureau (when engaged) — DPA in engagement letter.
  - Bank — banking relationships are usually controller-to-controller, *not* processor — no Art 28 DPA needed but check the terms.
  - HMRC, Companies House, ICO — public authorities discharging statutory functions, not processors.
  - Any CRM, mailing tool, e-signature tool added later — DPA required before use.
- **DPA evidence pack:** keep a single PDF "DPA Register" listing each processor, the DPA URL or attached PDF, acceptance date, and sub-processor list URL. Public buyers occasionally request this in detailed due diligence.

---

### 10.9. GDPR Article 27 — Representative

**Field — What it is:** UK GDPR Article 27 (mirroring EU GDPR Art 27) requires controllers or processors **not established in the UK** but offering goods/services to UK data subjects or monitoring their behaviour to designate, in writing, a UK Representative — and the reciprocal EU rule requires non-EU controllers to appoint an EU Representative if they target EU data subjects [Source: https://gdpr-info.eu/art-27-gdpr/].

**Field — Mandatory?** *No — not for Sorellon (UK-established).* Sorellon is a UK-incorporated controller with its sole establishment in the UK. Article 27 (UK or EU) applies only to controllers/processors **not established** in the relevant jurisdiction. Therefore Sorellon does **not** need a UK Representative (it is itself the UK establishment) and does **not** need an EU Representative *unless* it begins offering goods/services to data subjects in the EU/EEA in a manner that targets the EU market.

**Field — Eligibility for Sorellon:** N/A for UK Representative. Conditional for EU Representative: if Sorellon begins selling into Ireland / Germany / Netherlands public-sector or any EU customer base in a targeted manner (e.g. EU-language site, EU pricing in EUR, EU-targeted ads), then an EU Representative becomes required. Sorellon's current footprint (UK domain, GBP pricing, UK-only B2B) does not trigger Art 27.

**Field — Cost:** N/A today. If/when EU expansion happens: third-party EU Representative services typically cost £150-£600/year (e.g. EDPO, GDPR-Rep.eu, Prighter, VGS). Direct appointment of an in-EU lawyer/firm is more expensive (£1,000-£3,000/year).

**Field — Timeline:** N/A today.

**Field — Issuing body:** ICO publishes UK Representative guidance at https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/international-transfers/representatives/; European Data Protection Board (EDPB) Guidelines 3/2018 cover the EU side.

**Field — Application path:** Not required for Sorellon today. If EU expansion occurs: (1) select a UK-or-EU-based representative provider; (2) execute a mandate letter; (3) update privacy notice and ROPA to name the representative and provide their contact details; (4) ensure the representative is reachable by EU data subjects and EU DPAs.

**Field — Prerequisites:** EU establishment in scope of EU GDPR Art 3(2) (offering goods/services or monitoring behaviour of EU data subjects).

**Field — Renewal cadence:** Annual mandate typical.

**Field — Ongoing obligations:** Keep the representative's details current in the public privacy notice and in any communications with EU data subjects / DPAs.

**Field — Risk if absent:** Zero today (UK-established). If Sorellon later targets EU customers without appointing an EU Representative, a one-off EU DPA fine up to €10M or 2% of global turnover (Art 83(4)).

**Field — Sequencing recommendation:** **No action required.** Re-evaluate only on first EU customer.

**Field — Sorellon-specific notes:**
- **Dieter is German** but is resident in England and is acting as a director of a UK company — this does not create any EU establishment. Personal nationality of officers is irrelevant for Art 27.
- **Sorellon's website is UK-targeted** (GBP pricing implied, UK landline, sorellon.com TLD with UK-focused content). No EU-targeting indicators. Safe.
- **Future watchpoint:** if Sorellon bids on cross-border EU public-sector frameworks (rare for UK SMEs post-Brexit) or accepts an EU referral, run a fresh Art 27 assessment before accepting EU prospect data.
- **Indicators of EU targeting (any of which can trigger Art 3(2) EU GDPR):** EU-language localisation; pricing in EUR or other EU currency; payment options for EU customers; references to EU customer testimonials or markets; .eu domain; Google Ads / SEO targeting EU geo; shipping/delivery offered into EU countries; EU phone numbers. None apply to Sorellon today.
- **Reverse direction (UK Representative for non-UK controllers):** not Sorellon's concern, but worth noting — if a non-UK supplier wants to sell to UK consumers and processes their personal data, they need a UK Representative under UK GDPR Art 27. Sorellon may be asked by a non-UK supplier to act as its UK contact in commercial negotiations; that is a *commercial* arrangement, not the same as a statutory Art 27 representative and Sorellon should decline if asked.
- **Brexit residual risk:** in 2026 the UK/EU adequacy decision remains in force (renewed in 2025 for 4 years) [VERIFY: confirm UK adequacy is still current]; if adequacy is revoked or non-renewed in future, transfers to the UK from the EU need additional safeguards — which is a problem for EU customers sending data to Sorellon, not for Sorellon as a UK controller.

---

### 10.10. Companies House Identity Verification (ECCTA)

**Field — What it is:** The Economic Crime and Corporate Transparency Act 2023 (ECCTA) introduced mandatory identity verification at Companies House for every director, Person with Significant Control (PSC), and anyone filing on a company's behalf. Verification produces a unique 11-character Companies House "Personal Code" that links all roles a person holds across multiple companies. The regime commenced **18 November 2025** with a 12-month transition period ending 17 November 2026 [Source: https://www.gov.uk/guidance/verify-your-identity-for-companies-house] [Source: https://companieshouse.blog.gov.uk/2026/01/16/understanding-identity-verification-for-people-with-significant-control-pscs/].

**Field — Mandatory?** *Yes — absolutely.* All UK company directors and PSCs must verify. Two deadline rules apply during the transition:
- **Director (also PSC of the same company):** verify within 14 days starting the day after the company's confirmation-statement date.
- **PSC-only (not director):** verify within the first 14 days of the PSC's birth month.

Sorellon-specific (per internal docs and per the rule structure):
- **Dieter (PSC-only, German, born December [VERIFY exact DOB])** — deadline based on birth month, internal docs cite **14 Dec 2025**, consistent with a birthday somewhere in December 2025/2026 and the rule "14 days of birth month" giving a 1-14 Dec window. [VERIFY this against Dieter's actual DOB month — if December birthday, the 2026 deadline is 14 Dec 2026 (next occurrence); if internal docs say 14 Dec 2025, that window may have already lapsed — confirm and remediate immediately if missed.]
- **Zac (director and PSC, English, born [VERIFY])** — deadline tied to Sorellon's first confirmation statement after 18 Nov 2025; that confirmation statement is due 23 Jul 2026, so Zac's IV deadline is **23 Jul 2026** (consistent with internal docs).

**Field — Eligibility for Sorellon:** Both PSCs eligible. Dieter holds a German passport (biometric, accepted). Zac holds UK ID (biometric passport or UK photo driving licence). Both can use GOV.UK One Login from anywhere in the world.

**Field — Cost:** **Free via GOV.UK One Login.** ACSP (Authorised Corporate Service Provider) route: ~£50 + VAT per person per company if going through an accountant or formation agent; Post Office in-person verification ~£12 per attempt (where GOV.UK One Login cannot complete the digital step) [Source: https://www.identitysolutions.ncino.com/blog/costs-for-identity-verification-of-shareholders].

**Field — Timeline:** **10-15 minutes** end-to-end if all goes smoothly on GOV.UK One Login with a biometric passport + selfie video [Source: https://www.localbusinessmagazine.co.uk/companies-house-identity-verification-rollout/]. Personal Code arrives by email immediately or within a few hours of successful verification. Worst case: ID document rejected (poor scan, expired ID, photo mismatch) → retry; or escalation to Post Office in-person verification, adding ~1-2 weeks.

**Field — Issuing body:** Companies House (via GOV.UK One Login service operated by GDS). Helpline: 0303 1234 500. ACSP register: https://find-and-update.company-information.service.gov.uk/authorised-agent.

**Field — Application path (Zac and Dieter each separately):** (1) Visit https://www.gov.uk/guidance/verify-your-identity-for-companies-house; (2) sign in to / create a GOV.UK One Login account (uses email + 2FA); (3) confirm name, date of birth, current residential address (must match Companies House PSC/director record); (4) provide National Insurance number (for UK residents — Dieter, as a German national resident in England, should have an NI number; if not, alternative ID route); (5) photograph ID document — biometric passport (any country, includes German passport) preferred, or UK photocard driving licence, or UK biometric residence permit / Frontier Worker permit; (6) record a short selfie video for liveness check; (7) Companies House issues the 11-character Personal Code by email; (8) provide the Personal Code to Companies House (and to any ACSP filing on Sorellon's behalf) — entered on the relevant filings (e.g. next confirmation statement, any new appointment).

**Field — Prerequisites:** GOV.UK One Login account (free, ~5 min to create); valid ID document (biometric passport recommended); National Insurance number for UK-residents; address matching Companies House records; ability to record a selfie video.

**Field — Renewal cadence:** One-time identity verification; the Personal Code is permanent unless revoked for fraud / change of legal identity. Re-verification may be required if Companies House loses confidence in a record (e.g. name change, replacement passport with mismatched data).

**Field — Ongoing obligations:** Use the Personal Code on every relevant filing. Re-verify if Companies House requests (rare). Keep ID document details current with the issuing authority (e.g. passport renewal).

**Field — Risk if absent:** **Severe.** Post-transition (after 17 Nov 2026), unverified directors/PSCs commit a criminal offence (Companies House blog 2026-01-16 confirms criminal sanction for failure to verify by deadline). Companies House may refuse filings, mark the company "Active – Identity Verification Outstanding" on the public register, and ultimately initiate strike-off. **Procurement-eligibility risk: very high.** Any buyer pulling Sorellon's Companies House record will see "IV Outstanding" or "Director not verified" — for central-government buyers the bid will be paused or rejected pending remediation; for procurement-fraud-sensitive buyers (defence, NHS), the file is likely closed.

**Field — Sequencing recommendation:** **Highest priority — do *immediately* for Dieter (if the 14 Dec 2025 internal-doc deadline has already passed, the situation is reportable to Companies House and needs to be cured today). Do Zac by Q2 2026 well before the 23 Jul 2026 deadline (target completion June 2026 to leave headroom for any document rejection / Post Office fallback).**

**Field — Sorellon-specific notes:**
- **Dieter's deadline concern:** internal docs cite **14 Dec 2025** which is approximately 5 months *in the past* as of today (2026-05-25). If Dieter has not yet verified, Sorellon is in a state of statutory non-compliance. **Action today:** Dieter to attempt GOV.UK One Login verification this week; if Dieter is already verified, ensure the Personal Code has been provided to Companies House (it must be referenced on filings) and confirm Companies House record reflects "verified" status. If not verified, complete verification + consider a voluntary disclosure to Companies House citing the missed deadline.
- **Zac's deadline:** 23 Jul 2026, same day as Sorellon's first confirmation statement. Sequence the work so Zac is verified *before* the confirmation statement is filed — the CS01 needs Zac's Personal Code if his director role and PSC role are to be confirmed in the same filing.
- **German passport for Dieter:** the GOV.UK One Login service accepts any biometric passport including non-UK passports. No NI number is strictly required if Dieter uses passport + selfie route. However, having an NI number on file accelerates the address-match step.
- **Address consistency:** Dieter's and Zac's Companies House addresses (residential service address, not necessarily registered office) must match the address declared in One Login. Any mismatch will trigger manual review. Update PSC/director addresses at Companies House *before* attempting verification if there has been any house move.
- **Names — canonical spelling.** Dieter Xia Shuang Danicek's name has multiple components — ensure Companies House record spells it exactly as it appears on his German passport; small differences (umlauts, hyphenation) will cause verification failure.
- **ACSP fallback:** if One Login fails twice, escalate to an ACSP — common providers are mainstream UK accountants and formation agents; the £50 + VAT fee is trivial vs the procurement risk of unverified status.
- **Filing dependency:** Sorellon cannot legitimately make many future filings (appointments, address changes, accounts) without the directors' Personal Codes. IV is not just a one-time chore — it is a precondition for all forward operations of the company.
- **Pre-revenue does not exempt.** ECCTA IV applies to every director/PSC regardless of trading status; dormant companies are equally in scope.
- **Failure scenarios and recovery paths:**
  - *Camera/biometric capture fails on phone:* try a different phone with better camera; or use the desktop flow + smartphone camera-link; or escalate to Post Office.
  - *ID document not recognised:* check expiry (passport must be in-date; driving licence must be photocard not paper); try an alternative document; or use NI + bank-credentials route.
  - *Address mismatch (very common — Companies House shows older address):* update PSC/director residential service address at Companies House first (free, ~24 hours to process), then retry IV.
  - *Name mismatch (umlauts, hyphenation, transliteration — relevant for Dieter's German passport name):* the GOV.UK One Login flow now supports unicode characters; if mismatch persists, the issue is usually a Companies House record that strips diacritics — file CH01 to update the director details to exactly match the passport, then retry IV.
  - *Multiple roles across multiple companies:* the same Personal Code applies to all roles; verify once.
- **OAuth / 2FA hygiene:** GOV.UK One Login uses SMS or authenticator-app 2FA. Use an authenticator app (Google Authenticator, Authy, 1Password) rather than SMS to avoid SIM-swap risk on a procurement-critical login.
- **Personal Code storage:** the 11-character Personal Code is private and should be treated like a password. Store in a director-only password manager. It will be needed every time Sorellon files a director appointment, change, removal, or confirmation statement after the transition.
- **ACSP option for Dieter — German national considerations:** if One Login refuses German-passport verification (rare, but possible), an ACSP can do verification in person or remotely. Cost ~£50 + VAT. Use a Companies-House-listed ACSP — see https://find-and-update.company-information.service.gov.uk/authorised-agent.
- **Procurement signalling:** several public-sector tender forms now ask "Have all your directors and PSCs completed Companies House identity verification under ECCTA?" — answering "yes" with the date of completion is a strong fraud-controls signal.
- **Strategic dependency map:**
  - Dieter's IV → unblocks Sorellon's PSC register accuracy → unblocks first confirmation statement
  - Zac's IV → unblocks director-role confirmation → critical for 23 Jul 2026 CS01
  - Both verified → Sorellon's public Companies House record shows "Verified" status → unblocks high-trust procurement bids

---

### 10.11. Cross-Cutting Findings & Operator Decision Notes

**Finding 1 — Sorellon's "regulatory weight" is light but every line item matters.** With only 10 items in this section, Sorellon's statutory burden is small by any measure (a mid-sized B2B wholesaler at ~£10M turnover would have 30+ items including UKCA marking on PPE, Biocidal Products Regulation, ESOS Phase 4, IR35, etc.). But because Sorellon is pre-revenue and bidding from a near-zero credibility baseline, the *signalling value* of every voluntary item (ICO fee, modern slavery statement, IV completion) is disproportionately high.

**Finding 2 — Identity verification is the single most time-critical item.** Two ECCTA deadlines (Dieter approximately end-2025; Zac 23 Jul 2026) bracket the operator's planning horizon. The Dieter deadline, if internally docs-cited 14 Dec 2025 reflects reality, has likely already lapsed — this is the **highest-urgency item** on this section's action list. The Zac deadline is dependency-linked to the confirmation statement.

**Finding 3 — VAT registration is the longest lead-time controllable item.** HMRC's 30-day expected and 60-day-plus realistic processing window means Sorellon should start the VAT application *before* it has any imminent need, not after. Voluntary pre-revenue registration has a non-trivial rejection risk that the operator can mitigate with a strong evidence pack.

**Finding 4 — ICO + ROPA + DPA cluster is a "three-day project."** Pay the £47 DD, open the ICO controller register entry, paste into the standard ROPA template, click-accept Google CDPA and Netlify DPA, draft a 1-page privacy notice. Done. This single afternoon's work converts Sorellon from "GDPR-naive" to "GDPR-baseline-compliant" with strong procurement-form signalling.

**Finding 5 — Modern Slavery is the highest leverage voluntary signal.** A 2-3 page voluntary statement on sorellon.com + registry permalink is cheap, fast, and creates a citable URL that resolves a recurring tender-form question favourably. Recommended even though not statutorily required.

**Finding 6 — Confirmation statement is the orchestrator deadline.** The 23 Jul 2026 confirmation statement filing requires Zac's IV completion, an accurate PSC register, confirmed registered email, and reviewed SIC codes — combining four distinct pieces of preparation into one deadline. Operators commonly fail by treating it as a one-click button on the day — instead, treat it as a quarterly mini-audit milestone.

**Finding 7 — No PAYE and no Article 27 representative needed today.** Two items in this section explicitly *do not* apply to Sorellon today, despite often being assumed mandatory. PAYE is not needed for a dividend-only director with no employees. Article 27 EU representative is not needed for a UK-established controller.

**Finding 8 — Domain-name consistency is a hidden risk.** The historic "sorelon" single-L typo in internal docs is not a regulatory risk in itself, but if it ever propagates to a public filing (Companies House, HMRC, ICO, modern slavery registry) it creates an audit-trail mismatch that procurement portals will flag. Every external filing must use sorellon.com (double-L) consistent with the Companies House canonical name.

**Finding 9 — Avantgarde Tower as registered office.** A residential-apartment registered office is a HMRC anti-fraud flag and a procurement-trust signal. Practical mitigations: ensure registered-office mail is collected promptly (HMRC sends paper letters); maintain a separate trading address if/when warehouse or office space is leased; keep the registered office consistent across Companies House, HMRC, ICO, modern slavery statement, and Find a Tender / SIS records.

**Finding 10 — One Google Doc for the dossier.** Recommend the operator create a single "Sorellon Compliance & Bid Dossier" Google Doc in the Sorellon shared drive containing: Companies House number, name, registered office, all directors and PSCs with their Personal Codes, UTR, VAT number (TBD), ICO registration number (TBD), accountant contact, banks, modern slavery statement URL, privacy notice URL, ROPA URL, DPA-register URL, insurance certificates (when obtained), Cyber Essentials certificate (when obtained). One doc = one source of truth for every bid.

---

### Section Summary — Sequencing Snapshot for Sorellon (as of 2026-05-25)

The following ordering optimises lead-time, dependency, and procurement-readiness:

1. **Immediately (today/this week):**
 - Confirm Dieter's identity-verification status (§3.10) — if not done, complete via GOV.UK One Login this week. Cure any missed 14-Dec-2025 deadline.
 - Locate Sorellon's CT UTR (§3.3) and confirm active-status notification with HMRC if any trading-equivalent activity has occurred.
2. **Next 30 days:**
 - Pay ICO Tier 1 data protection fee (§3.6) via direct debit (£47).
 - Draft Records of Processing Activities (§3.7) using ICO template; cover RFQ-form, Google Workspace, Netlify, supplier records.
 - Accept Google Workspace Cloud DPA and Netlify DPA (§3.8); collect any other processor DPAs.
3. **Next 30-60 days:**
 - Submit voluntary VAT registration (§3.1) with pre-revenue evidence pack — longest lead time (~30 working days expected, 60+ possible). [VERIFY pre-revenue acceptance risk; Sorellon's profile may attract HMRC anti-fraud review.]
 - Draft voluntary Modern Slavery Statement (§3.5); publish on sorellon.com and add to Modern Slavery Statement Registry. Sign by Zac.
4. **By 2026-07-23 (hard deadline):**
 - Complete Zac's Companies House identity verification (§3.10) — well before the deadline (target end of June 2026).
 - File Sorellon's first Confirmation Statement (§3.4) for £50 online via WebFiling.
5. **Deferred / contingent:**
 - PAYE registration (§3.2) — only when first employee or director-salary above LEL.
 - GDPR Article 27 EU Representative (§3.9) — only on first EU customer activity.

> **Critical reminder:** every item that touches a procurement portal (VAT number, UTR, ICO registration number, modern slavery statement URL, Companies House identity-verification status) should be captured in a single bid-prep dossier (suggest a Google Doc in the Sorellon Drive) so that Find a Tender / SIS registration and every subsequent bid pulls from a single, version-controlled source of truth.
>
> **Domain hygiene note:** all of the above filings, registrations, and statements must use the canonical name **SORELLON LIMITED** (Companies House #16575318) and domain **sorellon.com** (double-L). The single-L "sorelon" typo seen in some internal L6 docs must not appear in any external filing — once a regulatory or procurement record contains the wrong spelling it is difficult and costly to correct.

---

### Appendix A — Consolidated Cost & Timeline Matrix (2026 prices, Sorellon-applicable)

| Item | Mandatory? | Direct cost | Indirect cost | Best timeline | Expected timeline | Worst timeline | Hard deadline for Sorellon |
|---|---|---|---|---|---|---|---|
| §3.1 VAT (voluntary) | Conditional | £0 | MTD software £0-£59/mo | 10 working days | 30 working days | 60+ working days | None (commercial) |
| §3.2 PAYE | Conditional | £0 | Payroll software £0-£185/yr | 5 working days | ~3 weeks | 4-6 weeks | None today |
| §3.3 Corp Tax / UTR | Yes (automatic) | £0 | Accountant £300-£900/yr | Already done | Already done | Already done | Active-status notification 3 months post-trading start |
| §3.4 Confirmation Statement | Yes | £50 online / £110 paper | None | 1 hour | 1 day | 14 days | **2026-07-23** |
| §3.5 Modern Slavery (voluntary) | No (under threshold) | £0 | Drafting 5-15 hours | 1 week | 2 weeks | 4 weeks | None statutory; pre-first-major-bid recommended |
| §3.6 ICO Fee | Conditional (likely yes) | £47 (DD) / £52 (card) | None | 15 minutes | 24 hours | 48 hours | Within 30 days of first RFQ activity |
| §3.7 Article 30 ROPA | Conditional (likely yes) | £0 | 4-12 hours initial | Same day | 1 week | 1 month | Before ICO request / first bid asking |
| §3.8 Article 28 DPAs | Yes (per processor) | £0 | 15-60 min per processor | Same day | 1 week | 1 month | Before first personal-data flow to processor |
| §3.9 Article 27 EU Rep | No (UK-established) | N/A | N/A | N/A | N/A | N/A | Only on EU expansion |
| §3.10 IV — Dieter (PSC) | Yes | £0 | 10-15 min | 15 minutes | Same day | 1-2 weeks (Post Office fallback) | **14 Dec 2025** [VERIFY against actual DOB month] |
| §3.10 IV — Zac (Director+PSC) | Yes | £0 | 10-15 min | 15 minutes | Same day | 1-2 weeks | **2026-07-23** (with confirmation statement) |

> **Total cash outflow Sorellon needs to fund in 2026 from this list:** £47 (ICO DD) + £50 (CS01) + £0 (everything else direct) = **£97 statutory base.** With voluntary modern slavery statement drafting (~£500 outsourced max) and accountant CT prep (~£600 if outsourced), realistic annual recurring cost ≈ **£1,200-£2,000** including a basic accountant relationship.

---

### Appendix B — Procurement-Portal Field Map (what each item provides to a bid)

When Sorellon registers on Find a Tender's Supplier Information Service (SIS) and submits bids, the following procurement-portal fields map to the items above:

| Procurement field (typical) | Source | Sorellon answer today | Sorellon answer after remediation |
|---|---|---|---|
| Companies House number | §3.4 record | 16575318 | 16575318 |
| Legal entity name | §3.4 record | SORELLON LIMITED | SORELLON LIMITED |
| Registered office | §3.4 record | Avantgarde Tower, Apartment 144, London E1 6GS | Same |
| Trading address (if different) | Operator | None | Optional |
| Corporation Tax UTR | §3.3 letter | [from HMRC letter] | Same — store in dossier |
| VAT number | §3.1 | Not registered | GB-prefixed 9-digit number once issued |
| ICO registration number | §3.6 | Not registered | ZA-prefixed number once paid |
| Modern slavery statement URL | §3.5 | None | sorellon.com/modern-slavery-statement + registry permalink |
| Number of employees | §3.2 | 0 or 1 (director) | Same |
| Most-recent annual turnover | §3.3 / accounts | £0 (pre-revenue) | Same until first sales |
| Directors all ID-verified (ECCTA) | §3.10 | Partial (TBD) | Yes — both verified, dates documented |
| GDPR / data-protection statement on file | §3.6/3.7/3.8 | None | Privacy notice + ROPA + DPAs available on request |
| Insurance certificates (Employers Liability, Public Liability, Product Liability) | Out of scope this section | TBD | Cover in separate research section |
| Cyber Essentials / Cyber Essentials Plus | Out of scope this section | None | Cover in separate research section |

> **Buyer behaviour for missing fields:** central-government buyers (e.g. Crown Commercial Service, DBT, NHS England) typically have soft tolerance for missing VAT numbers from genuine SMEs but **zero tolerance** for missing Companies House data, unverified directors, or "Confirmation statement overdue" status. Local authorities are more variable — some accept self-declared status for ICO/VAT, others require numeric proof at PQQ stage.

---

### Appendix C — Open Items / VERIFY Queue (operator action list)

Prioritised list of items the operator must independently verify before acting on this research:

1. **Dieter's identity verification deadline** — internal docs cite 14 Dec 2025. Confirm against Dieter's actual passport DOB and ECCTA "first 14 days of birth month" rule. If deadline has lapsed: complete verification immediately and consider voluntary disclosure to Companies House.
2. **Zac's IV deadline** — confirmed as 2026-07-23 by alignment with first confirmation statement, but check Companies House public record for the actual confirmation-statement date for #16575318.
3. **Sorellon's UTR location** — was the HMRC letter received at Avantgarde Tower? If not, request reissue via 0300 200 3410.
4. **Sorellon's "trading start" date for HMRC active-status purposes** — was the 3-month window opened by sorellon.com publication / first RFQ? If >3 months elapsed, file active-status notification immediately.
5. **Modern Slavery Act §54 threshold** — Home Office may reform during 2026; track legislation.gov.uk and Home Office news.
6. **UK adequacy decision** — current to ~2029 per the 2025 renewal; track ICO international-transfers page for any change.
7. **Single-director Employment Allowance exclusion** — confirm still in force for 2026/27 (still excluded as of recent guidance).
8. **2026/27 NI Lower Earnings Limit, Primary Threshold, Secondary Threshold, employer NI rate** — confirm with Spring Budget 2026 outcome before designing any director-salary plan.
9. **First accounting reference date for Sorellon at Companies House** — verify against the public record. Default rules suggest a 21-month first period to 31 Mar 2027 with accounts due 31 Dec 2027; internal docs say accounts due 2027-04-10 which may indicate a shortened first period — verify.
10. **Sorellon's share-register split between Zac and Dieter** — must match the PSC register entries (e.g. if Zac holds 60 shares and Dieter 40 of 100 issued, the PSC entries should reflect "more than 50% but not more than 75%" and "more than 25% but not more than 50%" respectively).
11. **Existence and quality of any pre-revenue evidence pack** for the HMRC VAT application — operator to assemble before applying.
12. **Domain consistency audit** — review every internal Sorellon document for the historic "sorelon" single-L typo and correct to "sorellon" before any external filing references them.

---

### Sources

Primary regulatory sources:
- HMRC — VAT registration: https://www.gov.uk/register-for-vat
- HMRC — PAYE registration: https://www.gov.uk/register-employer
- HMRC — Corporation Tax: https://www.gov.uk/corporation-tax
- Companies House — Fee changes Feb 2026: https://www.gov.uk/government/news/companies-house-fees-are-changing-from-1-february-2026
- Companies House — Confirmation Statement: https://www.gov.uk/guidance/confirmation-statement-guidance
- Companies House — Identity Verification: https://www.gov.uk/guidance/verify-your-identity-for-companies-house
- Companies House blog — PSC IV: https://companieshouse.blog.gov.uk/2026/01/16/understanding-identity-verification-for-people-with-significant-control-pscs/
- ICO — Data protection fee: https://ico.org.uk/for-organisations/data-protection-fee/data-protection-fee/
- ICO — Data protection fee self-assessment: https://ico.org.uk/for-organisations/data-protection-fee/data-protection-fee-self-assessment/
- ICO — Article 30 documentation guidance: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/accountability-and-governance/documentation/who-needs-to-document-their-processing-activities/
- ICO — Contracts (Article 28): https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/accountability-and-governance/contracts-and-liabilities-between-controllers-and-processors-multi/what-needs-to-be-included-in-the-contract/
- ICO — Penalty notices for non-payment: https://ico.org.uk/about-the-ico/our-information/penalty-notices-issued-for-non-payment-of-data-protection-fee/
- Modern Slavery Statement Registry: https://modern-slavery-statement-registry.service.gov.uk/
- Gov.uk — Publish a modern slavery statement: https://www.gov.uk/guidance/publish-an-annual-modern-slavery-statement
- Gov.uk — PPN 009 Tackling modern slavery in supply chains: https://www.gov.uk/government/publications/ppn-009-tackling-modern-slavery-in-government-supply-chains/ppn-009-guidance-on-tackling-modern-slavery-in-government-supply-chains-html
- Home Office — TISC statutory guidance (March 2025): https://assets.publishing.service.gov.uk/media/68873308cec9ccd515ae09b0/Transparency_in_supply_chains_a_practical_guide.pdf
- Modern Slavery Act 2015 §54: https://www.legislation.gov.uk/ukpga/2015/30/section/54
- Gov.uk — Procurement Act 2023 supplier short guide: https://www.gov.uk/government/publications/procurement-act-2023-short-guides/the-procurement-act-2023-a-short-guide-for-suppliers-html
- GDPR-info — Article 27: https://gdpr-info.eu/art-27-gdpr/
- Google — Cloud Data Processing Addendum: https://cloud.google.com/terms/data-processing-addendum

Supporting commentary / cross-checks:
- THP Chartered Accountants — VAT threshold 2026: https://www.thp.co.uk/uk-vat-threshold-2026-a-complete-guide-for-growing-businesses/
- MBridge Technologies — VAT threshold 2026/27: https://mbridgetechnologies.cloud/blog/vat-threshold-increase-april-2026
- Acuity Law — ICO fee rise Feb 2025: https://acuitylaw.com/rise-in-ico-fees-data-protection-fees-february/
- nCino Identity Solutions — ECCTA IV costs: https://www.identitysolutions.ncino.com/blog/costs-for-identity-verification-of-shareholders
- LocalBusinessMagazine — IV rollout 2026 deadlines: https://www.localbusinessmagazine.co.uk/companies-house-identity-verification-rollout/
- CMS Law — Section 54 reform watch: https://cms.law/en/gbr/legal-updates/section-54-modern-slavery-act-where-do-we-go-from-here
- Latham & Watkins — TISC March 2025 update: https://www.lw.com/en/insights/uk-government-issues-updated-modern-slavery-act-transparency-guidance-key-takeaways-for-businesses
- Quality Company Formations — UTR / PAYE / Confirmation Statement guides: https://www.qualitycompanyformations.co.uk/
- MTD.digital — MTD VAT 2026 complete guide: https://mtd.digital/mtd-income-tax/mtd-vat-complete-guide/
- Starling Bank — Voluntary VAT registration benefits: https://www.starlingbank.com/resources/business-guides/benefits-of-being-vat-registered/

---

## 11. Insurance deep-dive

> **Context.** Sorellon Limited (Companies House #16575318, incorporated 2025-07-10) is a pre-revenue B2B wholesale supplier of PPE, hygiene wipes (EN1276/EN14476 disinfecting), healthcare consumables (gloves, syringes, dressings, catheters), cleaning chemicals, janitorial supplies, painting kits and stationery. Sole director Zac Farrell; co-PSC Dieter Xia Shuang Danicek. Registered office is a residential apartment in London E1. No employees beyond the director; no premises beyond the registered office; no trading history; first accounts due 2027-04-10. Imports from UK/EU/China; delivers across UK including multi-site. This profile is unusually risky-looking to underwriters in three ways at once: (i) zero trading history, (ii) goods category includes regulated chemicals and medical-adjacent consumables (high product-liability severity), and (iii) registered office is a residential apartment (some insurers' rules flag this for premises survey or refuse). Every item below is calibrated to this profile.
>
> **Sources used throughout.** HSE official guidance (hse.gov.uk), GOV.UK, ABI guidance summaries, Crown Commercial Service / NHS Supply Chain framework agreement extracts, Hellios JOSCAR materials, and broker-side disclosures from Simply Business, Hiscox, AXA UK, Markel Direct, Direct Line for Business, Premierline, Towergate, Insure24, accfirm, Just Quote Me, Connection Technologies (cyber), and NimbleFins (where indexed). [VERIFY: all premium ranges are indicative starting prices; bind-stage premiums for Sorellon will be higher than the headline "from" figures because of the pre-revenue + chemicals + residential-office combination.]

---

### 11.1. Employers' Liability Insurance (EL)

- **Field — What it is.**
  - Statutory employer-paid insurance against claims by employees (and certain workers) for injury or disease sustained in the course of employment.
  - Mandated by the Employers' Liability (Compulsory Insurance) Act 1969 and supporting regulations (the Employers' Liability (Compulsory Insurance) Regulations 1998, as amended).
  - Cover is on an each-occurrence basis with a £5,000,000 statutory minimum per claim.
  - [Source: https://www.legislation.gov.uk/ukpga/1969/57] [Source: https://www.hse.gov.uk/pubns/hse40.htm]
- **Field — Mandatory.**
  - Yes by default — but with a narrow exemption that **applies to Sorellon today**.
  - The HSE confirms a limited company is **not required** to hold EL insurance if the only employee is also a person who owns 50% or more of the issued share capital. [Source: https://www.hse.gov.uk/pubns/hse40.htm] [Source: https://www.hiscox.co.uk/business-blog/employers-liability-act] [Source: https://www.kingsbridge.co.uk/blog/contractors/contractor-guides/employers-liability-requirements-and-exemptions/]
  - The Hiscox commentary and Kingsbridge contractor guide both summarise this as the "sole-director-owning-50%-plus" carve-out.
  - Companies House register for #16575318 will determine whether Zac Farrell (sole director) holds ≥50% of the shares; the PSC filing names Dieter Xia Shuang Danicek as a co-PSC, so the share split must be checked before relying on the exemption.
  - [VERIFY: actual shareholding split — if Zac Farrell holds <50%, Sorellon needs EL insurance from the moment he is treated as an employee, even with no other staff.]
  - Other family-member and unincorporated-business exemptions do not help Sorellon because it is a limited company. [Source: https://www.hse.gov.uk/pubns/hse40.htm]
- **Field — Eligibility (to buy).**
  - Universally available to UK Ltds from every major direct insurer (AXA, Aviva, Zurich, RSA, NIG, Allianz, Ageas) and from all the brokers and platforms (Simply Business, Hiscox, Markel Direct, Direct Line for Business, Premierline, Towergate, Superscript).
  - Pre-revenue status is not a barrier for EL specifically — underwriters rate it on payroll/wageroll, not turnover.
  - With zero payroll Sorellon is in the lowest possible band.
  - [Source: https://www.simplybusiness.co.uk/business-insurance/limited-company-insurance/]
- **Field — Cost.**
  - Statutory minimum cover is £5M; the market default is £10M and that is what most direct quote engines bind by default.
  - Indicative annual premium for a Ltd with one employee (director) in a low-injury-risk wholesale-distribution trade: **£60–£180 per year stand-alone**.
  - Or essentially zero marginal cost when bundled inside a combined business policy (most insurers throw EL in for £10–£30 of premium when it sits next to public liability).
  - [Source: https://www.simplybusiness.co.uk/business-insurance/limited-company-insurance/]
  - [VERIFY: bundle pricing depends on activity selection — "wholesale of chemicals" raises the combined premium and the EL component does not move much.]
- **Field — Timeline.**
  - Same-day. Quote-to-bind in 7–15 minutes via Simply Business, Direct Line for Business, AXA Business or Hiscox Direct.
  - Documents emailed instantly on payment.
  - [Source: https://www.simplybusiness.co.uk/business-insurance/wholesale-insurance/] [Source: https://help.simplybusiness.co.uk/hc/en-gb/articles/12666795779357-Give-us-a-call]
- **Field — Issuer.**
  - Capacity is provided by FCA-authorised UK insurers and Lloyd's syndicates: AXA Insurance UK plc, Aviva Insurance Ltd, RSA, Allianz, Zurich, NIG, Ageas, Hiscox Insurance Company Ltd.
  - Brokered (Simply Business, Premierline, Towergate, Markel) versus direct (AXA Business, Direct Line for Business, Hiscox Direct) routes both reach this same capacity pool.
- **Field — Application.**
  - Underwriter asks: trade description, payroll/wageroll estimate, number of employees, subcontractor spend, business address, claims history (5 years), director details.
  - For Sorellon the answers are trivially small numbers — but the trade-description code matters.
  - Choosing "wholesale of chemicals" or "PPE / healthcare consumables wholesale" rather than "general wholesale" is the honest answer and changes the rest of the bundle's pricing.
  - [Source: https://www.hellopolaris.co.uk/blog/what-happens-when-declared-turnover-no-longer-matches-reality/]
  - [VERIFY: each platform's exact activity dropdown.]
- **Field — Prerequisites.**
  - None. No certifications, no audited accounts, no premises survey for EL alone.
  - A business bank account is not technically required to buy a policy (though it is required to pay by direct debit).
- **Field — Renewal.**
  - Annual. Reconfirm payroll, claims, headcount, activity.
  - If Sorellon hires any non-director employee mid-term, EL becomes legally mandatory and the policy must be updated immediately.
  - [Source: https://www.hse.gov.uk/pubns/hse40.htm]
- **Field — Ongoing.**
  - **Certificate display** is the single most under-appreciated obligation.
  - The certificate must be displayed where employees can read it (paper at premises) or electronically (intranet/email) with reasonable access.
  - Failure to display, or refusal to produce on demand to an HSE inspector, attracts up to £1,000 fine.
  - Operating without required EL cover attracts up to £2,500 **per day**.
  - [Source: https://www.hse.gov.uk/pubns/hse40.htm] [Source: https://www.kingsbridge.co.uk/blog/contractors/contractor-guides/employers-liability-requirements-and-exemptions/]
  - Certificates must be retained for **at least 40 years** after expiry under the 1998 regulations (long-tail disease claims).
  - [VERIFY: 40-year retention is the post-2008 simplified requirement — pre-2008 cover may have different retention rules but is not relevant to Sorellon.]
- **Field — Risk if absent.**
  - (1) HSE prosecution and per-day fines if Sorellon hires anyone outside the director and skips cover.
  - (2) Personal injury claims by an injured worker against an uninsured employer become company debts and can be enforced against directors via wrongful trading or piercing the corporate veil in egregious cases.
  - (3) Public-sector framework agreements (NHS Supply Chain, CCS) **require** EL of at least £5M per claim with no exemption for "no employees" — Sorellon will be asked for a certificate during onboarding and "we're exempt" will not satisfy the framework's evidence requirement.
  - [Source: https://www.sccl.nhs.uk/wp-content/uploads/2024/10/VWS_Framework-Agreement.docx_Redacted.pdf]
  - **This is the operative point:** Sorellon should buy EL even though it can technically claim exemption, because every framework will demand a certificate.
- **Field — Sequencing.**
  - Buy EL inside a combined business policy at the same moment as Public Liability and Product Liability.
  - There is no procurement value in buying it earlier; there is procurement value in buying it before the first NHS Supply Chain / CCS DPS application.
- **Field — Sorellon-specific.**
  - **Recommendation: buy EL at the £10M level inside a combined policy, regardless of exemption.**
  - Cost is negligible (£10–£30/year inside a bundle).
  - Upside is that every public-sector tender, NHS Supply Chain MIA registration, and many private B2B clients (NHS Trusts, large facilities-management companies) ask for the certificate as a binary tick.
  - Saying "we are exempt because our sole director owns 100% of shares" will not satisfy a procurement portal that has a mandatory upload field.
  - [Source: https://www.sccl.nhs.uk/wp-content/uploads/2024/10/VWS_Framework-Agreement.docx_Redacted.pdf]

---

### 11.2. Public Liability Insurance (PL)

- **Field — What it is.**
  - Indemnity against third-party injury, illness or property damage caused by Sorellon's business activities.
  - Example claims: a courier dropping a pallet on a customer's foot, a leaking container on customer premises, a slip on Sorellon's residential office should anyone visit.
  - Not statutory — but commercially indispensable for any B2B supplier.
- **Field — Mandatory.**
  - No statutory UK requirement. [Source: https://www.tradedirectinsurance.co.uk/insurance/public-liability/faq/is-it-law-to-have-public-liability-insurance/]
  - Contractually mandatory under essentially every public-sector framework Sorellon will target.
  - Local authority and NHS contracts typically require **£5M minimum**, often defaulting to £10M.
  - CCS frameworks (RM6xxx series) require Good Industry Practice plus a stated minimum that is usually £5M.
  - MoD/JOSCAR-touching contracts require £10M and frequently £25M for safety-critical items.
  - [Source: https://www.insure24.co.uk/blog/insurance-needed-for-government-and-public-sector-contracts/] [Source: https://www.livingstonesinsurance.co.uk/blog/public-liability-insurance-limits-uk-how-much-do-you-need/]
- **Field — Eligibility.**
  - Available to all UK Ltds. Pre-revenue status is not a barrier.
  - The activity description, however, is — "wholesale of chemicals" or "wholesale of PPE / healthcare consumables" attracts a higher base rate than "general wholesale" or "office supplies wholesale".
- **Field — Cost.**
  - Headline cover bands and indicative annual premiums for a small/new UK Ltd in low-risk distribution (no construction, no installation, no on-site work):
  - £1M cover: £80–£200/yr (low-risk trades around £50–£120). [Source: https://accfirm.co.uk/public-liability-insurance-uk/]
  - £2M cover: £130–£300/yr (upgrade from £1M typically adds £50–£100). [Source: https://accfirm.co.uk/public-liability-insurance-uk/]
  - £5M cover: £200–£500/yr (local authority and event tier).
  - £10M cover: £350–£900/yr (government/infra tier, MoD-touching contracts).
  - £20M+ cover: £1,500+/yr (high-risk construction, infrastructure) — not applicable to Sorellon.
  - Hiscox Direct headlines "Public Liability from £5.76/month" (~£70/yr) but that is the **lowest 10%** of policies; a chemical/PPE wholesaler will not see that price.
  - Realistic Sorellon bind cost at £5M PL inside a combined policy: **£300–£600/year**.
  - [Source: https://www.simplybusiness.co.uk/business-insurance/public-liability-insurance/] [Source: https://www.hiscox.co.uk/business-insurance/public-liability-insurance] [Source: https://www.markeluk.com/business-insurance/public-liability-insurance]
  - [VERIFY: real quote required — chemicals/PPE classification will push Sorellon above the median.]
- **Field — Timeline.**
  - Same-day for £1M–£5M cover via direct online providers.
  - £10M and especially £10M-with-products-liability-stacked typically requires a brief broker-handled call (1–3 business days) because most direct engines cap at £5M for new businesses in chemical-adjacent trades.
  - [Source: https://www.simplybusiness.co.uk/business-insurance/wholesale-insurance/]
  - [VERIFY: Hiscox Direct quotes up to £10M online for many trades; Markel Direct currently caps at £5M.] [Source: https://www.markeluk.com/business-insurance/public-liability-insurance]
- **Field — Issuer.**
  - Same insurer pool as EL (AXA, Aviva, Allianz, Zurich, RSA, NIG, Ageas, Hiscox, Markel International / Lloyd's syndicates).
- **Field — Application.**
  - Trade description and activities.
  - Estimated turnover (the killer question for Sorellon — see §5.10).
  - Territories of operation.
  - Premises (issue: residential apartment).
  - Employee count, subcontractor use.
  - Claims history (5 years).
  - Products supplied list.
  - Whether any work done on customer premises.
  - Depots/warehouses used.
- **Field — Prerequisites.**
  - None other than honest answers.
  - For higher cover bands (£10M+) underwriters increasingly want:
    - (a) a real (non-residential) premises, or evidence that no business operations happen at the residential office.
    - (b) a turnover projection that justifies the cover.
    - (c) evidence of supplier QA / certificates for PPE and chemicals (EN1276, EN14476, CE / UKCA marking, MHRA registration where relevant).
    - (d) sometimes a director's CV in trade.
  - [Source: https://www.insure24.co.uk/blog/product-liability-insurance-for-chemical-products-uk-a-practical-guide/]
- **Field — Renewal.**
  - Annual.
  - Mid-term notification required if Sorellon takes on a non-trivial new product category (e.g., starts supplying sharps / syringes when policy was bound for "PPE and cleaning chemicals only").
  - Material misstatement at renewal voids cover.
  - [Source: https://www.hellopolaris.co.uk/blog/what-happens-when-declared-turnover-no-longer-matches-reality/]
- **Field — Ongoing.**
  - Keep certificate accessible.
  - Many procurement portals (NHS Jaggaer, Bravo, Atamis; Pro-Contract; Find a Tender) ask for it as a PDF upload.
  - Track expiry — set a 60-day-before-expiry calendar reminder; gaps in cover will fail framework re-checks instantly.
- **Field — Risk if absent.**
  - (1) A single slip-and-fall at a delivered-to customer site can be a £50k–£500k claim; without PL it lands on Sorellon's balance sheet.
  - (2) Inability to win any public-sector work — every framework asks for it.
  - (3) Inability to win most private B2B work — large customers' procurement / vendor-management portals (Coupa, SAP Ariba, Achilles) require an uploaded certificate.
  - (4) Reputation: brokers will tell prospects that "uninsured supplier" is an automatic disqualifier.
- **Field — Sequencing.**
  - Bind PL at the same moment as EL and Product Liability — they are sold as a combined policy by every UK SME insurer.
  - Do not buy stand-alone PL then "add" Product Liability later; the activity questionnaire is identical and the marginal cost is small.
- **Field — Sorellon-specific.**
  - **Recommendation: bind £10M PL inside a combined policy** (not £5M — the marginal cost £100–£300/yr is small vs. tender exclusion risk).
  - Watch two underwriting pitfalls:
    - (a) The residential-apartment registered office (Avantgarde Tower, E1 6GS) — declare that no goods are stored at the apartment and that the office is administrative only; some insurers (Aviva, RSA) will otherwise insist on a premises survey or refuse stock cover.
    - (b) The activity code — pick the **specific** chemical/PPE wholesale code, not "general wholesale (46900)" which the SIC says but the insurer rating engines treat as a catch-all that may not respond to a chemical injury claim.
  - [VERIFY: each platform's exact activity dropdown — the SIC 46900 maps to several insurer activities; choose by what is actually sold.]

---

### 11.3. Product Liability Insurance (PrL)

- **Field — What it is.**
  - Indemnity against claims that a product supplied by Sorellon caused injury, illness, or property damage to a third party.
  - Applies even if the product was made by someone else and Sorellon merely distributed it.
  - Under the Consumer Protection Act 1987 a distributor can be jointly and severally liable with the manufacturer where the manufacturer cannot be identified or is outside the UK/EU.
  - **This is the most material insurance risk for Sorellon.**
  - [Source: https://www.simplybusiness.co.uk/business-insurance/product-liability-insurance/] [Source: https://www.insure24.co.uk/blog/product-liability-insurance-for-chemical-products-uk-a-practical-guide/]
- **Field — Mandatory.**
  - Not statutory.
  - But contractually required by every NHS Supply Chain framework agreement (£5M minimum per claim), every CCS framework, every MoD/JOSCAR-touching contract (typically £10M), and every reputable private B2B buyer who handles risk via a vendor portal.
  - NHS Supply Chain explicitly: "the Supplier shall maintain insurance in respect of employer's liability, public liability (including pure economic loss), and product liability with the minimum cover per claim being the greater of £5,000,000 or any sum required by Law".
  - [Source: https://www.sccl.nhs.uk/wp-content/uploads/2023/10/Appendix-4-NHS-Supply-Chain-Framework-Agreement-for-the-Supply-of-Goods.pdf]
- **Field — Eligibility.**
  - Available to UK Ltds via every combined-policy insurer.
  - **Sorellon's category is non-trivial.**
  - Disinfectants (EN1276/EN14476), gloves, syringes, dressings and catheters are all categories where underwriters apply loadings or refer to a senior underwriter rather than auto-bind.
  - Some direct online engines (e.g. Markel Direct, Direct Line for Business) will decline to auto-quote and will refer to a phone underwriter, particularly when the questionnaire mentions "imports from China".
  - [Source: https://www.compliancegate.com/united-kingdom-product-liability-insurance/]
- **Field — Cost.**
  - Headline starting prices are misleading for Sorellon's category:
  - Hiscox Direct: PL + PrL combined "from £5.20/month" (~£62/yr) — only the lowest decile. [Source: https://www.hiscox.co.uk/business-insurance/product-liability-insurance]
  - Simply Business: PrL "from £5.76/month" — same caveat. [Source: https://www.simplybusiness.co.uk/business-insurance/product-liability-insurance/]
  - AXA UK: from "£6 for 10 months after deposit" per Q1 2026 (10th percentile). [Source: https://www.axa.co.uk/business-insurance/public-liability/product-liability-insurance/]
  - Realistic Sorellon premiums for the **chemical/PPE/medical-consumable wholesale** profile at first-year pre-revenue, bundled with PL + EL inside a combined policy:
    - £5M PL/PrL: **£600–£1,500/year** for a low/projected turnover. [VERIFY: bind quotes from 2+ insurers required.]
    - £10M PL/PrL: **£1,200–£3,000/year** — required for NHS Supply Chain, MoD/JOSCAR, and most large LA cleaning/PPE contracts.
    - With imports-from-China loading: add 10–25%.
    - With Sorellon as a "first-supplier-into-UK" of products from Chinese manufacturers: another 10–20% loading.
  - Why the loading: under the Consumer Protection Act the importer is treated as the producer if the actual producer cannot be sued in the UK.
  - [Source: https://www.compliancegate.com/united-kingdom-product-liability-insurance/]
- **Field — Timeline.**
  - Online direct quote engines may auto-quote £1M–£5M for PPE / cleaning chemicals immediately.
  - £10M and the inclusion of disinfectants and medical consumables (syringes, catheters, dressings) **will route to a referred underwriter** at most direct providers.
  - Expect a 1–5 business-day broker / underwriter conversation before bind.
  - [Source: https://www.simplybusiness.co.uk/business-insurance/product-liability-insurance/]
- **Field — Issuer.**
  - Same major insurers as PL/EL.
  - For higher limits or trickier categories (syringes, catheters) the capacity often comes from Lloyd's syndicates via brokers: Markel International, Hiscox, AmTrust, Travelers, AIG, CNA Hardy.
  - [Source: https://www.compliancegate.com/united-kingdom-product-liability-insurance/]
- **Field — Application.**
  - Underwriter questions for chemical/PPE/medical-consumable wholesale (per Insure24 broker guide and consistent with AXA/Hiscox/Markel questionnaires):
  - What products are sold, by category and unit volume?
  - Manufacturer country of origin (UK / EU / China / other)?
  - Does Sorellon hold UK Responsible Person status for any product (e.g. PPE under PPE Regs, medical devices under UK MDR)?
  - CE / UKCA / EN1276 / EN14476 / BS EN 374 certificates held by manufacturers?
  - Quality control / batch traceability process?
  - Customer mix: trade, retail, consumer, industrial, public sector?
  - Territory of supply (UK / EU / worldwide)?
  - Annual turnover projection split by product category and territory?
  - Labelling control: is Sorellon re-labelling, repackaging, or selling in original manufacturer packaging?
  - Safety Data Sheets (SDS) held for chemical products?
  - Complaint handling / recall procedures?
  - [Source: https://www.insure24.co.uk/blog/product-liability-insurance-for-chemical-products-uk-a-practical-guide/]
- **Field — Prerequisites.**
  - Honest answers.
  - Ideally supplier QA documents on file:
    - certificates of conformity from manufacturers
    - EN1276/EN14476 test reports for any disinfectant SKU
    - BS EN 374 for chemical-resistant gloves
    - UK Responsible Person agreements for imported PPE
  - Sorellon is at the disadvantage that pre-revenue means no claims history (good) but also no track-record evidence of supplier QA discipline (a soft negative the underwriter will discount or load).
- **Field — Renewal.**
  - Annual.
  - Reconfirm turnover, products, claims, suppliers.
  - **Critical:** material expansion into a new product category (e.g. starting to supply syringes / sharps when policy was bound for "PPE and cleaning chemicals only") must be notified mid-term.
  - Failure to notify can void the policy.
- **Field — Ongoing.**
  - Keep supplier-side QA documentation organised — when a claim is made the insurer will subrogate against the upstream manufacturer, and Sorellon's ability to identify and prove the chain matters for retained excess and renewal pricing.
  - Maintain SDS / batch records for 11 years (statutory long-tail period for product claims under the Limitation Act).
  - [VERIFY: 11-year longstop on product claims under Consumer Protection Act 1987 s.11A.]
- **Field — Risk if absent.**
  - A single bad batch of disinfectant that fails its EN claim and causes a healthcare-associated infection outbreak in an NHS Trust is realistically a £1M–£10M claim.
  - A single contaminated syringe is potentially a personal-injury claim per affected patient.
  - PPE that fails (e.g. gloves leak in a clinical setting causing a sharps injury) sits squarely in Product Liability territory.
  - Without PrL these are uninsured corporate debts; with PrL but below the cover limit, Sorellon's excess sits on the balance sheet.
- **Field — Sequencing.**
  - Bind PrL at the same time as PL/EL inside a combined policy.
  - Add **Product Recall / Withdrawal extension** as a deliberate add-on (not standard).
  - Standard PrL excludes the cost of the recall itself (logistics, communications, destruction); covers only third-party injury/damage.
  - [Source: https://www.insure24.co.uk/blog/product-recall-insurance-for-chemical-manufacturers-explained/]
- **Field — Sorellon-specific.**
  - **Recommendation: bind £10M PrL aggregate / £5M each-occurrence minimum, with explicit naming of the product categories Sorellon will actually sell, plus a Product Recall extension of £100k–£250k.**
  - Do not skimp on aggregate vs each-occurrence: underwriters now typically write PrL on an **aggregate-per-period** basis (a hard cap on the policy's total payouts for the year), and a recall affecting 50 NHS hospitals could exhaust a £5M aggregate in one event.
  - [VERIFY: per-occurrence vs aggregate wording in each quote.]
  - Specifically declare: imports from China, EN1276/EN14476 disinfecting wipes, healthcare consumables (gloves, syringes, dressings, catheters), and that Sorellon does **not** re-label or repackage.
  - Re-labelling materially increases premium because Sorellon becomes the "deemed producer" under the Consumer Protection Act 1987.

---

### 11.4. Professional Indemnity Insurance (PI)

- **Field — What it is.**
  - Indemnity against claims of financial loss caused by negligent advice, error or omission in a professional service.
  - Designed for consultants, accountants, designers, IT services, surveyors, brokers — not goods distributors.
- **Field — Mandatory.**
  - No. Not for goods supply.
  - The FCA mandates PI for certain regulated activities (financial advisers, mortgage brokers, insurance distributors) but none apply to Sorellon.
  - [Source: https://www.fca.org.uk/firms/professional-indemnity-insurance/distributors]
  - (Note: that FCA page is about insurance-product distributors, i.e. brokers selling insurance — unrelated to Sorellon's wholesale activity.)
- **Field — Eligibility.**
  - Universally available, from £6.62/month at Simply Business, but Sorellon does not need it.
  - [Source: https://www.simplybusiness.co.uk/business-insurance/professional-indemnity-insurance/]
- **Field — Cost.**
  - £200–£600/yr at £1M for a small consultancy.
  - Irrelevant for Sorellon unless it starts offering advisory services (e.g. infection-control consultancy alongside the products).
- **Field — Timeline.** N/A.
- **Field — Issuer.** Hiscox, Markel, AXA, AmTrust, Liberty, CFC, Hanover. Mostly broker-routed at higher limits.
- **Field — Application.** Trade, services described, fee income, contracts held, claims history.
- **Field — Prerequisites.** None for buying; standard cover.
- **Field — Renewal.** Annual.
- **Field — Ongoing.** N/A for Sorellon today.
- **Field — Risk if absent.**
  - Effectively none for a pure goods wholesaler.
  - **Caveat:** if Sorellon ever signs a procurement contract that includes "supplier shall provide advice on appropriate product selection / safe use / training" then PI becomes relevant — a recommendation to use the wrong disinfectant in a clinical setting could become a negligence claim.
  - The risk is contract-drafting-driven, not activity-driven.
- **Field — Sequencing.**
  - Do not buy now.
  - Re-evaluate if Sorellon enters infection-control advisory, equipment training, or any consultancy line.
- **Field — Sorellon-specific.**
  - **Recommendation: skip PI in v1.**
  - Add it (~£300/yr for £1M cover) only if Sorellon contracts include explicit advisory language.
  - Many large CCS / NHS contracts now include "advisory services" boilerplate even for goods supply — read every signed contract for the words "advise", "recommend", "specify" before relying on the no-PI position.
  - [VERIFY: review NHS Supply Chain framework clause set for advisory language at the point of MIA application.]

---

### 11.5. Cyber Insurance

- **Field — What it is.**
  - First-party + third-party cover for breach response (forensics, customer notification, credit monitoring, PR).
  - Business interruption from cyber incidents.
  - Ransomware payments (where lawful).
  - Liability to customers / data subjects.
  - Regulatory defence costs.
  - Distinguish: cyber **insurance** is not a substitute for ICO fines — insurers will pay defence costs but cannot indemnify the fine itself in most policies, and **GDPR fines are uninsurable in the UK as a matter of public policy**.
  - [Source: https://www.policybee.co.uk/blog/does-cyber-insurance-cover-gdpr] [Source: https://justquoteme.co.uk/cyber-liability-insurance-the-complete-2026-guide-for-uk-businesses/]
- **Field — Mandatory.**
  - Not statutory.
  - Increasingly required in public-sector contracts that handle personal data — but Sorellon as a goods wholesaler is unlikely to be handling citizen personal data.
  - **PPN 014** (in force since Feb 2025) makes Cyber Essentials (not cyber insurance) mandatory in central-government contracts handling personal/employee data or ICT systems at OFFICIAL — but it does not require cyber insurance.
  - [Source: https://www.gov.uk/government/publications/ppn-014-cyber-essentials-scheme/ppn-014-cyber-essentials-scheme-html] [Source: https://netsecgroup.io/guides/ppn-014-government-contracts-cyber-essentials]
- **Field — Eligibility.**
  - Widely available from Hiscox, AXA, Markel, Superscript, CFC, Beazley, Hannover, Tokio Marine HCC, via brokers or direct platforms.
  - Pre-revenue Ltds quote without difficulty at the SME tier.
- **Field — Cost.**
  - 2026 SME indicative annual premiums:
    - Micro-business (<£500k turnover, £250k–£1M cover): **£175–£500/yr**
    - Small business (£500k–£2M turnover, £1M cover): **£1,500–£4,000/yr**
    - SMB (£2M–£10M turnover, £2M cover): **£3,500–£9,000/yr**
    - Most SME premiums fall in **£350–£5,000/yr** range
  - Bundled into Cyber Essentials via IASME: **free £25,000 cyber liability** included with CE certification if turnover under £20M.
  - [Source: https://connection-technologies.co.uk/blog/cyber-insurance-uk-2026] [Source: https://justquoteme.co.uk/cyber-liability-insurance-the-complete-2026-guide-for-uk-businesses/] [Source: https://www.jmginsurance.co.uk/blog/how-much-does-cyber-liability-insurance-cost-in-the-uk/]
- **Field — Timeline.**
  - Same-day via Superscript or Hiscox Direct at £1M cover.
  - Higher limits or ransomware-inclusive cover may need a 1–3 day underwriter review.
- **Field — Issuer.**
  - CFC, Beazley, Hiscox, AIG, Travelers, Markel are the leading London-market capacity providers.
  - Bound via Simply Business, Superscript, brokers.
- **Field — Application.**
  - Annual turnover, business activities.
  - Number of records held.
  - MFA / endpoint protection / backup posture.
  - Prior incidents.
  - Dependency on third-party IT.
  - Customer notification obligations.
  - Payment-card processing volume.
- **Field — Prerequisites.**
  - Better quotes (and sometimes any quote at all) require:
    - MFA on all email and admin accounts.
    - Regular off-site backups.
    - EDR / antivirus.
    - Basic patch management.
  - Cyber Essentials certification typically gets a 5–10% premium discount and is a precondition for some capacity providers.
- **Field — Renewal.**
  - Annual.
  - Recent claims, security-control changes, turnover, records held — reconfirm.
- **Field — Ongoing.**
  - Sorellon should buy **Cyber Essentials** (£300+VAT for micro-tier) before cyber insurance.
  - The certification both gets the free £25k cyber cover and is a separate procurement gate (PPN 014).
  - [Source: https://connection-technologies.co.uk/blog/cyber-essentials-cost-uk-2026]
- **Field — Risk if absent.**
  - For a B2B wholesaler with no ecommerce, the residual risk is:
    - (a) Email-based business-email-compromise fraud diverting supplier or customer payments — a real and common £10k–£250k loss for SMEs.
    - (b) Ransomware locking the trading email, CRM and order systems.
    - (c) Customer-data breach if Sorellon holds any individual contact data (which it will — every B2B procurement officer's email and phone).
- **Field — Sequencing.**
  - Buy **Cyber Essentials** first (3–4 weeks to certify) — it ticks the PPN 014 box and includes free baseline cyber cover.
  - Then add a paid cyber insurance policy (~£300–£600/yr at £250k–£1M cover) only when Sorellon's revenue or customer-data scale justifies it.
  - Do not buy stand-alone expensive cyber before CE.
- **Field — Sorellon-specific.**
  - **Recommendation: prioritise Cyber Essentials over standalone cyber insurance for v1.**
  - Cost: £300+VAT for certification + free £25k cover.
  - Add a £250k–£500k cyber insurance policy (~£300–£600/yr) inside the combined business policy at renewal.
  - This sequence satisfies PPN 014 and gives baseline financial cover without spending £1.5k+ on a standalone policy before there's any data to protect.

---

### 11.6. Combined business insurance package (BBB-style)

- **Field — What it is.**
  - A single policy combining EL + PL + PrL + buildings/contents/stock + business interruption + money + legal expenses + (sometimes) cyber.
  - Sold under names like "Business Combined", "Shopkeepers/Wholesalers Combined", "BBB" (Business Block Buster).
  - Marketed by:
    - Hiscox ("Office Insurance" / "Business Insurance")
    - AXA Business
    - Direct Line for Business
    - Markel Direct
    - Simply Business (as a multi-cover quote)
    - Premierline (broker)
    - Towergate (broker)
  - [Source: https://www.axaconnect.co.uk/commercial-lines/branch-traded/business-combined/] [Source: https://compareyourbusinesscosts.co.uk/combined-business-insurance]
- **Field — Mandatory.**
  - No.
  - But every supplier-level requirement at NHS Supply Chain, CCS frameworks, MoD/JOSCAR is more easily evidenced via one combined policy schedule than five separate certificates.
- **Field — Eligibility.**
  - Available to UK Ltds.
  - Pre-revenue companies are eligible.
  - Direct online engines may auto-quote up to certain combinations.
  - Chemical / medical-consumable trades typically refer to a phone underwriter.
- **Field — Cost.**
  - Indicative all-in for Sorellon profile (pre-revenue UK Ltd, projected first-year turnover £100k–£500k, wholesale of PPE / chemicals / healthcare consumables / janitorial, residential reg. office only, sole director, no warehouse):
    - Bare-bones: EL £10M + PL £5M + PrL £5M (no contents/BI/cyber): **£500–£1,200/yr**
    - Mid-tier: above + PL/PrL £10M + £25k stock cover declared + £10k contents + business interruption: **£900–£2,500/yr**
    - Full-fat: above + cyber £250k + legal expenses + product recall £100k: **£1,500–£4,000/yr**
  - [VERIFY: real quotes required from at least 2 of {Simply Business, Hiscox, AXA Business, Markel Direct} and 1 broker {Premierline / Towergate}.]
- **Field — Timeline.**
  - Direct online: 15–45 minutes to quote, same-day bind for the bare-bones combination.
  - Broker-led full-fat: 2–5 business days because chemical/medical-consumable trades will be referred to underwriters at the £10M PrL tier.
- **Field — Issuer.**
  - Same insurer pool.
  - Broker-led packages often write through Aviva, AXA, Allianz, NIG, RSA, Zurich.
  - Direct platforms through their captive/preferred panels.
- **Field — Application.**
  - A single questionnaire covering all sections — the trade and activity questions are answered once and rate all sections.
  - Underwriter wants: trade, activities, turnover projection, employee count, premises (residential office issue), claims, products, supplier countries, certifications held.
- **Field — Prerequisites.**
  - Same as the most-onerous component (PrL).
  - If Sorellon can pass PrL underwriting it can usually bind a combined.
- **Field — Renewal.**
  - Annual single renewal — operationally simpler than maintaining five policies.
- **Field — Ongoing.**
  - Single schedule, single certificate to upload to procurement portals — operationally large win.
- **Field — Risk if absent.**
  - No risk in not buying *combined* specifically (Sorellon can buy components separately).
  - But the combined route is materially cheaper than the sum of stand-alone covers, often 20–35% saving.
- **Field — Sequencing.**
  - Bind the combined policy as the first insurance purchase, after Cyber Essentials and the bank account.
- **Field — Sorellon-specific.**
  - **Recommendation: bind a combined business policy with £10M EL, £5M (preferably £10M) PL, £10M PrL aggregate, £25k stock, £10k contents (administrative office only), £100k–£250k product recall extension, optional £250k cyber.**
  - **Target premium: £1,000–£2,500/yr.**
  - Buy from a broker (Premierline or Towergate) rather than direct, because:
    - The chemical/medical-consumable activity descriptions will trigger referred underwriting at every direct engine anyway.
    - A broker can shop the same risk across 5+ insurers in one call.

---

### 11.7. Direct online quote providers — who will underwrite Sorellon?

A pre-revenue Ltd with chemicals and medical-adjacent consumables is **on the edge** of direct online quote engines' auto-bind appetite. The following is the practical reality for each major direct route.

**Simply Business** — https://www.simplybusiness.co.uk
- Phone: 0333 014 6683 (Mon–Fri 9–5:30, Sat 9–2).
- Acts as broker; panel includes AXA, Aviva, Hiscox, RSA, NIG, Markel, Ageas, Liberty.
- Will quote pre-revenue Ltds online.
- Wholesale-of-chemicals and PPE activities are in their dropdown.
- **Likely outcome for Sorellon:** auto-quote for £1M–£5M PL/PrL bundle inside a wholesale insurance policy; £10M PrL will probably refer to a human underwriter (1–2 business days).
- Premium expected in the £800–£2,000/year range.
- [Source: https://www.simplybusiness.co.uk/business-insurance/wholesale-insurance/] [Source: https://www.simplybusiness.co.uk/business-insurance/limited-company-insurance/]

**Direct Line for Business** — https://www.directlineforbusiness.co.uk
- Underwrites in-house.
- Will quote pre-revenue Ltds.
- **Likely outcome:** auto-quote up to £5M; chemicals + medical consumables may refer or decline at £10M.
- Often the strongest price for low-risk SME trades; less competitive on chemicals.

**AXA Business** — https://www.axa.co.uk/business-insurance/
- Major direct insurer for SMEs.
- AXA appears on Simply Business's panel for the same trade.
- **Likely outcome:** auto-quote for the standard PL/PrL/EL combo; chemicals + China imports + residential office may push to a phone underwriter at the bind stage.
- AXA's combined product is robust.
- [Source: https://www.axa.co.uk/business-insurance/]

**Hiscox Direct** — https://www.hiscox.co.uk/business-insurance
- Headline "from £5.20/month" PrL.
- Hiscox has a startup line of business and explicitly markets to new businesses. [Source: https://www.hiscox.co.uk/business-insurance/start-up-businesses]
- **Likely outcome:** Hiscox Direct caps several trades at £5M, references higher-limit needs to its broker channel.
- Premiums on the higher end for chemicals/medical consumables but the broker-channel option works for £10M.

**Markel Direct** — https://www.markeluk.com
- Direct SME platform from a Lloyd's insurer.
- PL/PrL cover £1M–£5M auto-quote in 90 seconds for many trades. [Source: https://www.markeluk.com/business-insurance/public-liability-insurance]
- **Likely outcome:** chemical/medical wholesale may not appear in the direct dropdown — Sorellon may be routed to Markel UK's broker channel (uk.markel.com).
- [Source: https://uk.markel.com/insurance]

**Superscript** — https://gosuperscript.com
- Modern subscription-style SME broker.
- Strong on tech and startups; cyber included naturally.
- Pre-revenue underwriting is explicitly supported.
- [Source: https://gosuperscript.com/business-insurance/business-type/startup-insurance/]

**Questions every direct engine will ask Sorellon at quote-time:**

1. Limited company / sole trader? (Limited.)
2. Business activity / trade description? (**Critical — pick "wholesale of chemicals" or "PPE / healthcare consumables wholesale", not "general wholesale"**.)
3. Estimated annual turnover for the next 12 months? (**Critical — Sorellon must declare an honest projection; £0 may auto-decline at some platforms because they cannot rate, so use £50k–£250k as a conservative projection.**)
4. Number of employees including directors? (1.)
5. Trading address vs registered address? (Both = Avantgarde Tower, E1 6GS — **flag**: most engines will ask "is this a residential address?" — answer honestly; some will decline contents/stock cover at a residential address but will still write liability cover.)
6. Years trading? (0.)
7. Claims history last 5 years? (None.)
8. Cover required (£1M / £2M / £5M / £10M)? PL and PrL separately.
9. Territory of operation (UK / EU / worldwide)?
10. Does the business import goods from outside the UK? (**Yes — declare China imports**.)

**Quote-to-bind timeline (direct):**
- 7–45 minutes if auto-quoted.
- 1–5 business days if referred (chemicals, £10M+, China imports, residential office combination is likely to refer).

---

### 11.8. Specialty brokers for new businesses / wholesale distributors

For Sorellon's profile the broker channel is **likely to outperform direct online** on price, cover and speed-of-bind, because brokers:
- Shop the same risk across 5+ insurers.
- Present a packaged narrative ("UK Responsible Person discipline, supplier QA framework, conservative projections") that softens the direct-engine red flags.
- Can lean on long-standing underwriter relationships to write a referred risk inside 48 hours.

**Premierline** — https://www.premierline.co.uk
- Phone: **0333 320 6009**.
- Owned by Beyond Doubt Ltd.
- Has a panel that includes Hiscox among others. [Source: https://www.premierline.co.uk/about-us/contact-us/] [Source: https://www.premierline.co.uk/about-us/insurers/hiscox-insurance/]
- Strong on SME combined business policies; will handle pre-revenue Ltds.
- No specialist "wholesale-only" division but a generalist broker that knows the wholesale trades.

**Towergate** — https://www.towergateinsurance.co.uk
- New business enquiries: **0330 123 5922**.
- Existing customers: 0330 123 3322.
- [Source: https://www.biba.org.uk/find-insurance/broker-directory/towergate-insurance-london/]
- Large national broker with a wholesale and manufacturing team.
- Will handle chemicals and medical-consumables wholesale.
- Strong on broker-led combined business policies.
- Now operating partly under the "Everywhen" rebrand. [Source: https://www.everywhen.co.uk/locations/london]

**Markel UK (broker channel)** — https://uk.markel.com
- Markel International is a Lloyd's-and-London-market insurer with a UK SME broker channel.
- For chemical / medical-consumable wholesale this is one of the better capacity providers.
- Their broker channel writes specialist risk that Markel Direct refuses online.

**Hiscox (broker channel)** — https://www.hiscox.co.uk/broker/contact
- Broker line: **0800 042 0345**.
- For higher PL/PrL limits and trickier trades, Hiscox Broker writes risk Hiscox Direct will not.
- [Source: https://www.hiscox.co.uk/broker/contact]

**Hanwell Insurance Services**
- [VERIFY: Hanwell is a regional broker; their specialism in wholesale-distribution for new Ltds is anecdotal — confirm with phone call.]

**Specialist SME brokers worth contacting:**
- Insure24
- AJG (Arthur J Gallagher)
- Howden
- Lockton (SME division)
- PIB Insurance
- NFP — has a wholesale-insurance practice. [Source: https://www.nfp.co.uk/commercial-insurance/by-sector/wholesale-insurance/]

**Typical broker fee structures.**
- Most UK SME brokers are commission-paid (insurer pays them; the customer does not see a separate fee) for policies under £5k.
- Larger policies may attract a 5–10% brokerage on top of net premium, disclosed up front.
- Some brokers charge a one-off **administration fee** of £25–£75 at bind and renewal — ask about this explicitly.
- [VERIFY: each broker's fee disclosure document.]

**What to ask a broker on the first call (a 5-question opener for Sorellon):**

1. "We are a pre-revenue UK Ltd incorporated July 2025, wholesale-distributing PPE, hygiene wipes, healthcare consumables (gloves, syringes, dressings, catheters), cleaning chemicals, janitorial supplies, painting kits and stationery. We import from UK/EU/China and deliver across UK. Can you place a combined policy with £10M EL, £5M (or £10M) PL, £10M PrL aggregate, £25k stock-in-transit, £100k product recall? What is your indicative range?"
2. "Are any of your panel insurers happy with a residential registered office where no stock is stored?"
3. "Does the activity dropdown have separate lines for 'wholesale of disinfectants', 'wholesale of medical consumables', and 'wholesale of PPE'? Or are these combined?"
4. "What is the bind-stage premium loading for first-year-trading? For projected-turnover-£100k? For China-origin products?"
5. "Can you bind cover within 5 business days, and what evidence will the insurer want?"

---

### 11.9. Comparison aggregators

- **NimbleFins (https://www.nimblefins.co.uk).** Strong editorial coverage of UK SME insurance prices — useful for benchmark research but not itself a broker. [VERIFY: NimbleFins' panel coverage of pre-revenue Ltds — they generally redirect to Simply Business / Superscript / direct insurers.]
- **MoneySuperMarket Business (https://www.moneysupermarket.com).** Has a business-insurance section but in practice routes most quotes to Simply Business or to direct underwriters.
- **CompareTheMarket Business.** Similar — routes to broker partners.
- **Howden / GoCompare for Business.** Similar routing.

**Reality:** Aggregators in UK business insurance overwhelmingly white-label Simply Business or a small set of brokers. For Sorellon, calling Simply Business directly + 1 specialist broker (Premierline or Towergate) covers ~90% of capacity. The aggregator step adds little value.

---

### 11.10. Underwriting checks for new businesses (what Sorellon will be asked)

Pre-revenue Ltds are not refused outright — but the questionnaire is specifically designed to flush out the risks Sorellon's profile presents. Expect to be asked (and to need to be ready with answers for) every one of the following.

**Trading history.**
- Sorellon has none.
- **Answer:** "Pre-revenue, incorporated 2025-07-10, no trading history yet."
- This is honest and will not auto-decline at any of the major SME platforms.
- But it pushes premium **up** because the underwriter has no claims experience to discount from.
- Conservative honest answer is better than overclaiming.

**Bank account / proof of business activity.**
- Sorellon should have a business bank account before applying for insurance.
- Not because the insurer demands it, but because the direct-debit / collection will hard-fail without one.
- [VERIFY: Sorellon's banking status — see Section 4 / Banking research.]

**Director personal credit.**
- Most SME EL/PL/PrL policies do NOT credit-check the director.
- Cyber insurance and D&O may; product-recall and high-limit policies may.
- Bad personal credit will not block a standard combined business policy in 2026.
- But it may block monthly direct-debit and push Sorellon to annual payment.
- [Source: https://www.simplybusiness.co.uk/legal/important-information/]

**Named senior officers / company structure.**
- Standard questionnaire asks for director(s) and beneficial owners.
- Sorellon should declare Zac Farrell (sole director) and the PSC structure (co-PSC Dieter Xia Shuang Danicek).
- No insurer requires a CV in trade for SME combined business cover.
- But a one-line "director has X years experience in Y" can soften a referred underwriting decision.
- [VERIFY: Zac Farrell's prior trade experience — relevant to softening underwriter concerns at the £10M PrL tier.]

**Premises survey.**
- Standard SME PL/PrL/EL is rated off the declared address.
- Full surveys are rare under £5M PL and uncommon under £10M for an administrative office.
- **The residential apartment (Avantgarde Tower, E1 6GS) is the single biggest underwriting red flag.**
- Brokers will ask:
  - "Is there any stock at this address?"
  - "Any business activity beyond email/admin?"
  - "Any visitors?"
- If the answer is "administrative only, no stock, no visitors", insurers will write liability cover without difficulty.
- If Sorellon stores any stock at the residential address, expect:
  - (a) Refusal of stock cover under most policies.
  - (b) Higher PL rate.
  - (c) Potential breach of the apartment's lease/insurance.
  - (d) Potential breach of the registered office's residential planning permission.
- **Operational recommendation: declare honestly that the apartment is administrative-only and never store stock there — use a 3PL or distribution partner for stock.**
- [VERIFY: confirm with Sorellon's apartment freeholder and the LBTH that business use of the kind described is permitted; some E1 6GS leases prohibit business activity.]

**Estimated turnover.**
- Underwriters will ask for a projected next-12-months turnover figure.
- Sorellon must produce one — declaring £0 may auto-decline at some platforms (because the rating engine cannot calculate).
- Recommended approach: declare a conservative projection (e.g., £100k–£250k for Year 1).
- Include a note: "this is a projection; we are pre-revenue and will notify the insurer if turnover materially differs".
- A material divergence at renewal can result in premium correction.
- Failure to notify can void cover.
- [Source: https://www.hellopolaris.co.uk/blog/what-happens-when-declared-turnover-no-longer-matches-reality/]

**Activity / SIC code.**
- Sorellon's Companies House SIC is 46900 (Non-specialised wholesale trade).
- Insurer activity dropdowns rarely use 46900 cleanly — they will ask which specific categories are sold.
- **Recommended declaration:**
  - "Wholesale of PPE (gloves, masks)"
  - "Wholesale of medical / healthcare consumables (syringes, dressings, catheters)"
  - "Wholesale of cleaning chemicals and disinfectants (EN1276/EN14476)"
  - "Wholesale of janitorial supplies"
  - "Wholesale of stationery and paint kits"
  - "No manufacturing. No retail. B2B only."
- Activity selection materially changes premium.

---

### 11.11. CCS framework minimum cover levels

The Crown Commercial Service runs hundreds of framework agreements ("RM6xxx") under which government departments and the wider public sector buy goods and services. Each framework has a Joint Schedule 3 (insurance) and each call-off may set higher contract-specific levels.

**Baseline pattern across CCS frameworks.**
- The Mid Tier and Public Sector terms (used in most goods/services frameworks) set the supplier insurance requirement at:
  - **EL minimum £5M per claim** (statutory minimum)
  - **PL minimum £5M per claim**
  - **PrL minimum £5M per claim**
- With the right of the contracting authority to require higher levels per call-off.
- Coverage must be maintained for the duration of the contract and typically for **6–21 years post-termination** depending on the framework (long-tail product/professional claims).
- [Source: https://assets.crowncommercial.gov.uk/wp-content/uploads/RM6240-Framework-Schedule-1-SpecIfication.pdf] [Source: https://www.clacks.gov.uk/document/7034.doc]

**RM6178 (Face Coverings / PPE) — historical framework, post-COVID.**
- [VERIFY: RM6178 specific insurance schedule; the COVID-era PPE frameworks set EL/PL/PrL at £5M minimum but call-offs often pushed to £10M.]
- Replaced/succeeded by later supply arrangements.

**RM6240 — Public Sector Legal Services.**
- Not goods; not directly relevant to Sorellon.
- (The user's brief associates RM6240 with cleaning — that mapping is incorrect; RM6240 is legal services per the framework specification.)
- [Source: https://www.crowncommercial.gov.uk/agreements/RM6240]

**RM6331 — Healthcare Soft FM** (cleaning, catering, portering, waste management for healthcare).
- Goods component (cleaning chemicals, hygiene products) is procured via supply contracts under this framework.
- **Insurance baseline:** £5M EL / PL / PrL minimum, often £10M for call-offs touching NHS estate.
- [Source: https://www.crowncommercial.gov.uk/agreements/RM6331]
- [VERIFY: live Joint Schedule 3 at award time.]

**RM6378 — Facilities Management** (succeeding RM6232).
- Similar pattern.
- £5M minimum, £10M typical for larger call-offs.
- [Source: https://psip.co.uk/blog/ccs-rm6378-facilities-management-framework]

**RM6232 — FM and Workplace Services.**
- Predecessor framework. Same insurance pattern.

**RM6206 — Estates Professional Services.**
- Not applicable to Sorellon (advisory services).

**PPN 014 (Cyber Essentials).**
- Mandatory for central-government contracts handling personal data or OFFICIAL ICT systems.
- **Not** an insurance requirement; a certification requirement.
- Sorellon will hit this if it supplies a government department that touches employee/citizen data — unlikely for a pure goods wholesaler.
- [Source: https://www.gov.uk/government/publications/ppn-014-cyber-essentials-scheme/ppn-014-cyber-essentials-scheme-html]

**Operational point for Sorellon:**
- Bind insurance at **£10M PL / £10M PrL / £10M EL** to satisfy the **highest plausible CCS call-off level** without needing to re-quote at tender.
- The marginal cost from £5M to £10M is typically £100–£300/yr.
- The cost of failing a tender because cover is only £5M is the entire tender.

---

### 11.12. NHS Supply Chain minimum cover levels

NHS Supply Chain (SCCL — Supply Chain Coordination Limited) is the primary procurement vehicle for goods into the NHS. Sorellon's category mix (PPE, gloves, disinfectants, healthcare consumables) is exactly the NHS Supply Chain catalogue territory.

**Baseline (every NHS Supply Chain Framework Agreement):**
- "The Supplier shall maintain insurance arrangements in respect of employer's liability, public liability (including pure economic loss), and product liability in accordance with Good Industry Practice with the minimum cover per claim being the greater of **five million pounds (£5,000,000)** or any sum required by Law unless otherwise agreed with NHS Supply Chain in writing."
- [Source: https://www.sccl.nhs.uk/wp-content/uploads/2023/10/Appendix-4-NHS-Supply-Chain-Framework-Agreement-for-the-Supply-of-Goods.pdf]

**Post-termination cover:**
- "ongoing liability...shall continue to be the subject of appropriate indemnity arrangements for the period of **twenty one (21) years** from termination or expiry of the Framework Agreement."
- [Source: https://www.sccl.nhs.uk/wp-content/uploads/2023/10/Appendix-4-NHS-Supply-Chain-Framework-Agreement-for-the-Supply-of-Goods.pdf]
- **This is operationally significant** — it effectively requires Sorellon to maintain cover (or run-off cover) for 21 years post-contract.

**MIA (Master Indemnity Agreement) supplier register.**
- To supply goods directly to NHS Trusts (rather than via the Supply Chain catalogue), Sorellon must register on the MIA.
- Insurance evidence is mandatory at registration:
  - **£5M Product Liability minimum per claim**
  - **£5M Public Liability minimum per claim**
- Certificates must be re-uploaded before expiry, or the supplier is dropped from the register.
- [Source: https://www.supplychain.nhs.uk/suppliers/master-indemnity-agreement-approved-suppliers/]

**Call-off practice.**
- Higher-risk product categories (sharps, syringes, catheters, implantables) often have call-off-specific £10M PrL requirements.
- Disinfectants in healthcare settings have appeared at £10M in recent contract launches.
- [VERIFY: latest RM6 / NHS Supply Chain Cleaning Supplies and Environmental Decontamination contract launches — re-check the live insurance schedule at the point of application.]
- [Source: https://www.supplychain.nhs.uk/product-information/contract-launch-brief/environmental-decontamination/] [Source: https://www.supplychain.nhs.uk/product-information/contract-launch-brief/cleaning-supplies-equipment-and-associated-products/]

**Sorellon-specific.**
- Buying £10M PL / £10M PrL / £10M EL is the right move for NHS Supply Chain.
- The 21-year run-off requirement is the genuine sting in the tail.
- Sorellon should price annual renewals over that horizon when sizing margins.
- Even modest run-off premium drift (£200/year × 21 years = £4.2k) is meaningful for a low-margin wholesale book.

---

### 11.13. Local authority typical minimums

District councils, county councils, NHS Trusts (where buying outside the Supply Chain), education authorities, and similar local public buyers each set their own insurance schedules. They cluster as follows:

- **District / borough council goods contracts:** £5M PL, £5M PrL, £5M (or statutory) EL is the standard floor. [Source: https://www.clacks.gov.uk/document/7034.doc]
- **County council / unitary authority larger contracts:** £10M PL, £10M PrL routinely required for any contract above ~£100k value.
- **NHS Trust direct purchases (outside Supply Chain):** Trust-specific procurement teams typically mirror the NHS Supply Chain baseline (£5M minimum, frequently £10M for clinical-adjacent goods).
- **Education authorities / FE colleges:** £5M PL, £5M PrL minimum. Higher for catering and cleaning chemical supply.
- **Police / Fire / Ambulance:** £10M PL, £10M PrL typically.
- **Run-off:** Local authority contracts typically require **6 years** post-termination run-off cover (Limitation Act simple-contract limitation period). Healthcare-related contracts mirror NHS Supply Chain's 21 years for product claims.

**Sorellon-specific.** Binding £10M PL / £10M PrL means Sorellon does not have to re-quote for the vast majority of LA and NHS-Trust goods contracts.

---

### 11.14. MoD / JOSCAR minimum cover levels

JOSCAR (Joint Supply Chain Accreditation Register), run by Hellios, is the prequalification system used by MoD, BAE Systems, Rolls-Royce, Babcock, Boeing, Leonardo, QinetiQ and other UK defence/aerospace/security primes.
- [Source: https://hellios.com/joscar] [Source: https://www.qinetiq.com/-/media/dfadf4f2226943008cf047d61841088a.ashx]

**Stage 1 (mandatory for all suppliers):**
- Includes upload of EL, PL and PrL certificates.
- The portal's Document Scanner extracts cover levels automatically.
- [Source: https://hellios.com/hubfs/JOSCAR%20Supplier%20Compliance%20Management_An%20ASC%20Suppliers%20Guide.pdf?hsLang=en]

**Typical buyer requirement at MoD/JOSCAR-touching call-off:**
- **£10M PL, £10M PrL, £10M EL.**
- [VERIFY: each prime sets its own threshold; £10M is the common floor and £25M is not unusual for direct-MoD high-value contracts.]
- Some defence call-offs require £25M+ for PrL on safety-critical items.

**Cyber Essentials Plus** is required for many MoD contracts (per Def Stan 05-138).
- Sorellon would need to upgrade Cyber Essentials → Cyber Essentials Plus (~£1,500–£3,500 audit cost) to pursue defence-direct work.

**Other JOSCAR document requirements** (relevant context):
- ISO 9001 (quality)
- ISO 14001 (environment)
- ISO 27001 (information security)
- Counterfeit-parts management policy (CCAP/SAE AS5553)
- Business continuity plan
- Modern slavery statement
- Anti-bribery policy
- None of these are insurance but all sit alongside the insurance evidence in the JOSCAR pack.
- [Source: https://hellios.com/hubfs/JOSCAR%20Supplier%20Compliance%20Management_An%20ASC%20Suppliers%20Guide.pdf?hsLang=en]

**Sorellon-specific.**
- Defence/aerospace wholesale is a different game from NHS/LA.
- **Recommendation: defer JOSCAR registration until Sorellon has 12+ months trading and the ability to evidence ISO 9001 (or a documented QMS) and Cyber Essentials Plus.**
- The £10M cover is necessary but nowhere near sufficient — the JOSCAR pack is a year-long onboarding for a new entrant.

---

### 11.15. Recommended broker outreach order for Sorellon

The operator should contact brokers in roughly this order. The objective is to bind a combined business policy (EL £10M, PL £10M, PrL £10M aggregate, £25k stock-in-transit, £100k product recall, optional £250k cyber) within 5–10 business days at a target premium of £1,000–£2,500/year. Use the script in §5.8 ("What to ask a broker on the first call") and tailor as below.

1. **Premierline Insurance Brokers** — **0333 320 6009** — https://www.premierline.co.uk
   - **Why first.** Generalist SME broker with a wide panel (including Hiscox). Strong on combined business policies for newly-formed Ltds. Fast turnaround. Likely to come back within 48 hours with 2–3 indicative quotes.
   - **What to say.** "Sorellon Limited, Companies House 16575318, incorporated July 2025, pre-revenue, B2B wholesale distributor of PPE, hygiene wipes (EN1276/EN14476), healthcare consumables (gloves, syringes, dressings, catheters), cleaning chemicals, janitorial supplies, painting kits and stationery. UK delivery, China imports. Sole director, no employees beyond director, projected first-year turnover £100k–£250k. Registered office is residential (administrative only, no stock storage). Need combined business policy: £10M EL, £10M PL, £10M PrL aggregate, £25k stock-in-transit (no fixed warehouse), £100k product recall extension. Can you quote?"
   - **What to ask for.** Indicative annual premium, named insurer panel, broker fee/admin charge, document list for bind.

2. **Towergate Insurance Brokers (new business)** — **0330 123 5922** — https://www.towergateinsurance.co.uk
   - **Why second.** Large national broker with a wholesale-and-manufacturing team. Will route Sorellon to a dedicated wholesale specialist. Likely the most knowledgeable on chemicals + medical-consumables wholesale of the generalist brokers.
   - **What to say.** Same script. Add: "Please route to your wholesale/distribution team."
   - **What to ask for.** Same. Specifically ask if they have any wholesale-distributor scheme/binding-authority rates (Towergate sometimes does).

3. **Simply Business** — **0333 014 6683** (Mon–Fri 9–5:30) — https://www.simplybusiness.co.uk/business-insurance/wholesale-insurance/
   - **Why third.** Largest UK SME digital broker. Will quote online in 7–15 minutes for the basic combination; phone for the £10M PrL upgrade. Useful as a price benchmark against the specialist brokers above.
   - **What to do.** Get an online quote for the basic combined policy at £5M, then phone to upgrade limits and add product recall.
   - **What to ask for.** Quote for £5M baseline AND £10M upgrade option; product recall extension; cyber add-on cost.

4. **Hiscox (Broker channel)** — **0800 042 0345** — https://www.hiscox.co.uk/broker/contact
   - **Why fourth.** Hiscox is one of the leading London-market SME insurers; their Broker channel writes risks that Hiscox Direct will not. Specifically strong on product liability for new businesses.
   - **What to say.** "I'd like to be routed to a Hiscox-appointed broker who can quote for a pre-revenue UK Ltd wholesaler of PPE / healthcare consumables / cleaning chemicals." Hiscox themselves will direct to a broker partner.
   - **What to ask for.** Hiscox PrL appetite for this trade; £10M aggregate availability; specific approach to first-supplier-into-UK imports from China.

5. **Markel UK (Broker channel)** — https://uk.markel.com/insurance — phone via BIBA broker directory or directly via uk.markel.com contact form.
   - **Why fifth.** Markel International is a strong Lloyd's-market capacity provider for harder-to-place SME trades. Worth a placement quote even if Premierline/Towergate already cover Markel on their panel — sometimes a direct Markel broker comes back better.
   - **What to ask for.** Specifically: PrL aggregate cover at £10M for chemicals + medical consumables + PPE imports.

**A 6th option (only if 1–5 cannot bind within 10 business days):**

6. **NFP UK Wholesale Insurance practice** — https://www.nfp.co.uk/commercial-insurance/by-sector/wholesale-insurance/ — phone via the practice page. Specialist wholesale sector team. Premium positioning is typically higher than Premierline/Towergate but they will write risks that generalists baulk at.

**Direct online (parallel exercise, useful as a sanity check):**

7. **Simply Business online wholesale quote** — https://www.simplybusiness.co.uk/business-insurance/wholesale-insurance/ — 7-minute self-serve. Bind the basic policy if needed as an interim while brokers come back with the better £10M placement.

**What to send to every broker (one-time pack to email after the first call):**
- Companies House summary (#16575318)
- One-paragraph trading description (the script above)
- List of product categories Sorellon will sell (PPE, hygiene wipes EN1276/EN14476, gloves, syringes, dressings, catheters, cleaning chemicals, janitorial supplies, painting kits, stationery)
- Supplier QA narrative: "We require CE/UKCA marking and EN-standard evidence from every supplier; we do not re-label or repackage; we maintain SDS files; we use 3PL distribution and do not store stock at the registered office."
- Estimated turnover: £100k–£250k Year 1 projection (revisit at 6-month renewal call).
- Risk-management statement: "We will not supply medicines or licensed medical devices Class IIa or above without separate authorisation; our category is consumables and PPE only." [VERIFY: confirm Sorellon's product list against UK MDR Class II definitions — syringes and catheters are typically Class IIa; this may attract higher PrL loading.]

**After bind:**
- Upload certificate(s) to Achilles, Constructionline, NHS Supply Chain MIA register (when applying), JOSCAR (deferred), and to any Sorellon vendor-management portal (Coupa, Ariba, Atamis, Bravo, Jaggaer) at the point of each tender.
- Set 60-days-before-expiry calendar reminders.
- Display EL certificate digitally (Google Drive folder shared with the director; or printed at registered office).
- Maintain a one-page "insurance summary" PDF for procurement officers who ask informally — saves an hour of certificate-chasing on every tender.

---

### 11.16. Summary table — recommended cover for Sorellon (Year 1)

| Cover | Limit | Indicative annual premium (bundled) | Mandatory? | Priority |
|---|---|---|---|---|
| Employers' Liability | £10M | £20–£60 inside bundle | Statutory-exempt for Sorellon today; procurement-mandatory | Buy |
| Public Liability | £10M | £400–£900 | Contract-mandatory (CCS/NHS/LA/MoD) | Buy |
| Product Liability | £10M aggregate / £5M each | £700–£2,000 | Contract-mandatory; commercially essential | Buy |
| Product Recall extension | £100k–£250k | £100–£400 | Optional but valuable for chemicals/healthcare | Buy |
| Stock-in-transit | £25k | £100–£200 | Optional | Consider |
| Business contents (admin office) | £5k–£10k | £50–£150 | Optional; minimal at residential office | Consider |
| Cyber liability | £250k–£500k | £300–£600 (or free £25k via Cyber Essentials) | Procurement-driven (PPN 014 = CE not insurance) | Defer to Year 2 unless required |
| Professional Indemnity | n/a | — | Not applicable (skip in v1) | Skip |
| Legal expenses | included | included | Useful add-on | Include |
| **Total combined business policy** | — | **~£1,000–£2,500/year** | — | — |

[VERIFY: All premium ranges above are indicative bind-stage targets. Real quotes from at least Premierline, Towergate, Simply Business and Hiscox Broker are required before finalising the operator's outreach plan.]

---

### 11.17. Open questions / verifications needed before broker outreach

1. **Shareholding split.** Confirm Zac Farrell holds ≥50% of Sorellon shares — required for EL statutory exemption (though we recommend buying EL regardless). [VERIFY: PSC07 / SH01 filings at Companies House #16575318.]
2. **Apartment lease.** Confirm Avantgarde Tower (E1 6GS) lease permits business use of the kind described (administrative-only, no stock). [VERIFY: lease and Tower Hamlets planning use class.]
3. **Business bank account.** Confirm Sorellon has a UK business bank account capable of direct-debit insurance payments. [VERIFY: Section 4 / Banking.]
4. **Product list — UK MDR class.** Confirm whether syringes and catheters Sorellon will sell are pre-filled / drug-coated (Class IIb+) or plain (Class IIa); medical-device classification materially changes PrL underwriting and may require MHRA economic-operator registration. [VERIFY: MHRA economic-operator registration requirement under UK MDR 2002 (as amended) for distributors of medical devices.]
5. **Disinfectant biocide registration.** EN1276/EN14476 are efficacy claims; UK biocide products also need authorisation under the GB Biocidal Products Regulation (BPR). Sorellon as distributor may have a duty to verify supplier authorisation. [VERIFY: HSE Biocides Unit guidance.]
6. **Cyber Essentials timeline.** Plan to certify within 60 days of trading start (£300+VAT, 3–4 weeks) — gates PPN 014 public-sector work and unlocks the free £25k cyber cover bundle.
7. **CCS framework live insurance schedules.** Re-pull the Joint Schedule 3 of any framework Sorellon applies to, at the point of application — they update. [VERIFY: framework number in force, Joint Schedule 3 version.]
8. **Run-off cover horizon.** Confirm 21-year NHS Supply Chain post-contract requirement is current in 2026 — and price renewals over that horizon when costing margins. [VERIFY: latest NHS Supply Chain Framework Agreement template.]

---

## 12. Certifications & Supplier Schemes

**Subject:** Sorellon Limited (Companies House #16575318) — UK certifications and supplier-scheme accreditations relevant to public-sector bidding in PPE, hygiene wipes, healthcare consumables, cleaning products, and janitorial supplies.

**Bidder profile recap (drives the analysis):**
- ~10 months trading (incorporated 2025-07-10); pre-revenue.
- Sole director and sole employee (Zac Farrell). No payroll beyond director.
- Registered office at a London E1 6GS residential apartment (no warehouse, no operating site).
- SIC 46900 wholesale of unspecified goods. B2B reseller / distributor business model.
- Full written policy suite exists but is NOT externally audited or certified.
- IT estate: Google Workspace + Netlify static site. Trivial cyber scope.

**Key framing:** UK public procurement does NOT use a single mandatory certification list. Each contract / framework / buyer chooses its own "selection-stage" evidence requirements under the Procurement Act 2023 (which began applying to procurements commenced on or after 24 February 2025). Cyber Essentials is mandated by PPN 014 (which replaced PPN 09/14 and PPN 09/23) for any contract handling citizen personal data, government employee data, or ICT systems at OFFICIAL classification or above. Everything else (ISO 9001, ISO 14001, Constructionline, etc.) is buyer-discretionary. The job for Sorellon is to build the **smallest credible bundle** that passes the most-frequently-asked selection questions for its product space, not to over-collect certifications. [Source: https://www.gov.uk/government/publications/ppn-014-cyber-essentials-scheme/ppn-014-cyber-essentials-scheme-html]

---

### 12.1. Cyber Essentials (basic / self-assessment)

**1. What it is**
Cyber Essentials is a UK government-backed, IASME-administered self-assessment cyber-security scheme. Five technical control families: firewalls, secure configuration, user access control, malware protection, and security update management. A signed Director attestation plus an IASME-side verification produces a certificate listed on the IASME public registry. Cyber Essentials Plus (separate item below) is the audited variant of the same controls.

**2. Mandatory?**
Effectively yes for any central-government, MoD, NHS, or Crown-Commercial-Service framework that touches personal data, employee data, or OFFICIAL-tier ICT. Under PPN 014 (current as of 2026, replacing PPN 09/14 and PPN 09/23) contracting authorities in scope of the policy MUST require Cyber Essentials or Cyber Essentials Plus where those characteristics are present. Local authorities are not bound by PPN 014 but increasingly mirror it. The Procurement Act 2023 also requires authorities to accept "equivalents" (so a credible ISO 27001 plus a mapping document can substitute), but in practice buyers run an IASME-registry lookup before contract award. For Sorellon's space (PPE / cleaning / hygiene) the **product itself** is not personal data, but ANY framework that involves order portals, supplier-portal sign-on, electronic invoicing, EDI, or NHS-affiliated buying will normally still ask for it as a baseline filter. [Source: https://www.gov.uk/government/publications/ppn-014-cyber-essentials-scheme/ppn-014-cyber-essentials-scheme-html] [Source: https://assets.publishing.service.gov.uk/media/67af78c8a75f02dffca29bd8/PPN_014_Cyber_essentials_scheme.pdf]

**3. Eligibility**
Any UK organisation of any size. There is no minimum trading history, no minimum employee count, and no revenue threshold. Sole-director limited companies are routine applicants.

**4. Cost (2026)**
Tiered by organisation size, set by IASME in April 2025 and held into 2026:
- Micro (1-9 employees): **£330 + VAT** assessment fee
- Small (10-49 employees): £400 + VAT
- Medium (50-249 employees): £450 + VAT
- Large (250+ employees): £500 + VAT
Includes one year of unlimited self-assessment attempts and free Cyber Liability Insurance for UK organisations with turnover under £20m (Sorellon qualifies). Add VAT 20% (£66 on the micro fee). [Source: https://www.isms.online/cyber-essentials/cost/] [Source: https://iasme.co.uk/cyber-essentials/]

[VERIFY: pricing pages from at least two licensed Certification Bodies (e.g. Cyphere, Connection Technologies, Fig) sometimes show £300 — possibly residual 2024 pricing. Cross-check with IASME's own pricing page before booking.]

**5. Timeline**
For Sorellon's IT footprint (Google Workspace + Netlify, no on-prem servers, sole user), realistic end-to-end is **1-3 weeks** including evidence collation. The questionnaire takes 1-2 days of focused work; IASME marks it within ~3 working days. Faster on FastTrack-style consultancy assistance but not necessary at this scope. [Source: https://cefasttrack.com/blog/cyber-essentials-cost-2026]

**6. Issuer**
IASME (the National Cyber Security Centre's delivery partner) directly, OR any of ~300 licensed Certification Bodies (CBs) trading under the IASME mark. Pricing is set at the IASME level; CBs add their own consultancy markup if they bundle preparation support. [Source: https://www.ncsc.gov.uk/cyberessentials/overview]

**7. Application process**
(a) Create account on the IASME portal. (b) Confirm scope (whole organisation, or named sub-set — for Sorellon, "whole organisation" is the right and only sensible scope). (c) Complete the self-assessment questionnaire (Montpellier question set in 2026, transitioning to "Danzell" question set from April 2026 for any account created after 26 April 2026 — accounts created earlier get a 6-month grace period). (d) Pay the assessment fee. (e) Director (Zac Farrell) signs the attestation. (f) IASME assessor reviews; pass / fail / clarifications. (g) Certificate issued and listed on the IASME registry.

**8. Prerequisites**
None formal. Practically: a documented IT asset inventory (which Sorellon's InfoSec / Cyber policy should describe), MFA enabled on Google Workspace and Netlify, password policy in place, automatic OS updates enabled on all devices, supported OS versions only, and a basic patch-status review across endpoints. Sorellon's existing written InfoSec policy is helpful **as context** but the assessment is question-and-answer about live controls, not a policy review.

**9. Renewal cycle**
Annual. The certificate expires exactly 12 months from issue. The IASME assessment fee is payable each year at then-current pricing. Failing to renew on time means lapsed status — buyers who run registry lookups will see "no current certification". [Source: https://connection-technologies.co.uk/blog/cyber-essentials-renewal-uk-2026]

**10. Ongoing obligations**
Maintain the five control families continuously (not just at audit). Notable in-life requirements: MFA on cloud admin accounts, supported OS only (no Windows 10 unattended past EoS, no abandoned legacy software), automatic security updates within 14 days of vendor release, and a documented removable-media stance. If Sorellon onboards a second employee, that user must be inside scope.

**11. Risk if absent**
HIGH for any central-gov, MoD, NHS, or CCS-framework bid. Many buyers use the IASME registry as an automated pre-qualification filter — no certificate = bid automatically excluded at the selection-stage gate, before evaluators see commercial content. For local-authority cleaning / janitorial tenders, absence is "yellow card" not "red card", but presence is consistently positively scored.

**12. Sequencing rank**
**FIRST.** Lowest cost, fastest, broadest acceptance. Single biggest unlock per pound spent. Must precede Cyber Essentials Plus (it's the literal prerequisite) and provides the InfoSec evidence that pre-empts other questions in PAS-91 / SQ Module 3 / Constructionline / JOSCAR / Achilles.

**13. Sorellon-specific notes**
- Scope is trivial: 1 user, Google Workspace, Netlify, no on-prem, no BYOD beyond Zac's own device(s).
- Critical control gap to close BEFORE applying: ensure every device used for work has full-disk encryption (BitLocker / FileVault), supported OS, automatic updates, MFA-enforced Google Workspace, antimalware (built-in Defender / XProtect counts).
- The residential office address is fine — Cyber Essentials does not assess physical security beyond device-level locking.
- Apply via IASME direct (cheapest) unless Sorellon also wants consultancy hand-holding from a CB. Direct route: ~£396 inc. VAT all-in.

---

### 12.2. Cyber Essentials Plus (audited variant)

**1. What it is**
Same five-control technical scope as Cyber Essentials, but assessed by an external IASME-licensed assessor through a hands-on technical audit: vulnerability scans (internal and external), a sample of user devices physically / remotely tested for malware-protection effectiveness and patch currency, and an MFA-enforcement check. The certificate carries materially more weight with risk-conscious buyers (MoD, defence primes, parts of the NHS, financial-services-adjacent contracts).

**2. Mandatory?**
PPN 014 lets the contracting authority require EITHER Cyber Essentials OR Cyber Essentials Plus depending on assessed risk. CE+ is mandated where personal data volumes are high, ICT classification is OFFICIAL-SENSITIVE-adjacent, or the buyer's own risk model demands it. Sorellon's product space (physical PPE / cleaning supply wholesale) almost NEVER requires CE+; some MoD non-warlike supply lines do, and a small number of NHS framework lots (especially those tied into NHS Supply Chain master agreements) may. [Source: https://www.isms.online/cyber-essentials/plus-requirements/]

**3. Eligibility**
Must hold a current basic Cyber Essentials certificate. The CE+ audit must commence within 3 months of the basic CE certification date — miss the 90-day window and the basic CE has to be re-done.

**4. Cost (2026)**
- IASME audit fee (paid via the CB): variable, ~£1,500-£8,000 + VAT for SMEs, driven primarily by device count and number of cloud services in scope.
- For Sorellon's footprint (1 user, ~2 devices, Google Workspace + Netlify): realistic quote band **£1,500-£2,250 + VAT** all-in including CB time. [Source: https://www.intelance.co.uk/cyber-essentials-plus-certification-uk/]
- Plus the basic CE fee (£330 + VAT) as the gating prerequisite.

[VERIFY: solicit at least 3 CB quotes — there is meaningful (~30-40%) price spread between Cyphere, Fig, Precursor Security, ISMS.online, CertBetter, etc. Pricing for the smallest businesses is sometimes near the £1,500 floor.]

**5. Timeline**
6-8 weeks for a well-prepared single-user organisation, 6-12 weeks more typically. Critical-path items: vuln scan remediation cycle (any "high" finding must be fixed and re-scanned), audit-window scheduling with the CB. [Source: https://connection-technologies.co.uk/blog/cyber-essentials-renewal-uk-2026]

**6. Issuer**
An IASME-licensed Certification Body conducts the audit; IASME issues the certificate. Sorellon CANNOT apply direct to IASME for CE+ — must go via a CB.

**7. Application process**
Pick a CB, scope the audit, pass basic CE (if not already), schedule remote and external vuln scans, host the device sample audit (usually remote screen-share for 1-2 hours per device), provide MFA-enforcement evidence, remediate any findings, re-test, certificate issued.

**8. Prerequisites**
- Basic Cyber Essentials in force, dated within the last 90 days.
- All in-scope devices physically available for the audit window (so don't schedule it during travel).
- Sample devices configured production-equivalent.

**9. Renewal cycle**
Annual, mirrored to basic CE. Practically: re-do basic CE first, then re-do CE+ within 90 days. Compounding annual cost.

**10. Ongoing obligations**
Same as basic CE plus: continuous vulnerability management (not just audit-time), patched-within-14-days discipline on all OS and application updates, documented removable-media policy, and (for the Danzell question set rolling out from April 2026) tighter expectations around password-manager use, MFA on all admin tooling, and supported-vendor inventory.

**11. Risk if absent**
LOW-MEDIUM for Sorellon's typical bid set. CE+ becomes blocking only in defence / MoD non-warlike supply, certain NHS Supply Chain master frameworks, or where the contracting authority has explicitly named "Cyber Essentials Plus" in the tender notice. Useful as a competitive differentiator versus other micro-SME bidders in mid-value tenders.

**12. Sequencing rank**
**FOURTH or later.** Cost-to-benefit for Sorellon's product space is poor until the bidding pipeline actually surfaces a CE+-requiring tender. Strong candidate for "trigger-based investment": pull the trigger only when a specific tender requires it (the 6-8 week timeline is compatible with most public-sector SQ-to-tender windows IF basic CE is already held).

**13. Sorellon-specific notes**
- Defer until at least one targeted lot requires it. Don't pre-buy.
- If a specific MoD or NHS-Supply-Chain lot becomes the target, build the cost into that bid's win economics — £2-3k effective qualification cost is not amortised across many bids.
- The vuln-scan finding-fix cycle is the unpredictable risk factor. Run a one-off Nessus or OpenVAS scan against Sorellon's external footprint (effectively just the Netlify domain) before booking, to surface anything embarrassing.

---

### 12.3. ISO/IEC 27001 (Information Security Management System)

**1. What it is**
The international standard for an Information Security Management System (ISMS). Annex A controls (114 in the 2013 edition; 93 controls in the 2022 revision currently in force). Demonstrates that the organisation has a systemic, risk-based information-security programme — not just point-in-time technical controls.

**2. Mandatory?**
Almost never in Sorellon's product space. Some defence primes, financial-services-adjacent procurement, and a small number of NHS data-processor contracts will ask for ISO 27001. For physical-goods wholesale (PPE / cleaning / hygiene) the bar is usually Cyber Essentials, not 27001. The Procurement Act 2023's "equivalents" clause means ISO 27001 can SUBSTITUTE for Cyber Essentials in a tender response — but most buyers still prefer to see the CE certificate because their evaluator scoring rubric is written around it. [Source: https://www.gov.uk/government/publications/ppn-014-cyber-essentials-scheme/ppn-014-cyber-essentials-scheme-html]

**3. Eligibility**
Any organisation. No minimum size, employee count, or revenue. For very small organisations the certification body may compress the audit days but cannot waive them entirely.

**4. Cost (2026)**
UKAS-accredited route (only credible route — see ESPECIAL FLAG below):
- Micro (1-10 staff): **£8,000-£20,000** year-one all-in (consultancy + Stage 1 + Stage 2)
- Small (11-50 staff): £12,000-£40,000 year-one
- Year-two onwards: surveillance audits £1,500-£3,000/yr
- UKAS-accredited auditor day rates ~£1,250-£1,500/day in 2026
[Source: https://iso27001cost.com/cost-uk] [Source: https://hightable.io/iso-27001-certification-cost/]

[VERIFY: same audit can vary 30-40% across UKAS-accredited bodies — obtain quotes from BSI, NQA, LRQA, Alcumus ISOQAR, and URS before committing.]

**ESPECIAL FLAG — UKAS-accredited vs non-UKAS:**
- **UKAS-accredited certification bodies (only credible for central government):** BSI, NQA, LRQA, Alcumus ISOQAR, URS, SGS, Bureau Veritas, DNV. The full list is on the UKAS register at https://www.ukas.com.
- **Non-UKAS / non-IAF certificates:** Cheaper (often half the cost) but routinely REJECTED by central-government, NHS, and large-buyer procurement teams. Buyers verify IAF recognition through the IAF / UKAS lookup, and certificates from non-recognised bodies are treated as non-compliant evidence. [Source: https://www.rkmsuk.co.uk/blog/non-iaf-iso-certificates-uk/] [Source: https://brightercompliance.co.uk/ukas-vs-non-ukas-iso-certification/]
- **Self-claimed "ISO 27001 compliant" (no certificate):** Worthless in a bid. Do not use this language in tender responses.

**5. Timeline**
6-12 months end-to-end for a first-time SME. Internal preparation (gap analysis, risk register, documented procedures, internal audit, management review) is the long pole. Stage 1 + Stage 2 audits typically span 4-8 weeks once the ISMS is operationally mature (must have ~3 months of live evidence including at least one internal audit and one management review).

**6. Issuer**
Any UKAS-accredited (or IAF-equivalent overseas-accreditor) certification body. For Sorellon, BSI, NQA, and Alcumus ISOQAR are the most frequently named by UK central-gov buyers.

**7. Application process**
(a) Gap analysis. (b) Build the ISMS (scope statement, Statement of Applicability, risk methodology, risk register, policies, procedures, training records, internal audit programme, management review cadence). (c) Run the ISMS live for ~3 months collecting evidence. (d) Internal audit cycle. (e) Management review. (f) Book Stage 1 (documentation review). (g) Address Stage 1 findings. (h) Stage 2 (operational audit). (i) Address findings; certificate issued.

**8. Prerequisites**
None formal; practically, the ISMS must be in live operation with real evidence (not just documents) before Stage 2 succeeds.

**9. Renewal cycle**
3-year certification cycle with annual surveillance audits in years 2 and 3, full recertification audit in year 4.

**10. Ongoing obligations**
Continuous risk register maintenance, supplier-risk reviews, internal audits (minimum annually covering all clauses over the 3-year cycle), management reviews (minimum annually), incident logging, document control, training records, continual-improvement actions.

**11. Risk if absent**
LOW for Sorellon's bid pipeline. Most public-sector PPE / cleaning / hygiene buyers do not list ISO 27001 as a selection criterion. Where it appears, the Procurement Act equivalence clause means Cyber Essentials Plus + a written ISMS-aligned policy set (which Sorellon already has) is usually defensible.

**12. Sequencing rank**
**LATER (year 2-3 if at all).** For a 1-employee pre-revenue org, ISO 27001 is poor value relative to Cyber Essentials Plus. The £8-20k year-one cost is roughly 10-20× the cost of CE+ for marginal incremental selection-stage acceptance.

**13. Sorellon-specific notes**
- DO NOT pursue this in year 1. The certifier will struggle to audit "real operating evidence" on a sole-employee pre-revenue entity, and the cost-per-bid-unlocked is uneconomic at current pipeline size.
- DEFENSIVE ALTERNATIVE: maintain the existing written InfoSec policy + add a one-page ISMS scope statement + a simple risk register + a documented internal audit (Zac auditing Zac is fine on paper) → this gives equivalent-evidence language in bids where ISO 27001 is "preferred" not "required".
- If Sorellon ever wins a contract that mandates ISO 27001 as a CONTRACT condition (vs selection condition), the certification cost can be amortised against that contract's revenue.

---

### 12.4. ISO 9001 (Quality Management System)

**1. What it is**
The international Quality Management System standard. Process-based: documented quality policy, customer-focus, leadership, planning, support, operation, performance evaluation, improvement. Far more widely recognised than ISO 27001 in physical-goods public procurement. Roughly half of all UK public-sector physical-goods tenders ask for ISO 9001 or "equivalent quality system evidence" at the selection stage.

**2. Mandatory?**
Buyer-discretionary; appears as a selection-stage requirement in a meaningful minority of physical-goods tenders, especially construction-adjacent, NHS Supply Chain, and FM-services tenders. Where required, the Procurement Act 2023 demands the buyer accept "equivalents" — but the equivalents claim must be evidenced with a documented quality procedure (see item 6.5 below).

**3. Eligibility**
Any organisation, no size minima. Single-person organisations are certified routinely.

**4. Cost (2026)**
UKAS-accredited route:
- Micro (under 10 employees, single site): **£2,250-£2,750** year-one (audit only); **£6,000-£7,000** all-in including consultancy [Source: https://www.rkmsuk.co.uk/iso-consultancy/how-much-does-iso-9001-cost/]
- Small (under 25 employees): £2,000-£5,000 year-one audit only
- Ongoing surveillance audits: ~£1,500-£3,000/yr
- Subsequent years typically 40-50% lower than year one
- UKAS auditor day rates: £1,000-£1,500/day in 2026
[Source: https://youriso.co.uk/cost-of-iso-9001/]

**ESPECIAL FLAG — UKAS-accredited vs non-UKAS:**
- **UKAS-accredited (only credible for central government):** BSI (premium pricing, premium brand recognition), LRQA, NQA (mid-range and commonly recommended for SMEs), SGS, Bureau Veritas, Alcumus ISOQAR, URS, Amtivo.
- **Non-UKAS / "IAF-non-recognised" bodies:** Cheaper (sometimes £600-£1,500/yr) but central-government, NHS, and tier-1 buyers verify certificates against IAF recognition lists and reject non-UKAS evidence. The cost saving is a false economy if Sorellon's target buyer list includes any central-gov framework. [Source: https://www.rkmsuk.co.uk/iso-consultancy/the-difference-between-ukas-and-non-ukas-certification/] [Source: https://www.riskbriefing.co.uk/businesses-facing-unaccredited-iso-certificates-being-rejected/]
- **Self-declared "ISO 9001 compliant" (no certificate):** Should not be used in tender responses. Buyers treat this as non-evidence.

**5. Timeline**
3-6 months for a first-time SME. Faster than 27001 because the documentation set is lighter and Sorellon already has a Quality-policy-shaped item in its policy suite. Stage 1 + Stage 2 typically 4-6 weeks apart.

**6. Issuer**
UKAS-accredited body. For SMEs: NQA is the most-commonly-recommended; BSI carries the strongest brand premium but charges 20-30% more. [Source: https://compareyourbusinesscosts.co.uk/iso-certification-uk]

**7. Application process**
Same shape as ISO 27001: gap analysis → build QMS → run live for ~3 months → internal audit → management review → Stage 1 (docs) → Stage 2 (operational). Lighter documentation burden.

**8. Prerequisites**
None formal. Practically: a documented QMS in live operation with at least one internal audit and one management review on record before Stage 2.

**9. Renewal cycle**
3-year certification cycle, annual surveillance audits years 2 and 3, full recertification year 4.

**10. Ongoing obligations**
Internal audit programme, management review, document control, customer-feedback tracking, non-conformance / corrective action log, supplier evaluation.

**11. Risk if absent**
MEDIUM. For physical-goods wholesale, ISO 9001 absence is a competitive disadvantage in any tender where it's named — but the "equivalents" route under the Procurement Act 2023 means a well-documented in-house quality procedure can satisfy many SME-friendly buyers. Local authorities are more lenient than central government here.

**12. Sequencing rank**
**SECOND-TIER (year 2).** After Cyber Essentials is in place and after the bid pipeline has shown that ISO 9001 is a recurring blocker. Don't pre-buy in year 1 unless a specific high-value tender is already on the horizon and requires it.

**13. Sorellon-specific notes**
- Sorellon's existing policy suite is policy-shaped, not QMS-shaped. ISO 9001 wants process-and-evidence: customer order intake, supplier qualification, non-conformance logging, corrective actions, internal audit records. These do NOT yet exist for a pre-revenue org.
- Recommend: defer formal certification until Sorellon has executed enough real customer transactions to have meaningful records to audit (typically requires 6-12 months of real trading volume).
- INTERIM: use the "documented quality procedure" route (item 6.5) for tenders in the meantime.

---

### 12.5. Alternative QMS evidence for SMEs (no ISO 9001 certificate)

**1. What it is**
A documented in-house quality procedure submitted as "equivalent evidence" under the Procurement Act 2023's technical-specifications equivalents clause, OR under buyer-specific SQ (Selection Questionnaire) language permitting "ISO 9001 OR equivalent quality system evidence". Not a certification — a defensible written body of evidence.

**2. Mandatory?**
N/A — this is a substitute pathway, used WHEN the buyer accepts equivalents and Sorellon doesn't yet hold ISO 9001.

**3. Eligibility**
Any bidder. The burden of demonstrating equivalence sits on the bidder.

**4. Cost (2026)**
£0 to ~£1,500 (if Sorellon uses a tender-writer / consultant to draft the procedure). Largely Zac's time.

**5. Timeline**
2-4 weeks of focused effort to build a defensible quality-procedure pack.

**6. Issuer**
Self-authored; no external issuer.

**7. Application process**
Build a written Quality Procedure document covering, at minimum:
- Quality Policy (Sorellon already has a written policy — adapt)
- Scope and context of the business
- Process map (lead → quote → order → supplier-sourcing → goods-receipt → onward delivery → customer feedback)
- Supplier qualification and ongoing-review procedure
- Goods inspection and non-conformance handling procedure
- Customer-complaint handling procedure
- Document control and record-retention practices
- Internal review cadence (e.g. quarterly self-review with documented output)
- Continual-improvement mechanism (log of changes, lessons learned)

**8. Prerequisites**
None.

**9. Renewal cycle**
N/A; update the document live as the business changes.

**10. Ongoing obligations**
Actually FOLLOW the documented procedure. If a buyer later audits or asks for sample records, Sorellon must produce evidence consistent with the written procedure — paper without practice is the most common failure mode.

**11. Risk if absent**
HIGH where ISO 9001 is asked but not held. With this written procedure in place, the equivalence claim is defensible; without it, the bid is exposed to a "fail at selection" outcome.

**12. Sequencing rank**
**FIRST (immediate).** Build this in the next 30 days; it costs almost nothing and pre-emptively answers a frequent SQ question.

**13. Sorellon-specific notes**
- This is the highest-ROI single piece of work in the entire certification stack for Sorellon's year-1 reality.
- Useful template: BS EN ISO 9001:2015 clause structure (clauses 4-10). Don't certify against it, but write the procedure in that shape so a later ISO 9001 audit (year 2-3) can re-use 80% of the documents.

[VERIFY: confirm acceptance of equivalents language in actual target tender notices — some authorities still write "ISO 9001 required" without the equivalence wording, which is technically Procurement-Act-non-compliant but happens.]

---

### 12.6. ISO 14001 (Environmental Management System)

**1. What it is**
The international Environmental Management System standard. Documented environmental policy, aspects-and-impacts register, environmental objectives, operational controls, monitoring, evaluation, and continual improvement.

**2. Mandatory?**
Rarely mandatory; frequently scored. Appears in social-value / quality scoring for many UK public-sector tenders, especially construction-adjacent, FM, and cleaning / janitorial frameworks. The Carbon Reduction Plan (separately mandated by PPN 06/21 for contracts above £5m / yr with central gov) is a parallel and INDEPENDENT requirement — Sorellon's existing CRP draft addresses that.

**3. Eligibility**
Any organisation. No size minima.

**4. Cost (2026)**
UKAS-accredited route:
- Micro (under 10 employees, single site): **£2,250-£3,250** initial certification audit only; £3,500-£7,000 all-in with consultancy
- ADD-ON to existing ISO 9001: ~£4,000-£5,000 year-one (combined-audit savings)
- Surveillance audits: £3,000-£4,200/yr ongoing
[Source: https://accendo.org.uk/9001-and-iso-14001-costs-2/] [Source: https://amtivo.com/uk/standards/iso-14001/insights/how-much-does-certification-cost/]

**ESPECIAL FLAG — UKAS-accredited vs non-UKAS:**
- Same logic as ISO 9001. UKAS-accredited route (BSI, NQA, LRQA, etc.) is the only credible route for central-gov bids. Non-UKAS certificates are routinely rejected and the saving is illusory. [Source: https://amtivo.com/uk/ukas-accredited/]

**5. Timeline**
4-6 months for a first-time SME with a written environmental policy already in place.

**6. Issuer**
UKAS-accredited certification body.

**7. Application process**
Same shape as ISO 9001 / 27001. Documentation, live operation, Stage 1, Stage 2.

**8. Prerequisites**
None formal. Practically: aspects-and-impacts register, environmental objectives, monitoring evidence, internal audit cycle, management review.

**9. Renewal cycle**
3-year certification cycle with annual surveillance audits.

**10. Ongoing obligations**
Aspects-and-impacts register maintenance, environmental performance monitoring, legal-compliance evaluation, supplier environmental screening, internal audit, management review.

**11. Risk if absent**
LOW for selection-stage gates (rarely mandatory); MEDIUM for quality / social-value scoring sub-elements. Sorellon's existing Environmental Policy + Carbon Reduction Plan partially substitute for the scoring purpose without the certification.

**12. Sequencing rank**
**LATER (year 2-3, optional).** ISO 14001 makes sense only if Sorellon (a) already has ISO 9001 (combined audit saves cost), AND (b) is consistently bidding in environmentally-scored tenders where the certificate moves the needle.

**13. Sorellon-specific notes**
- Existing Environmental Policy + CRP are the cheap path to environmental credibility for now.
- ISO 14001 adds an aspects-and-impacts register, monitoring evidence, and operational controls — substantial documentation work for marginal selection-stage benefit at Sorellon's scale.
- DEFENSIVE ALTERNATIVE: maintain Environmental Policy + CRP + add a one-page "environmental procedure" describing how Sorellon screens its product range (e.g. preferring biodegradable cleaning products, recyclable packaging) → answers most social-value scoring questions.

---

### 12.7. ISO 45001 (Occupational Health and Safety Management)

**1. What it is**
The international Occupational Health & Safety Management System standard. Replaced OHSAS 18001. Hazard identification, risk assessment, legal compliance, workforce participation, incident reporting, performance evaluation.

**2. Mandatory?**
Almost never mandatory in Sorellon's product space. Strongly weighted in construction, FM, and high-risk physical-services bids. For wholesale / supply-only of PPE / cleaning products, where Sorellon is not the user of the products (the buyer is), ISO 45001 is essentially never relevant.

**3. Eligibility**
Any organisation.

**4. Cost (2026)**
UKAS-accredited route:
- Small (under 20 employees): £4,000-£9,000 year-one all-in
- Annual ongoing costs: £1,500-£3,000/yr
[Source: https://youriso.co.uk/iso-45001-certification-cost/] [Source: https://www.smartqhse.com/safety-blog/iso-45001-certification-cost-guide]

**ESPECIAL FLAG — UKAS-accredited vs non-UKAS:**
- Non-UKAS ISO 45001 certificates exist at £2,500-£10,000 but face the same central-gov rejection risk as other non-UKAS ISO certificates. UKAS-only for credible public-sector use.

**5. Timeline**
4-6 months.

**6. Issuer**
UKAS-accredited certification body.

**7-10. Process / Prerequisites / Renewal / Ongoing**
Same shape as other ISO standards. 3-year cycle with annual surveillance.

**11. Risk if absent**
LOW for Sorellon's product space. Health & safety policy + SSIP membership (item 6.12) cover the same buyer concerns at vastly lower cost.

**12. Sequencing rank**
**SKIP (do not pursue).** Disproportionately expensive for a sole-employee wholesale business. SSIP via SMAS at £195/yr is the proportionate alternative.

**13. Sorellon-specific notes**
- ISO 45001 was designed for organisations with operational workforces exposed to occupational hazards. A sole-director admin business with no warehouse and no on-site staff has minimal applicable hazards.
- Auditors will struggle to find meaningful operational evidence to audit.
- Strong "do not pursue" recommendation for year 1-3.

---

### 12.8. Constructionline (Bronze / Silver / Gold / Platinum)

**1. What it is**
UK's largest pre-qualification supplier register. Owned by Capita. Tiers from Bronze (entry, ~PAS-91-aligned) through Silver, Gold, Platinum. Used by ~3,500 buyers (local authorities, housing associations, NHS Trusts, central-gov bodies, tier-1 contractors). Not just construction — FM, cleaning, janitorial, and related supply businesses are also commonly on it. [Source: https://www.constructionline.co.uk/products-services/memberships/sub-contractor-memberships/]

**2. Mandatory?**
Buyer-discretionary. Many buyers require Constructionline membership (any tier) as a pre-qualification gate for their dynamic purchasing systems and frameworks. The most common required minimum is Silver.

**3. Eligibility**
Any UK supplier. No size minima.

**4. Cost (2026)**
By tier and turnover band (smallest turnover band, no SSIP already held):
- **Bronze: from £249/yr** (basic business and H&S info)
- Silver: company verification + financial assessment + insurance + SSIP-aligned H&S; pricing approximately £400-£700/yr at smallest turnover band [VERIFY: Constructionline does not publish full Silver pricing publicly — quote required]
- **Gold: from £499/yr** (Silver + quality management + SSIP via Acclaim) at smallest turnover band [Source: https://www.constructionline.co.uk/wp-content/uploads/2026/02/CL-Gold-Platinum-Supplier-Checklist-Guide-February-2026.pdf]
- **Platinum: from £2,299/yr** at smallest turnover band (Gold + on-premises H&S audit)
- Joining fee may apply year 1, annual fee only year 2+. [Source: https://www.beaconrisk.co.uk/news/123-is-constructionline-expensive-constructionline-fees-explained]

**5. Timeline**
Bronze: ~2-4 weeks (mostly form-filling).
Silver: 4-6 weeks (financial verification via Dun & Bradstreet, insurance review, SSIP-aligned H&S assessment).
Gold: 6-10 weeks (Silver + Acclaim SSIP assessment).
Platinum: 10-16 weeks (Gold + on-premises audit; not viable from a residential apartment).

**6. Issuer**
Constructionline (Capita) directly. SSIP element is delivered by Acclaim Accreditation (Constructionline's sister brand).

**7. Application process**
Online portal questionnaire keyed to PAS-91 / SSIP Common Assessment Standard. Upload policies, financials (Companies House data auto-pulled), insurance certificates, H&S evidence, quality evidence (for Gold).

**8. Prerequisites**
Bronze: company exists, basic policies, insurance.
Silver: Bronze items + audited financials (Sorellon as pre-revenue with no filed accounts yet may face difficulty here) + SSIP-aligned H&S documentation.
Gold: Silver items + a quality management documented system (ISO 9001 OR equivalent procedure — see item 6.5).
Platinum: Gold items + physical site amenable to audit (not viable for Sorellon).

**9. Renewal cycle**
Annual.

**10. Ongoing obligations**
Update policies and evidence on changes; respond to verification queries; renew insurance certificates annually.

**11. Risk if absent**
MEDIUM-HIGH for many local-authority and housing-association cleaning / janitorial / FM tenders. Many buyers list Constructionline Silver+ as a pre-qualification requirement. Absence is often a "do not pass go" gate.

**12. Sequencing rank**
**SECOND (after Cyber Essentials).** Bronze is the cheap entry. Silver is the standard credibility tier. Skip Gold until ISO-9001-equivalent documentation is in place; skip Platinum entirely (premises audit not feasible).

**13. Sorellon-specific notes**
- Pre-revenue + 10-month trading history creates a financial-verification problem at Silver. Constructionline pulls Dun & Bradstreet data and may rate Sorellon as "high risk" purely on age. Some buyers accept a Director's Personal Guarantee or a recent letter of credit as compensating evidence — buyer-specific.
- Bronze is achievable immediately with Sorellon's existing policy suite + a Cyber Essentials certificate. Realistic target: Bronze in month 2, Silver in month 6-9 once Sorellon has 6+ months of real trading evidence.
- Constructionline + SafeContractor / SMAS is duplicative; pick ONE SSIP scheme (cheapest acceptable to target buyers is SMAS — see item 6.12).

---

### 12.9. Achilles (UVDB / Worksafe / Connexus)

**1. What it is**
Supplier qualification registry across multiple industry verticals. Three relevant communities for Sorellon's space:
- **UVDB** (Utilities Vendor Data Base): the utilities sector (water, energy, telecoms) supplier qualification register. ALL major UK utilities buyers use it.
- **Worksafe**: CDM / construction safety pre-qualification (now mostly subsumed into SSIP-aligned schemes; less relevant).
- **Connexus**: housing-association supplier register (formerly "Building Confidence" rebranded) — relevant for housing association cleaning / janitorial / PPE tenders. [Source: https://www.achilles.com/uvdb/]

**2. Mandatory?**
Buyer-specific. UVDB is effectively mandatory for utilities-sector bids. Connexus is required by participating housing associations.

**3. Eligibility**
Any UK supplier. SMEs explicitly accepted (Achilles markets a "smaller suppliers" tier).

**4. Cost (2026)**
- UVDB self-registration tier: **from £145** for businesses under 10 employees and £1m turnover [Source: https://www.achilles.com/features/uvdb-membership-pricing-and-features/]
- UVDB Silver+ (verified): from £800
- UVDB Verify (full audit): from ~£1,500+ depending on category
- Plus per-buyer-fee where the buyer pays for the audit; sometimes the buyer subsidises
[VERIFY: Achilles doesn't publish full pricing publicly; obtain quote.]

**5. Timeline**
Self-registration: 1-2 weeks.
Silver+: 4-8 weeks.
Verify audit: 8-12 weeks.

**6. Issuer**
Achilles Information Ltd directly.

**7. Application process**
Achilles portal questionnaire; documentation upload; verification calls; for Verify tier, an audit (remote or on-site).

**8. Prerequisites**
At self-registration tier, essentially none. At Silver+/Verify, similar evidence to Constructionline Silver: H&S, financial, insurance, quality, environmental.

**9. Renewal cycle**
Annual.

**10. Ongoing obligations**
Maintain documents; respond to verification cycles; respond to buyer-initiated questionnaires.

**11. Risk if absent**
LOW for Sorellon's current target buyer set (most local-authority and NHS cleaning / hygiene buyers don't use Achilles). HIGH if Sorellon ever targets utilities-sector PPE supply (Anglian Water, National Grid, Severn Trent, etc. all use UVDB) or housing-association cleaning frameworks using Connexus.

**12. Sequencing rank**
**TRIGGER-BASED (only if a utilities or Connexus tender appears).** Don't pre-buy.

**13. Sorellon-specific notes**
- UVDB is broad ("PPE for utilities operatives" is a relevant Sorellon-shaped supply line — utilities workforces need PPE).
- Self-registration at £145 is cheap if a UVDB-using buyer appears on the pipeline.
- Worksafe is the weakest of the three communities for Sorellon — skip.

---

### 12.10. JOSCAR (Hellios Information — Defence / Aerospace / Security)

**1. What it is**
Joint Supply Chain Accreditation Register. Run by Hellios Information. The standard pre-qualification register for defence, aerospace, and security supply chains in the UK. Used by all the major defence primes (BAE Systems, Babcock, MBDA, Leonardo, Thales UK, Rolls-Royce Defence, Airbus Defence, RTX/Raytheon UK, etc.) and MoD as a "single fill, many recipients" model. [Source: https://hellios.com/joscar]

**2. Mandatory?**
Effectively mandatory if Sorellon ever bids to MoD or defence-prime tier-1s. Most defence primes will not engage with a supplier not on JOSCAR.

**3. Eligibility**
Any UK supplier. Free SME Portal entry is open to all SMEs (basic self-declaration only).

**4. Cost (2026)**
- **SME Portal: FREE** — basic profile, self-declaration, limited information. Visible to defence buyers as a "potential supplier" but does NOT satisfy full JOSCAR requirements.
- **Full JOSCAR registration: fee-based**, typically a few hundred to low thousands per year depending on bands. ADS / Make UK Defence members (membership bands 1-3, turnover < £5m) get discounted rates.
[VERIFY: Hellios doesn't publish full JOSCAR fee table publicly — obtain quote.]

**5. Timeline**
SME Portal self-registration: 1 week.
Full JOSCAR: 4-12 weeks depending on stage requirements.

**6. Issuer**
Hellios Information Ltd directly.

**7. Application process**
Hellios portal questionnaire (extensive — security, financial, quality, H&S, modern slavery, anti-bribery, environmental, cyber, IP protection). For defence supply, additional security-control modules.

**8. Prerequisites**
At SME Portal: none.
At full registration: typically Cyber Essentials (CE+ for some defence tier-1s), ISO 9001 or equivalent, financial accounts.

**9. Renewal cycle**
Annual.

**10. Ongoing obligations**
Continuous data refresh; respond to buyer-triggered modules; maintain underlying certifications.

**11. Risk if absent**
LOW unless Sorellon targets defence supply. HIGH if Sorellon targets MoD or defence-prime cleaning / hygiene / PPE contracts (which DO exist — defence-base FM and PPE contracts).

**12. Sequencing rank**
**TRIGGER-BASED. SME Portal: free, register now (zero downside). Full JOSCAR: only if defence pipeline appears.**

**13. Sorellon-specific notes**
- Register the free SME Portal entry as a no-cost option in month 1. Costs nothing, makes Sorellon visible to defence buyer searches.
- Full JOSCAR registration is not justified by Sorellon's current pipeline scope. Revisit after the first 6 months of bid activity to see whether defence tenders are surfacing.
- ADS / Make UK Defence membership discount only matters if Sorellon joins one of those trade bodies (membership fees themselves are several hundred to low thousands).

---

### 12.11. EcoVadis (sustainability rating)

**1. What it is**
A sustainability rating platform (not a UK-government scheme). Assesses suppliers across four themes: environment, labour and human rights, ethics, sustainable procurement. Issues Bronze / Silver / Gold / Platinum medals based on percentile ranking. Widely required by tier-1 corporate procurement (NOT typically by UK public-sector procurement — there's overlap with Carbon Reduction Plan + Modern Slavery Statement instead). [Source: https://ecovadis.com/]

**2. Mandatory?**
Almost never in UK PUBLIC procurement. Often required by tier-1 corporate buyers and some defence primes who use EcoVadis as their sustainability gate.

**3. Eligibility**
Any organisation. No size minima but the questionnaire is calibrated to organisations of meaningful operational scale.

**4. Cost (2026)**
- Small businesses (26-99 employees): approximately $500-$1,200 USD/yr subscription
- Mid-sized (100-999): $1,000-$2,800 USD/yr
- INTERNAL EFFORT: 50-500+ person-hours to complete the assessment (the largest hidden cost) — at senior-staff rates often £5,000-£20,000 in effective internal cost. [Source: https://www.doubleyoursustainability.com/blog/true-cost-of-ecovadis]
- For Sorellon-sized organisations (1 employee) the EcoVadis questionnaire is poorly fit and the assessment process is likely to score poorly purely due to insufficient operational evidence.

**5. Timeline**
8-16 weeks for a first-time assessment.

**6. Issuer**
EcoVadis (private company, Paris).

**7. Application process**
Buyer-initiated invitation OR self-initiated subscription. Complete the online questionnaire across four themes; upload documentary evidence; EcoVadis assessors score; medal issued.

**8. Prerequisites**
Documented sustainability policies (Sorellon has these), reasonable operational maturity (Sorellon currently does not).

**9. Renewal cycle**
Annual re-assessment.

**10. Ongoing obligations**
Maintain subscription; respond to re-assessment cycles; address any improvement plans.

**11. Risk if absent**
LOW in UK public procurement. MEDIUM-HIGH if Sorellon ever targets tier-1 corporate buyers using EcoVadis as a procurement gate.

**12. Sequencing rank**
**SKIP for year 1-2.** Not relevant to Sorellon's UK public-procurement target. Revisit only if a specific corporate-supplier opportunity surfaces.

**13. Sorellon-specific notes**
- A pre-revenue 1-employee org will struggle to achieve a Bronze medal (top 35% percentile) because the questionnaire scores on operational evidence not just policies.
- The hidden internal-time cost is the killer. 50+ person-hours is roughly 1.5 weeks of full-time work for Zac.

---

### 12.12. SafeContractor / SMAS / Acclaim — SSIP schemes

**1. What it is**
SSIP (Safety Schemes In Procurement) is the umbrella body for ~30 health-and-safety assessment schemes that mutually recognise each other against the Common Assessment Standard. Member schemes include SafeContractor (Alcumus), SMAS Worksafe, Acclaim Accreditation (Constructionline's H&S brand), CHAS, Achilles BuildingConfidence, Constructionline Gold (incorporates Acclaim). For Sorellon's purpose any SSIP-member scheme provides "SSIP-accredited" status accepted by participating buyers. [Source: https://www.complys.co.uk/blog/chas-vs-safecontractor-vs-smas]

**2. Mandatory?**
Buyer-discretionary but common in local-authority, FM, NHS-affiliated, and construction-adjacent tenders. Where required, ANY SSIP-member scheme satisfies, though some buyers name a specific scheme (CHAS most often, SafeContractor second).

**3. Eligibility**
Any UK supplier. Sole traders and sole-director companies are explicitly catered for.

**4. Cost (2026) — annual fees for sole traders / micro businesses:**
- **SMAS Worksafe: from £195 + VAT/yr** (cheapest of the major SSIP schemes) [Source: https://www.complys.co.uk/blog/chas-vs-safecontractor-vs-smas]
- SafeContractor (Alcumus): higher, typically £300-£500+ depending on package
- CHAS: from £425 + VAT/yr typically
- Acclaim (via Constructionline Gold path): bundled into Constructionline fee
- Cross-band range across schemes for very small businesses: £99-£250 + VAT/yr typically. [Source: https://www.beaconrisk.co.uk/news/150-how-much-does-safecontractor-cost-and-is-it-worth-it]

**5. Timeline**
2-4 weeks from application to certificate.

**6. Issuer**
The chosen SSIP-member scheme (SMAS Ltd, Alcumus SafeContractor Ltd, CHAS Ltd, Acclaim Accreditation Ltd).

**7. Application process**
Online questionnaire against the SSIP Core Criteria approved by the HSE: H&S policy, training and competence, accident reporting, risk assessments, RIDDOR awareness, equipment safety, etc.

**8. Prerequisites**
- Written H&S policy (Sorellon has)
- Competent person H&S advice (sole-director companies can self-declare with appropriate H&S training certification OR outsource to a competent-person service)
- Risk assessments documented
- Insurance (Employers' Liability if applicable, Public Liability)

**9. Renewal cycle**
Annual.

**10. Ongoing obligations**
Maintain H&S documentation; update on changes; respond to verification cycles.

**11. Risk if absent**
MEDIUM. SSIP appears frequently as a selection-stage requirement, especially in cleaning, FM, and any tender with a physical-site delivery component. Absence can be defended via documented H&S evidence + the equivalence clause but at scoring cost.

**12. Sequencing rank**
**SECOND (alongside Constructionline Bronze).** SMAS is the cheapest entry point.

**13. Sorellon-specific notes**
- SMAS Worksafe at £195/yr is the lowest-cost SSIP entry. Acceptable for buyers asking for "SSIP" in general; occasionally rejected by buyers who specifically name "CHAS" or "SafeContractor" — but those buyers are a minority.
- For Sorellon as wholesale-only (no on-site work), the H&S evidence is light: office-based risk assessment, DSE assessment, manual-handling awareness, no specific contractor-on-site activities to assess.
- If Sorellon goes Constructionline Gold (which includes Acclaim SSIP), do NOT also buy SMAS — duplicative. Pick one path.

---

### 12.13. Living Wage Foundation accreditation

**1. What it is**
Recognition that the employer pays all directly-employed staff at least the real Living Wage (set annually by the Resolution Foundation, currently £13.85/hr inside London and £12.60 outside in 2026 [VERIFY: rates set in October, confirm current figures]). Accreditation is a signed licence between Living Wage Foundation and the employer. [Source: https://www.livingwage.org.uk/accreditation]

**2. Mandatory?**
Never mandatory. Often scored in social-value evaluation, particularly in local-authority tenders (some councils require or strongly score Living Wage Employer status).

**3. Eligibility**
Any UK employer with at least one directly-employed staff member paid at or above the real Living Wage. Sorellon currently pays only the director — director remuneration via salary above the Living Wage hourly equivalent qualifies. Coverage also needs to extend to third-party contracted staff (cleaners, security, etc.) — Sorellon has none of these so this is trivially satisfied.

**4. Cost (2026)**
Sliding scale by employee count:
- Under 10 employees: **from £71-£100/yr** (sources differ; £100 is the most commonly cited 2026 figure) [Source: https://www.livingwage.org.uk/faqs]
- Larger organisations scale up.

**5. Timeline**
~10 working days from application to accreditation, sometimes faster.

**6. Issuer**
Living Wage Foundation (Citizens UK).

**7. Application process**
Online enquiry form; complete an undertaking confirming all directly-employed staff and contracted-in third-party staff (if any) earn at least the real Living Wage; signed licence; published on the Living Wage Employer register.

**8. Prerequisites**
Pay all directly-employed staff at or above the current real Living Wage. For Sorellon's sole director, Zac's effective hourly rate must meet the threshold (a £25k+ annual salary at any reasonable working week clears this; below that, the math gets tight).

**9. Renewal cycle**
Annual recommitment; fee due annually.

**10. Ongoing obligations**
- Maintain Living Wage compliance for all directly-employed staff.
- Within 6 months of each annual Living Wage uplift (announced in early November each year), increase pay to the new rate.
- Apply Living Wage requirements to any future third-party contracted staff (cleaners, security, etc.).
- Display the accreditation mark.

**11. Risk if absent**
LOW for selection-stage. MEDIUM for social-value scoring in some councils (especially London boroughs, Manchester, Cardiff, Glasgow which all have aligned Living Wage Council frameworks).

**12. Sequencing rank**
**EARLY (month 1-2). Very low cost; high signalling value.**

**13. Sorellon-specific notes**
- Trivial to satisfy: Sorellon has 1 employee (Zac), no third-party contracted staff, presumably pays Zac above the threshold.
- Caveat: if Sorellon ever brings on a low-paid part-timer or contracts cleaning to a non-Living-Wage cleaner, accreditation must extend to cover them.
- £100/yr is the cheapest social-value signalling on the menu.

---

### 12.14. B Corp certification

**1. What it is**
B Lab's certification for "businesses that meet high standards of social and environmental performance, accountability, and transparency". Global scheme, UK affiliate is B Lab UK. The B Impact Assessment (BIA) scores across five impact areas: governance, workers, community, environment, customers. Minimum 80-point threshold (out of 200) for certification. New V2.1 standards rolling out from 2026 with structural changes including specific minimum requirements across pillars. [Source: https://bcorporation.uk/b-corp-certification/the-certification-process/pricing/]

**2. Mandatory?**
Never. Marketing / brand differentiation play. Some socially-motivated buyers (particularly in education and charity-sector procurement) score it positively.

**3. Eligibility**
- For-profit organisation.
- **Minimum 12 months of trading history for full certification** (Sorellon hits this in July 2026). [Source: https://www.allianz-trade.com/en_GB/insights/grow-your-business/what-is-a-b-corp-and-why-should-my-sme-become-one.html]
- "Pending B Corp" status is available for startups < 12 months as an interim recognition.
- Must adopt the B Corp legal-form commitment (UK companies: amend articles to lock-in stakeholder-governance obligations).

**4. Cost (2026)**
- B Impact Assessment: FREE to complete
- Submission fee: one-time, revenue-tiered
- Annual certification fee: revenue-tiered (lowest tier — under £150k revenue — historically ~£1,000/yr; mid-tier £150k-£1m ~£1,250-£2,000/yr) [VERIFY: B Lab UK's pricing table is image-based and not directly extractable; contact certification@bcorporation.uk for current SME pricing]
- 40% discount for companies under £5m revenue with 50%+ ownership by underrepresented groups
- **Fees FROZEN through 2026** to support V2.1 transition
- INTERNAL EFFORT: 60-200 person-hours typical to complete the BIA

**5. Timeline**
6-18 months end-to-end. The BIA itself can be completed in weeks; the verification process is queue-bound at B Lab UK and currently runs months.

**6. Issuer**
B Lab UK (UK affiliate of B Lab Global).

**7. Application process**
(a) Complete the B Impact Assessment online. (b) Score 80+. (c) Submit for verification. (d) Pay submission fee. (e) Verification calls with B Lab UK. (f) Legal commitment via articles amendment. (g) Annual fee. (h) Certificate issued and published on the B Corp directory.

**8. Prerequisites**
- 12 months trading history (or apply for Pending B Corp).
- Legal-form change in articles of association.
- 80+ BIA score (most first-time SMEs score 50-70 without targeted work).

**9. Renewal cycle**
Recertification every 3 years under new V2.1 standards.

**10. Ongoing obligations**
- Maintain stakeholder-governance language in articles.
- Recertify against latest standards.
- Pay annual fee.
- Submit to ongoing-improvement expectations.

**11. Risk if absent**
LOW for UK public procurement. Not a frequent selection-stage criterion.

**12. Sequencing rank**
**LATER (year 2-3 if at all).** Disproportionate effort for selection-stage benefit in Sorellon's space.

**13. Sorellon-specific notes**
- Brand-signalling tool more than procurement gate.
- If Sorellon ever targets the education-sector cleaning / hygiene supply space (where B Corp scoring is more common), revisit.
- The legal-form commitment is meaningful — amending articles binds the company to a stakeholder-governance model. Worth considering whether this aligns with Sorellon's future investor / exit options.

---

### 12.15. Disability Confident (employer scheme)

**1. What it is**
DWP-administered scheme recognising employers committed to recruiting, retaining, and developing disabled employees. Three levels:
- **Level 1: Committed** (self-declaration; entry tier)
- **Level 2: Employer** (self-assessment against two themes; valid for 3 years — reducing to 2 years from December 2026)
- **Level 3: Leader** (Level 2 + external validation + employs disabled people)
[Source: https://www.gov.uk/government/publications/disability-confident-guidance-for-levels-1-2-and-3]

**2. Mandatory?**
Never mandatory in UK public procurement, but scored in social-value evaluation in many tenders.

**3. Eligibility**
Any UK employer of any size. Sole-employee companies can sign up at Level 1; Level 2 self-assessment is feasible with documented commitments even with one employee; Level 3 requires actually employing disabled people, so not viable for Sorellon at sole-employee stage.

**4. Cost (2026)**
- **Levels 1-3: FREE** (no fees) — DWP-administered.
- Internal effort: Level 1 is minutes (online declaration); Level 2 is a few hours self-assessment; Level 3 requires external validation effort.

**5. Timeline**
Level 1: same day.
Level 2: 1-2 weeks.
Level 3: variable; depends on validator availability.

**6. Issuer**
UK Department for Work and Pensions (DWP), via the Disability Confident portal.

**7. Application process**
Online registration; signed undertaking; choose level; for Level 2, complete the self-assessment; for Level 3, secure external validation.

**8. Prerequisites**
Level 1: agree commitments + identify one action.
Level 2: meet the two-theme self-assessment criteria.
Level 3: actively employ disabled people + external validation.

**9. Renewal cycle**
Level 1: until December 2026, valid 3 years; from late 2026 onwards, validity reducing to 2 years and renewal at Level 1 being removed — progression to Level 2 required. Level 2: 3 years (reducing). Level 3: 3 years.

**10. Ongoing obligations**
Maintain the commitments; deliver on the declared actions; progress through levels as required by the post-2026 overhaul.

**11. Risk if absent**
LOW for selection-stage. LOW-MEDIUM for social-value scoring (3-10% scoring uplift in many tenders for Disability Confident Employer status).

**12. Sequencing rank**
**EARLY (month 1-2).** Free, fast, signalling value.

**13. Sorellon-specific notes**
- Level 1 is achievable today at zero cost.
- Level 2 is achievable with the existing Equality & Diversity policy + commitment to disability-inclusive recruitment language — also zero cost.
- Level 3 requires actually employing disabled people, which is a hiring-dependent future state.
- The DWP overhaul announced for completion by December 2026 means Level 1 will be a time-bound entry only — plan to progress to Level 2 within 24 months.

---

### 12.X. Recommended certification sequencing for Sorellon

**Framing of the recommendation:**
- Sorellon is pre-revenue with a single employee and a residential office. Many certifications are technically achievable but commercially uneconomic at this scale.
- The right strategy is a **layered, trigger-based** approach: do the cheap-and-fast items immediately to unblock the broadest range of public-sector bids; defer expensive items until a specific tender justifies them.
- Total year-1 spend on certification (recommended bundle) is approximately **£600-£900 + VAT**, plus modest internal time.

**MONTH 1-2 (immediate, ~£550 + VAT total):**
1. **Cyber Essentials (basic)** — £330 + VAT direct to IASME. Unlocks the broadest pre-qualification gates. Highest single-bid-unlock per pound. **Critical-path item.**
2. **Disability Confident Level 1** — FREE. Signed self-declaration. Same day.
3. **Living Wage Foundation accreditation** — ~£100/yr. Confirms Zac's pay above threshold; small social-value signalling.
4. **Written "Quality Procedure" document** (Section 6.5) — free; Zac's time. Pre-emptively answers ISO-9001-or-equivalent SQ questions. Critical for tender response.
5. **JOSCAR SME Portal free registration** — FREE. Zero downside, visibility-to-defence-buyers upside.

**MONTH 2-4 (after basic foundations, ~£444 + VAT):**
6. **Constructionline Bronze** — from £249/yr. Achievable with current evidence + Cyber Essentials. Unlocks ~3,500-buyer registry visibility.
7. **SMAS Worksafe (SSIP)** — £195 + VAT/yr. Cheapest SSIP entry. Covers H&S pre-qualification for the bulk of relevant buyers.

**MONTH 3-4 (escalation as evidence matures, optional):**
8. **Disability Confident Level 2** — FREE. Self-assessment upgrade once policy and commitments documented in operational shape.

**MONTH 4-8 (trigger-based, if bid pipeline justifies):**
9. **Cyber Essentials Plus** — ~£1,830 + VAT all-in. Only if a target tender requires it (defence, MoD, NHS Supply Chain, OFFICIAL-SENSITIVE).
10. **Constructionline Silver** — variable fee, ~£400-£700+. Only after 6+ months of real trading evidence to satisfy financial verification, and only if buyers in the pipeline require Silver+.
11. **Achilles UVDB self-registration** — from £145. Only if a utilities-sector tender appears.

**MONTH 6-12 (after first revenue, optional):**
12. **ISO 9001 (UKAS-accredited)** — £6,000-£7,000 all-in. Only when (a) Sorellon has 6+ months of real trading volume to audit, AND (b) ISO 9001 is consistently appearing as a selection-stage blocker. Use NQA or BSI; never non-UKAS.

**YEAR 2-3 (deferred or selective):**
13. **ISO 14001** — £4,000-£5,000 add-on to ISO 9001. Only if bid pipeline shows environmental scoring is materially affecting losses.
14. **B Corp** — only if brand positioning genuinely calls for it. Not procurement-driven.
15. **Constructionline Gold** — only if Gold-specific tenders surface. Requires ISO 9001 or equivalent.

**NEVER (for Sorellon's current profile):**
- **ISO 27001** in year 1 — disproportionate cost; defensible via CE+ + written InfoSec policy + equivalence claim.
- **ISO 45001** — sole-employee wholesale has no meaningful H&S operational scope to audit.
- **Constructionline Platinum** — requires on-premises audit, not feasible from residential apartment.
- **EcoVadis** — wrong scheme for UK public procurement; subscription + internal effort uneconomic.
- **Non-UKAS ISO certificates** — universally false economy; central-gov buyers reject; reputational risk if discovered.

**Total recommended year-1 spend (items 1-7):**
- IASME Cyber Essentials: £330 + VAT
- Living Wage Foundation: ~£100
- Constructionline Bronze: from £249
- SMAS Worksafe: £195 + VAT
- (Items 2, 4, 5, 8 are free)
- **Subtotal: ~£874 + VAT (~£1,049 inc. VAT)**

**Year-1 trigger-based add-ons (only if pipeline justifies):**
- Cyber Essentials Plus: +£1,830 inc. VAT (only if needed)
- Constructionline Silver upgrade: +£200-£500/yr (only if needed)
- Achilles UVDB: +£174 inc. VAT (only if utilities tender appears)

**Critical decision points (next 90 days):**
1. Cyber Essentials — book within 30 days. Single highest-ROI item.
2. Disability Confident Level 1 + Living Wage Foundation + Quality Procedure document — all within 60 days. Combined cost <£200 + Zac's time.
3. JOSCAR SME Portal — 30 days. Free option.
4. Constructionline Bronze + SMAS — by month 4. The "main SSIP + main supplier register" pair.

**Key strategic notes for Sorellon:**
- Buyer-driven, not pre-emptive: track ACTUAL selection-stage requirements in the tenders being pursued. If a category of certification never appears in the pipeline, don't buy it.
- UKAS-only on every ISO: cheap non-UKAS certificates are a trap. Bid evaluators do check.
- Pre-revenue financial verification (Dun & Bradstreet, Achilles, Constructionline Silver+) will be a recurring problem until Sorellon files its first set of accounts. Plan for Director's Personal Guarantee or letter-of-credit alternatives as compensating evidence in tender responses.
- Sole-employee H&S has a low burden — SSIP via SMAS is sufficient. Don't over-buy.
- The biggest leverage in year 1 is **writing things down well** (the Quality Procedure, Disability Confident commitments, Living Wage attestation) — not paying for badges.

---

[VERIFY: Final pricing for all certifications should be confirmed via direct quote (especially for Constructionline Silver pricing, full JOSCAR pricing, Achilles UVDB Silver+ pricing, EcoVadis SME tier, and current B Lab UK fee table) before commitment. Costs listed are 2026 indicative figures cross-checked across 2-3 independent sources but vendor pricing pages should be the authoritative reference.]

[VERIFY: PPN 014's interaction with the Procurement Act 2023's equivalence clause — current 2026 practice is that buyers list Cyber Essentials by name but must accept equivalents; check that this remains the operative position before relying on ISO 27001 as a substitute.]

[VERIFY: April 2026 Cyber Essentials "Danzell" question set changes — Sorellon should book under the current Montpellier question set if applying before 26 April 2026; thereafter use Danzell. Confirm with IASME's latest guidance.]

[VERIFY: Real Living Wage rates for 2026 (announced annually in early November). Confirm Zac's effective hourly rate exceeds the inside-London rate before applying.]

[VERIFY: Disability Confident scheme overhaul completion by December 2026 — confirm what Level 1 lifetime / renewal rules will be after the changes.]

---

## 13. Procurement Platform Registrations

### 13.1. Procurement Act 2023 — the new UK public procurement regime

#### 7.1.1. What it is
The Procurement Act 2023 is the post-Brexit replacement for the Public Contracts Regulations 2015 (PCR 2015) plus three sister regulation sets. It rewrites how UK central government, NHS, local authorities, and other "contracting authorities" buy goods and services from suppliers. For a reseller like Sorellon, it is the *over-arching* rulebook every other procurement registration sits within.

#### 7.1.2. Legal basis
- Primary statute: **Procurement Act 2023** (c.54), Royal Assent 26 October 2023. [Source: https://www.legislation.gov.uk/ukpga/2023/54/contents]
- Commenced 24 February 2025 by the Procurement Act 2023 (Commencement No.3) Regulations 2024. [Source: https://www.bevanbrittan.com/insights/articles/2024/the-procurement-act-2023-now-due-to-take-effect-24-february-2025/]
- Further provisions — section 69 (payments compliance notices), section 71 (contract performance assessment) — came into force 1 January 2026; section 70 (information about payments) came into force 1 April 2026. [Source: https://www.hoganlovells.com/en/publications/uk-procurement-law-further-procurement-act-2023-provisions-in-force-1-january-2026-and-1-april-2026]
- The Procurement Regulations 2024 (secondary legislation, SI 2024/692) supply operational detail.

#### 7.1.3. Who is in scope
Any supplier (including resellers) wanting to bid for "covered procurement" (i.e. contracts at or above the relevant threshold — £138,760 goods/services for central government; £213,477 sub-central; PSR also applies to "regulated below-threshold" contracts above £12,000 central / £30,000 sub-central). Sorellon is in scope from the moment it submits its first tender.

#### 7.1.4. Sorellon-specific exposure
- **Cleaning chemicals** to local authorities, schools, social housing landlords — sub-central threshold £213,477 applies, but "notifiable below-threshold" rules kick in from £30,000.
- **PPE / gloves** to NHS Trusts — central-government rules including £12,000 threshold.
- **Janitorial supplies** to academy trusts — treated as central government under PA 2023.

#### 7.1.5. Registration / onboarding mechanics
- Register on the **Central Digital Platform** (delivered through Find a Tender — https://www.find-tender.service.gov.uk). [Source: https://www.gov.uk/government/publications/procurement-act-2023-short-guides/the-procurement-act-2023-a-short-guide-for-suppliers-html]
- Procedure: create a personal **GOV.UK One Login** account (per team member, not per company), then designate an administrator who registers the organisation in "My Organisations → Register an organisation". [Source: https://www.gov.uk/government/publications/procurement-act-2023-short-guides/suppliers-how-to-register-your-organisation-and-first-administrator-on-find-a-tender-in-three-easy-steps-html]
- Free.

#### 7.1.6. Documents to prepare
- Companies House registration number: **16575318**.
- Two years of financial accounts (if available — Sorellon was incorporated 2025-07-10 so will only have one filing by Sep 2026 at earliest; the platform accepts "not available" for new companies but expect buyers to ask follow-up questions).
- Connected persons / PSCs: Zac Farrell (director), Dieter Xia Shuang Danicek (PSC, co-PSC).
- Postal address, email, website.
- Information on any criminal convictions or offences — Sorellon's principals should disclose ANY relevant convictions (theft, fraud, modern slavery offences, corporate manslaughter, competition law breaches).
- VAT number (if registered — `[VERIFY: Sorellon's VAT status, may not be registered yet pre-revenue]`).

#### 7.1.7. Recurring obligations
- Keep supplier information up to date on the CDP (it is *re-used* across procurements — out-of-date info travels with you).
- Notify changes in PSC / conviction status promptly.
- From 1 Jan 2026: comply with payments-compliance-notice regime if subcontracting (PA 2023 s.69, s.71).
- From 1 Apr 2026: publish information about payments under public contracts (s.70).

#### 7.1.8. Penalties for non-compliance
- **Exclusion** (specific procurement-by-procurement decision by the buyer).
- **Debarment** — appearance on the central debarment list maintained by a Minister, preventing bidding across all UK public procurement. Mandatory grounds now include theft, corporate manslaughter, modern slavery, organised-crime convictions, and **cartel/competition law infringement** (new). [Source: https://www.tussell.com/insights/what-is-supplier-debarment-procurement-act-2023]
- Information given on CDP that is materially false → automatic mandatory exclusion ground and potential criminal offence under the Fraud Act 2006.
- "Self-cleaning" defence available — if Sorellon demonstrates the circumstances giving rise to exclusion are not continuing or unlikely to recur, exclusion can be lifted.

#### 7.1.9. Interaction with other regimes
- The CDP feeds Find a Tender, Contracts Finder (during transition), and devolved platforms (some of which now require CDP unique identifier first — see 7.6).
- Triggers downstream contractual obligations: pay subcontractors within 30 days; report on contract performance against KPIs.

#### 7.1.10. Reseller carve-outs
- Sorellon does NOT need to prove manufacturing capability — but contracting authorities can still set "conditions of participation" that effectively require it (e.g. ability to source within a UK-only supply chain). Resellers can satisfy these via supplier letters of intent.
- For lots reserved for SMEs, Sorellon (very small, pre-revenue) actually has an *advantage*.

#### 7.1.11. Practical pitfalls
- **Old PCR 2015 procurements** running over the transition: contracts started before 24 Feb 2025 continue under old rules until completion. Some Sorellon target frameworks may still use legacy processes.
- **The Find a Tender "old account" issue**: existing users must re-register; only saved searches migrate. Sorellon (new entrant) is unaffected.
- **Companies House discrepancies** are a common rejection reason — ensure CDP name matches Companies House *exactly* ("Sorellon Limited" — note canonical double-L spelling).
- **"Connected persons" disclosure** is broader than UK PSC rules — includes anyone with material control or influence.

#### 7.1.12. Verification flags
- `[VERIFY: Sorellon's two-year financial history requirement — pre-revenue new entity may need parental/director financial guarantees on framework applications]`.
- `[VERIFY: whether any criminal conviction history for Zac Farrell or Dieter Xia Shuang Danicek needs disclosure]`.
- `[VERIFY: current Procurement Act 2023 transition rollout state as of May 2026 — section 69-71 in force; check for further commencement orders]`.

#### 7.1.13. Recommended action sequence
1. Each Sorellon principal creates GOV.UK One Login (personal, not corporate).
2. Designate administrator (recommend Dieter — the operator).
3. Register Sorellon Limited on Find a Tender via administrator account.
4. Complete "Supplier Information" — core company details and financial standing self-assessment.
5. Tag relevant CPV codes (see Section 7.4 for list).

---

### 13.2. Central Digital Platform (CDP) — supplier core information service

#### 7.2.1. What it is
The CDP is the *single* place where suppliers store reusable core business information for UK public procurement. It is delivered through Find a Tender (FTS) — not a separate URL, but rather an enhanced FTS with new "supplier information service" capability. It replaces the old Cabinet Office Supplier Registration Service (supplierregistration.cabinetoffice.gov.uk) for most purposes, but that legacy service is **still live** for some Crown Commercial Service DPS access. [Source: https://supplierregistration.cabinetoffice.gov.uk/]

#### 7.2.2. Legal basis
- Procurement Act 2023, ss. 95–98 (information service).
- Procurement Regulations 2024 specify data fields.

#### 7.2.3. Who is in scope
Every supplier that wants to bid above a regulated threshold. From **1 April 2026**, suppliers awarded *below-threshold* central-government contracts also need a CDP **unique supplier identifier** ("supplier ID"). [Source: https://www.tendersgo.com/post/united-kingdom-procurement-act-2023-new-rules-thresholds-cdp-registrat-6317]

#### 7.2.4. Sorellon-specific exposure
- Many of Sorellon's likely first wins will be **below-threshold** (e.g. school cleaning chemicals for an academy at £8,000–£15,000) — these now require a CDP ID from April 2026.
- Larger NHS / local authority opportunities require CDP info reused at tender submission.

#### 7.2.5. Registration / onboarding mechanics
- URL: https://www.find-tender.service.gov.uk → "Register an organisation".
- Identifiers: Companies House number (16575318) + a unique CDP-issued ID, which is what buyers will request in tenders.
- Cost: free.

#### 7.2.6. Documents to prepare
- All items in 7.1.6.
- **PAS-91-equivalent answers (now: Common Assessment Standard or contractor-side equivalent)** — see 7.3.
- Insurance certificates: Public Liability (typically £5M minimum for cleaning/PPE), Product Liability (typically £5M; HIGHER for medical-device-classed gloves and biocidal wipes — recommend £10M), Employers' Liability (statutory £5M minimum if any employees). `[VERIFY: Sorellon's current insurance limits — pre-revenue means likely entry-level policies]`.
- Quality management certifications (ISO 9001 is commonly asked) — Sorellon as a 10-month-old reseller almost certainly lacks ISO; the workaround is a written quality plan referencing manufacturer ISO certificates.
- Health & safety policy (mandatory if 5+ employees — small new entity may not need formal policy yet).
- Modern Slavery statement — required for companies with turnover > £36M; not Sorellon's exposure yet, but the question is asked routinely.

#### 7.2.7. Recurring obligations
- Update core info when material changes occur (new director, new PSC, new accounts filed, change of address, new conviction).
- Annual review of insurance certificates (auto-prompt by platform).

#### 7.2.8. Penalties for non-compliance
- Tenders rejected if mandatory CDP info missing.
- "Materially false" submissions → mandatory exclusion + Fraud Act 2006 exposure.

#### 7.2.9. Interaction with other regimes
- The CDP is now the *upstream* of Sell2Wales registration ("new suppliers must first register on the Central Digital Platform and obtain their Unique Identifier (UI) before registering on Sell2Wales"). [Source: https://www.sell2wales.gov.wales/]
- Public Contracts Scotland is in a transition state — Scotland has its own PCS portal but is aligning with PA 2023 for cross-border buyers.
- NHS Supply Chain's Atamis platform will consume CDP IDs going forward.

#### 7.2.10. Reseller carve-outs
- None — CDP applies equally to manufacturers, distributors, agents.

#### 7.2.11. Practical pitfalls
- **Accidentally completing it as a sole-trader entity** when you should be the limited company (or vice versa) — leads to mismatch with Companies House.
- **Email mismatches** between One Login email and corporate email — One Login uses personal accounts, the corporate registration uses a shared inbox. Both can desync.
- **Bid administrator turnover** — if the only registered admin leaves, recovery requires Cabinet Office support.

#### 7.2.12. Verification flags
- `[VERIFY: actual CDP rollout status as of May 2026 — initial rollout was rocky per industry reports; check for current advisory updates]`.
- `[VERIFY: whether CDP unique identifier is mandatory in below-threshold tenders Sorellon currently sees in market]`.

#### 7.2.13. Recommended action sequence
1. Complete 7.1.13 first (creates CDP account).
2. Add at least one backup administrator (resilience).
3. Populate ALL optional fields — buyers may filter on "fully populated" suppliers.
4. Save CDP unique ID to internal CRM — quote in every future tender.

---

### 13.3. Common Assessment Standard (CAS) — successor to PAS 91

#### 7.3.1. What it is
PAS 91 was the British Standards Institution's pre-qualification questionnaire for construction supply chains. It was withdrawn by BSI in **April 2023**. The **Common Assessment Standard (CAS)** is now the accepted UK pre-qualification standard, mandated for central-government construction work under **PPN 03/24**. [Source: https://www.chas.co.uk/blog/public-sector-procurement-now-specifies-the-common-assessment-standard/]

#### 7.3.2. Legal basis
- Standard owned and maintained by **Build UK** (industry body).
- Mandated for central-government construction procurement via Procurement Policy Note (PPN) 03/24.
- Not statutory — but contracting authorities can make it a "condition of participation" under PA 2023.

#### 7.3.3. Who is in scope
- Primarily **construction-related** suppliers — contractors, subcontractors, suppliers of construction-related goods (which can include some Sorellon products like maintenance kits, paints, certain PPE).
- Secondarily: when local authorities adopt CAS for their **whole supply chain** (some now require CAS-equivalent answers from cleaning suppliers too).

#### 7.3.4. Sorellon-specific exposure
- **LOW-MEDIUM**. Sorellon's core (cleaning, hygiene wipes, healthcare consumables) is largely outside CAS scope. However:
  - **Painting & maintenance kits** = construction-adjacent. Some councils require CAS or SSIP certification for paint suppliers.
  - **PPE supply to construction sites** (selling PPE to a Tier 1 building contractor) = often requires CAS or SSIP certification.
  - **Facilities supplies** to housing associations doing maintenance = sometimes asked.

#### 7.3.5. Registration / onboarding mechanics
- Cannot register directly with CAS; instead, register with one of the accredited assessment bodies that certifies against CAS:
  - **CHAS** (https://www.chas.co.uk) — most widely recognised.
  - **Constructionline** (https://www.constructionline.co.uk).
  - **Achilles BuildingConfidence** (https://www.achilles.com).
  - **Builders' Profile**.
  - **SMAS Worksafe**.
- Cost: tiered by company turnover; for small new entities typically £200–£500 annually for the entry-level CAS assessment.
- Process: complete an online questionnaire across 13 risk areas, upload evidence (insurance, H&S policy, quality processes, environmental policy, modern slavery, GDPR, financial standing, equality, building safety competence).

#### 7.3.6. Documents to prepare
- All items in 7.1.6 / 7.2.6.
- Health & Safety policy (mandatory).
- Environmental policy.
- Modern Slavery statement (or written confirmation of approach if under £36M).
- Equality / diversity policy.
- GDPR / data protection statement.
- Quality management plan (in lieu of ISO 9001).
- Insurance evidence (PL £5M+, EL £5M if employees, PI £1M+ if any consultancy element).
- Building Safety Act competence statement (only if work is in scope of higher-risk buildings).

#### 7.3.7. Recurring obligations
- Annual re-assessment.
- Maintain SSIP H&S accreditation if claiming Section 4 exemption.

#### 7.3.8. Penalties for non-compliance
- Loss of CAS certificate → loss of bid eligibility on procurements that mandate CAS.
- No statutory penalty per se.

#### 7.3.9. Interaction with other regimes
- Counts towards SSIP (Safety Schemes In Procurement) mutual recognition for the H&S section.
- CAS evidence can be re-used in CDP "supplier information" answers.

#### 7.3.10. Reseller carve-outs
- A pure distributor (no on-site work, no employees in client premises) has a streamlined CAS — Section 6 (site-specific controls) often N/A.

#### 7.3.11. Practical pitfalls
- Multiple assessment bodies = price/effort competition; **don't pay for both CHAS and Constructionline** unless explicitly required.
- Building Safety Act competence questions added in 2023 → if Sorellon supplies *any* paint or PPE used in higher-risk buildings (HRB), expect harder questions.
- **Two-year financial history** is the same blocker as 7.1.6 — pre-revenue entities should be ready to disclose director's financial standing.

#### 7.3.12. Verification flags
- `[VERIFY: which assessment body is most accepted by Sorellon's target buyers — survey 5–10 likely target contracts and check what they require]`.
- `[VERIFY: whether SMR (small-and-medium reseller) discount available with CHAS/Constructionline for new entrants]`.

#### 7.3.13. Recommended action sequence
1. **Defer** unless a target tender explicitly requires CAS — most cleaning/hygiene/PPE tenders for Sorellon will NOT need it.
2. If construction-adjacent tender appears: get a CHAS Elite quote first (most widely accepted).
3. Plan annual re-cert into operations calendar.

---

### 13.4. Find a Tender Service (FTS) — the new "above-threshold" notice board

#### 7.4.1. What it is
**Find a Tender (FTS)** is the UK government's mandatory publication service for *above-threshold* procurement notices under PA 2023. It also hosts the Central Digital Platform supplier information service. From April 2025 onwards, all "regulated" notices must appear here. URL: https://www.find-tender.service.gov.uk. [Source: https://www.find-tender.service.gov.uk/Search]

#### 7.4.2. Legal basis
- Procurement Act 2023, s. 95.
- Pre-PA 2023 it was the UK equivalent of OJEU/TED post-Brexit; PA 2023 deepens its role.

#### 7.4.3. Who is in scope
Every supplier wanting to see above-threshold UK public contracts, AND most "regulated below-threshold" notices (£12k central / £30k sub-central).

#### 7.4.4. Sorellon-specific exposure
- **PRIMARY** discovery channel. Every relevant cleaning, PPE, hygiene, healthcare consumables, education, facilities supplies contract from central government or sub-central authorities above £30k must publish here.

#### 7.4.5. Registration / onboarding mechanics
- See 7.1.13 / 7.2.13 — Find a Tender registration is the *same act* as CDP registration.
- Additional setup: **saved searches with email alerts** — keyword-based notifications. Critical for Sorellon's discovery workflow.
- CPV codes Sorellon should subscribe to (illustrative — `[VERIFY: full CPV mapping with CPV 2008 codes used by UK]`):
  - 33700000 Personal care products
  - 33741000 Hand care products
  - 33772000 Disposable paper products
  - 39800000 Cleaning and polishing products
  - 39830000 Cleaning products
  - 33140000 Medical consumables
  - 33141420 Surgical gloves
  - 33141119 Compresses (basic dressings)
  - 33190000 Miscellaneous medical devices and products
  - 18424000 Gloves (work)
  - 35113400 Protective and safety clothing
  - 19520000 Plastic products (for plastic packaging-adjacent buys)
  - 30192700 Stationery (education)

#### 7.4.6. Documents to prepare
- (Discovery only; tender response docs handled per-procurement.)
- A "tender library" of pre-written answers to common pre-qualification questions (technical capability, financial standing, contract examples) speeds up response.

#### 7.4.7. Recurring obligations
- Update saved searches quarterly.
- Respond to clarifications via the platform (NOT via email outside platform — that risks bid invalidation).

#### 7.4.8. Penalties for non-compliance
- N/A for discovery — but late or platform-bypassing tenders are routinely rejected.

#### 7.4.9. Interaction with other regimes
- FTS feeds (or is fed by) Contracts Finder during transition — see 7.5.
- Devolved nation portals cross-publish above-threshold notices to FTS.

#### 7.4.10. Reseller carve-outs
- None. Discovery channel is identical for all suppliers.

#### 7.4.11. Practical pitfalls
- **Over-broad alerts** (e.g. "cleaning") produce hundreds of irrelevant results from facilities cleaning *services* (not products) — combine with NUTS region filters and value bands.
- **"Pipeline notices"** (forward-look plans) are easy to miss — they preview future tenders by 6–12 months and are a competitive advantage for early supplier outreach.

#### 7.4.12. Verification flags
- `[VERIFY: current FTS UI as of May 2026 — UI changes have happened post-launch]`.
- `[VERIFY: whether CPV 2008 codes still in use or replaced by a successor code system under PA 2023]`.

#### 7.4.13. Recommended action sequence
1. Complete CDP registration (7.1.13).
2. Configure 6–10 saved searches covering Sorellon's CPV map.
3. Set email frequency to daily.
4. Triage first 30 days of results to refine.

---

### 13.5. Contracts Finder — below-threshold notices (transitioning)

#### 7.5.1. What it is
**Contracts Finder** (https://www.gov.uk/contracts-finder) historically published below-threshold central-government opportunities (£10k+) and sub-central (£25k+). Under PA 2023, its role has been transformed: below-threshold notices must now go on the CDP/FTS first. [Source: https://www.gov.uk/contracts-finder]

#### 7.5.2. Legal basis
- Pre-PA 2023: Public Contracts Regulations 2015, regulation 110.
- Post-PA 2023: Notifiable below-threshold contracts → CDP. Contracts Finder continues operating in a reduced role.

#### 7.5.3. Who is in scope
Suppliers tracking below-threshold work (£12k–£213k range), small contracts, and legacy PCR 2015 procurements.

#### 7.5.4. Sorellon-specific exposure
- **HIGH for early wins**. Sorellon's first realistic public-sector contracts will be small (£500–£20,000 direct purchases by schools, GP practices, social housing units, parish councils). These currently appear on Contracts Finder *and* the CDP.

#### 7.5.5. Registration / onboarding mechanics
- URL: https://www.gov.uk/contracts-finder.
- Free to view notices without registration.
- Account for saved searches: legacy email-based registration.

#### 7.5.6. Documents to prepare
- N/A for discovery.

#### 7.5.7. Recurring obligations
- N/A.

#### 7.5.8. Penalties for non-compliance
- N/A for discovery.

#### 7.5.9. Interaction with other regimes
- Below-threshold tender notices must be published on the CDP *before* publication elsewhere — Contracts Finder typically cross-publishes shortly after. [Source: https://www.gov.uk/government/publications/procurement-act-2023-guidance-documents-define-phase/guidance-below-threshold-contracts-html]
- Government has signalled Contracts Finder will eventually be consolidated into a single portal — `[VERIFY: current consolidation roadmap status May 2026]`.

#### 7.5.10. Reseller carve-outs
- None.

#### 7.5.11. Practical pitfalls
- **Duplicate alerts** if both Contracts Finder and FTS alerts are configured.
- **Contracts Finder filters are weaker** than FTS — more noise.

#### 7.5.12. Verification flags
- `[VERIFY: Contracts Finder retirement schedule — was supposed to fold into single portal but unclear by May 2026]`.

#### 7.5.13. Recommended action sequence
1. Maintain Contracts Finder saved searches as a *redundancy* channel against missed FTS alerts.
2. Plan migration off Contracts Finder when consolidation is finalised.

---

### 13.6. Devolved nation procurement platforms — Public Contracts Scotland, Sell2Wales, eTendersNI

#### 7.6.1. What it is
Each devolved UK nation runs its own procurement notice platform, partially overlapping with FTS:
- **Public Contracts Scotland (PCS)** — https://www.publiccontractsscotland.gov.uk.
- **Sell2Wales** — https://www.sell2wales.gov.wales.
- **eTendersNI** — https://etendersni.gov.uk (or https://etenders.com.). [Source: https://psip.co.uk/blog/uk-public-sector-procurement-portals]

#### 7.6.2. Legal basis
- Scotland: Public Contracts (Scotland) Regulations 2015 + Procurement Reform (Scotland) Act 2014 — **PA 2023 does NOT apply to Scotland**, which retains its own regime.
- Wales: Devolved Welsh Authorities operate under PA 2023 with Welsh modifications, but Sell2Wales remains the primary discovery channel.
- Northern Ireland: NI is in a transitional state; PA 2023 has limited NI applicability.

#### 7.6.3. Who is in scope
Suppliers wanting to bid in Scotland / Wales / NI.

#### 7.6.4. Sorellon-specific exposure
- **Wales**: significant cleaning/PPE contracts from NHS Wales, Welsh local authorities, schools.
- **Scotland**: NHS Scotland, Scottish Government, councils — sizeable hygiene and facilities supplies market.
- **NI**: smaller market but Department of Health and Education NI buy via eTendersNI.

#### 7.6.5. Registration / onboarding mechanics
- **PCS (Scotland)**: register at https://www.publiccontractsscotland.gov.uk/Register/Register_Supplier.aspx. Free. SC company numbers accepted; English-registered firms can register with Companies House number 16575318.
- **Sell2Wales**: requires CDP unique identifier *first* (new policy under PA 2023). Then register at https://www.sell2wales.gov.wales/Register_V2/Supplier_Register.aspx. Free, ~20 minutes.
- **eTendersNI**: requires a **D-U-N-S number** (Dun & Bradstreet). Free to obtain at https://www.dnb.com/duns-number/get-a-duns.html.

#### 7.6.6. Documents to prepare
- Same as 7.1.6 plus D-U-N-S for NI.

#### 7.6.7. Recurring obligations
- Each platform requires periodic profile refresh (typically annual or on company changes).
- Sell2Wales: keep CDP ID synced.

#### 7.6.8. Penalties for non-compliance
- Tender rejection.

#### 7.6.9. Interaction with other regimes
- Many devolved tenders cross-publish above-threshold notices to FTS — registering on FTS catches them, but **awarding remains via the devolved platform**, so you must register on it to bid.
- Scotland's PCS-Tender system is the more sophisticated procurement workflow tool on top of PCS — separate registration may be required for some buyers.

#### 7.6.10. Reseller carve-outs
- None.

#### 7.6.11. Practical pitfalls
- **NI D-U-N-S delay** — D&B can take 30 days to issue; register early.
- **Bilingual Welsh requirements** — some Welsh tenders require bilingual product labels. Sorellon's English-only label stock may be insufficient.
- **Scotland-specific PQQ** — Single Procurement Document (Scotland) [SPD(S)] differs from rest-of-UK Standard Selection Questionnaire (SSQ).

#### 7.6.12. Verification flags
- `[VERIFY: current Welsh-language labeling requirements for cleaning/PPE products sold to Welsh public sector]`.
- `[VERIFY: PCS-Scotland Procurement Act 2023 interaction — Scotland has not adopted PA 2023, so some "supplier reuse" features will not work]`.

#### 7.6.13. Recommended action sequence
1. Obtain D-U-N-S number now (long lead time).
2. Register PCS (Scotland) — easiest, no prerequisites.
3. After CDP registration completes, register Sell2Wales.
4. Register eTendersNI once D-U-N-S issued.

---

### 13.7. Crown Commercial Service (now Government Commercial Agency — GCA) frameworks

#### 7.7.1. What it is
Crown Commercial Service (CCS) was renamed **Government Commercial Agency (GCA)** on **1 April 2026**. [Source: https://www.gca.gov.uk/] It is the UK's central procurement agency; it negotiates large pan-government framework agreements that individual public bodies "call off" from. New URL: https://www.gca.gov.uk (old https://www.crowncommercial.gov.uk redirects).

#### 7.7.2. Legal basis
- GCA operates under PA 2023 (or PCR 2015 for legacy frameworks still running).
- Each framework has its own commercial terms (Schedule of Suppliers, Schedule of Services).

#### 7.7.3. Who is in scope
- Suppliers offering goods/services that match an existing or upcoming framework.
- Sorellon: **MEDIUM exposure** — most cleaning/PPE/healthcare consumables frameworks are *not currently open* for new supplier admission, but DPS-based mechanisms exist for some categories.

#### 7.7.4. Sorellon-specific exposure
See **Section 7.7B** below — separate sub-section enumerates relevant frameworks.

#### 7.7.5. Registration / onboarding mechanics
- Two routes:
  - **Framework competitions**: GCA periodically opens framework tenders (typically every 3–5 years per framework). Suppliers must respond to the framework tender during its open window.
  - **DPS (Dynamic Purchasing System)**: open continuously; suppliers can join at any time by completing the DPS PQQ.
- The Supplier Registration Service (https://supplierregistration.cabinetoffice.gov.uk) is still used for DPS access by some GCA agreements. [Source: https://supplierregistration.cabinetoffice.gov.uk/]

#### 7.7.6. Documents to prepare
- All items in 7.1.6 / 7.2.6.
- Insurance evidence (typically £5M PL / £5M PI / £5M EL; some frameworks require higher).
- ISO 9001 OR equivalent quality plan.
- ISO 14001 OR environmental policy (mandatory on most GCA frameworks).
- Cyber Essentials Plus (mandatory on most GCA frameworks since 2018).
- Equality / diversity policy.
- Modern Slavery Act statement or equivalent.
- Past contract examples (minimum 3, ideally public sector).
- Financials (3 years for established frameworks — significant blocker for Sorellon as a 10-month-old entity).

#### 7.7.7. Recurring obligations
- Quarterly Management Information (MI) returns: report all sales made via the framework.
- Pay GCA's commission (typically 1% of throughput — varies by framework).
- Maintain insurance, cyber accreditation, etc.

#### 7.7.8. Penalties for non-compliance
- Framework removal.
- MI under-reporting → audit + commission recovery + potential debarment.

#### 7.7.9. Interaction with other regimes
- Framework + below-threshold publication still required under PA 2023.
- Cyber Essentials requirement intersects with NHS DSP Toolkit if also doing NHS work.

#### 7.7.10. Reseller carve-outs
- Many frameworks specifically allow distributor/reseller route — but the manufacturer may need to be **named** in Sorellon's bid, and Sorellon may need supply-chain letters from manufacturers committing to the framework volume.

#### 7.7.11. Practical pitfalls
- Framework competitions are **massive bid efforts** (typically 200–400 hour bid teams).
- **Past performance evidence** is the biggest blocker for new entrants — almost every framework asks for 3 contracts of similar value/scope completed in the last 3 years.
- **Cyber Essentials Plus** has a 30-day audit lead time and ~£2,000 cost.
- **Sub-contracting model**: some frameworks accept "agent for manufacturer" arrangements — but the manufacturer becomes co-liable.

#### 7.7.12. Verification flags
- `[VERIFY: current open framework windows for Sorellon's categories — most cleaning/PPE/healthcare frameworks have last been let in 2022-2024 and won't reopen until 2027-2029]`.
- `[VERIFY: existence of "supplier on-ramping" routes on closed frameworks via subcontracting]`.

#### 7.7.13. Recommended action sequence
1. Audit current GCA framework catalogue for Sorellon's categories (see 7.7B).
2. Identify any open DPS routes (typically more accessible than frameworks).
3. For closed frameworks: identify Tier 1 prime suppliers and pitch as subcontractor.
4. Subscribe to GCA "Upcoming Agreements" pipeline (https://www.gca.gov.uk/agreements/upcoming).

---

### 13.7B. CCS / GCA framework-specific entry routes for Sorellon's product categories

This sub-section enumerates GCA frameworks Sorellon should target by product line. **All status / expiry dates `[VERIFY]` against https://www.gca.gov.uk/agreements at time of action.**

#### 7.7B.1. Cleaning products & janitorial supplies

- **RM6130 — Building Cleaning Services** — **EXPIRED 7 July 2023** per search results. Successor frameworks may exist. `[VERIFY: current GCA cleaning services replacement framework]`. This framework was for cleaning *services*, not products — but historically had supply lots.
- **Office Supplies & Equipment** frameworks (RM-numbered) — include stationery, paper, electronic office supplies, **and janitorial products** as a category. `[VERIFY: current RM number for office supplies post-RM6059]`.
- **Direct Hygiene/Cleaning Products DPS** — `[VERIFY: whether GCA currently runs a hygiene-products DPS]`.

#### 7.7B.2. PPE — face coverings, gloves, workwear

- **RM3763 — Personal Protective Equipment** — historic CCS PPE framework, **expired 2019** per search; long superseded. Search results inconclusive on current PPE framework.
- COVID-era ad-hoc procurement vehicles (e.g. PPE buyer's portal during 2020–2022) are wound down.
- `[VERIFY: current CCS/GCA PPE framework number and status as of May 2026]`. This is **the highest-priority verification item** in this whole document because Sorellon's PPE lines depend on it.

#### 7.7B.3. Healthcare consumables

- Generally routed through **NHS Supply Chain (see 7.8)** not GCA — NHS has its own commercial vehicle.
- GCA does run some general medical supplies frameworks but they are typically NHS-secondary.

#### 7.7B.4. Education / stationery / facilities

- **TePAS 2 (RM6098)** — Tablet & Pad supply DPS (not Sorellon's category but referenced in search).
- General office supplies frameworks include education stationery.
- `[VERIFY: current GCA education stationery framework]`.

#### 7.7B.5. Recommended GCA entry strategy for Sorellon

1. Prioritise **DPS-based** routes (always open).
2. Plan multi-year framework calendar — track upcoming agreements pipeline.
3. Consider **subcontractor** to existing framework primes as a fast-entry route.
4. Build past-performance evidence via **lower-barrier** routes (NHS Supply Chain dynamic markets, Sell2Wales direct awards) to qualify for next framework cycle.

---

### 13.8. NHS Supply Chain — the NHS's central commercial vehicle

#### 7.8.1. What it is
**NHS Supply Chain** (operated by Supply Chain Coordination Limited — SCCL) is a wholly-owned subsidiary of the Department of Health & Social Care that aggregates non-pay procurement for NHS England Trusts. It runs through **11 Category Towers** — each is a specialist commercial unit covering one product family. URL: https://www.supplychain.nhs.uk. [Source: https://www.supplychain.nhs.uk/categories/]

#### 7.8.2. Legal basis
- NHS Supply Chain operates under PA 2023.
- Individual frameworks are themselves PA 2023 / PCR 2015 frameworks.

#### 7.8.3. Who is in scope
- Suppliers wanting to sell into NHS England (note: NHS Scotland, NHS Wales, HSC Northern Ireland each have separate vehicles).
- Sorellon: **VERY HIGH exposure** — PPE, hygiene, cleaning, healthcare consumables ALL fall in NHS Supply Chain scope.

#### 7.8.4. Sorellon-specific exposure
**Sorellon's products span MULTIPLE category towers**:

- **PPE & medical consumables** — Single Use Personal Protective Equipment and Medical Protective Consumables framework. 76 suppliers in total; Lots 2/4/6 expire 3 Sep 2026 with 24+9 month extensions. [Source: https://www.supplychain.nhs.uk/product-information/contract-launch-brief/single-use-personal-protective-equipment-and-medical-consumables/]
- **Hand Hygiene** — Hand Hygiene and Associated Products and Services framework, started 12 Jun 2023, expires 11 Jun 2026 (+12 month extension option). [Source: https://www.supplychain.nhs.uk/product-information/contract-launch-brief/hand-hygiene-and-associated-products-and-services/]
- **Skin Cleansing & Disinfection** — separate framework.
- **Cleaning Supplies, Equipment & Associated Products** — sustainability/net-zero/social value mandatory requirements. [Source: https://www.supplychain.nhs.uk/product-information/contract-launch-brief/cleaning-supplies-equipment-and-associated-products/]
- **Environmental Decontamination Products** — new framework starts 30 March 2026, runs 24 months to 29 March 2028 (+24 month extension). [Source: https://www.supplychain.nhs.uk/product-information/contract-launch-brief/environmental-decontamination/]
- **Medical & Surgical Consumables tower** — covers airway, infection control, IV therapy, wound closure, PPE, theatre / ward products, respiratory, surgical instruments, sutures.
- **Facilities & Office Solutions tower** — catering & cleaning equipment, furniture, medical pulp, stationery, uniforms, workwear.

#### 7.8.5. Registration / onboarding mechanics
- Two parallel systems:
  - **Supplier Portal** (https://www.supplychain.nhs.uk/suppliers/) — once on a framework.
  - **Atamis** (https://health-family.force.com/s/Welcome) — for tender competitions across NHS England. [Source: https://www.supplychain.nhs.uk/suppliers/portal/]
- Atamis registration requires: Companies House Reg No (16575318), D-U-N-S Number, organisation info, sustainability disclosures (Evergreen).
- Free to register; competition response is per tender.

#### 7.8.6. Documents to prepare
- All items in 7.1.6 / 7.2.6 / 7.7.6.
- **NHS Evergreen Sustainable Supplier Assessment** — pillars: 1) Workforce, 2) Net Zero, 3) Modern Slavery, 4) Information Governance. [Source: https://england.nhs.uk/nhs-commercial/central-commercial-function-ccf/evergreen/]
- **ISO 13485** (medical devices QMS) — STRONGLY preferred for any device/PPE supplier; Sorellon as reseller may not need it directly but expect to be asked.
- DCB0129 / DCB0160 clinical safety case management (only for digital health — N/A here but flagged).
- **NHS DSP Toolkit** (Data Security & Protection) — N/A unless handling NHS data, but worth knowing.
- Sustainability commitments (carbon reporting, social value evidence).

#### 7.8.7. Recurring obligations
- Per-framework: MI reporting, audits, product compliance refreshes.
- Annual Evergreen reassessment.
- Vigilance/recall obligations (forwarded from MHRA — see 8.4).

#### 7.8.8. Penalties for non-compliance
- Framework suspension or removal.
- NHS-wide blacklisting (de facto).
- Substantial reputational damage in healthcare market.

#### 7.8.9. Interaction with other regimes
- MHRA medical device registration — every glove, syringe, dressing Sorellon sells through NHS Supply Chain must be MHRA-registered (see 8.3).
- UKCA / CE marking obligations.
- BSI standards (BS EN 455 for medical gloves, BS EN 1276 / 14476 for disinfectants).
- Modern Slavery Act compliance.
- NHS Net Zero Supplier Roadmap (2030 milestone — direct emissions; 2032 milestone — supply chain emissions).

#### 7.8.10. Reseller carve-outs
- Sorellon as distributor must demonstrate **supply chain control** — typically via manufacturer letters and stock commitments.
- Can register as "supplier" via Atamis even without ISO 13485 IF manufacturer holds it.

#### 7.8.11. Practical pitfalls
- **Foundation Trust vs central procurement**: NHS Trusts can buy outside NHS Supply Chain (and many do), but central procurement is moving steadily toward consolidation. Sorellon should ALSO target Trust-level procurement teams directly while pursuing NHS Supply Chain frameworks.
- **Towers boundaries are fluid** — same product (e.g. a disposable apron) may fit multiple frameworks. Apply to the most appropriate one, not all.
- **Foundation Trust direct procurement** — bypasses NHS Supply Chain but often demands the same evidence base.
- **Atamis is genuinely the dominant NHS e-procurement platform now** — by 2025 all NHS Tender opportunities are hosted there. [Source: https://hertsprocurement.nhs.uk/joomla/index.php?option=com_content&view=article&id=5&Itemid=10]

#### 7.8.12. Verification flags
- `[VERIFY: PPE framework re-tender timing — Lots 2/4/6 expiry 3 Sep 2026; check for re-procurement notice in early 2026]`.
- `[VERIFY: Hand Hygiene framework expiry 11 Jun 2026 — is it being re-tendered? When?]`.
- `[VERIFY: Environmental Decontamination Products framework — supplier list publicly available?]`.

#### 7.8.13. Recommended action sequence
1. Register on Atamis (D-U-N-S number required — same one used for eTendersNI).
2. Complete Evergreen Sustainable Supplier Assessment.
3. Apply to **upcoming** NHS Supply Chain frameworks during their tender windows — start with re-tenders coming up in 2026 (Hand Hygiene, PPE).
4. In parallel: pitch to Foundation Trust procurement teams directly for off-framework wins to build past-performance evidence.

---

### 13.9. Constructionline / Achilles / JOSCAR — cross-reference and overlap with CAS

#### 7.9.1. What it is
These are private accreditation bodies running pre-qualification databases used by many UK contracting authorities.

- **Constructionline** — construction supply chain (CAS-certified). https://www.constructionline.co.uk.
- **Achilles** — multiple databases including BuildingConfidence, UVDB (utilities), JOSCAR.
- **JOSCAR (Joint Supply Chain Accreditation Register)** — defence sector; less relevant for Sorellon.

#### 7.9.2. Legal basis
- Industry-led, not statutory.

#### 7.9.3. Who is in scope
- Suppliers selling to sectors that mandate these (utilities, defence, construction).

#### 7.9.4. Sorellon-specific exposure
- **LOW**. Sorellon is unlikely to face JOSCAR / Achilles UVDB unless selling into utilities or defence — both possible for PPE and cleaning chemicals but not Sorellon's main pipeline.

#### 7.9.5–7.9.13. (Abbreviated — defer; same patterns as 7.3.)

**Recommendation**: do not pursue unless a specific target tender mandates it.

---

## 14. Product-Specific Regulatory Obligations

### 14.1. UK REACH — chemicals registration (post-Brexit retained EU REACH)

#### 8.1.1. What it is
**UK REACH** (Registration, Evaluation, Authorisation and restriction of Chemicals) is the UK's chemicals regulation regime, replacing EU REACH after Brexit for Great Britain (Northern Ireland still under EU REACH via the Windsor Framework). The UK competent authority is the **HSE** (Health & Safety Executive); the appointed Agency is the HSE Chemicals Regulation Division. [Source: https://www.hse.gov.uk/reach/]

#### 8.1.2. Legal basis
- **REACH etc. (Amendment etc.) (EU Exit) Regulations 2019** (SI 2019/758).
- Various subsequent amendment SIs through 2024.
- Functionally: EU REACH preserved into GB law with UK-specific procedural changes (the "UK REACH IT" platform; HSE rather than ECHA).

#### 8.1.3. Who is in scope
- **Manufacturers** of chemicals: must register substances ≥1 tonne/year.
- **Importers** (i.e. anyone bringing chemicals from outside GB into GB): same obligations as manufacturers.
- **Downstream users**: obligations to follow risk management measures from suppliers' SDS, but no registration.
- **Distributors**: store and supply without modification — primarily a duty to pass on information (SDS) up and down the chain.

#### 8.1.4. Sorellon-specific exposure
- Sorellon distributes cleaning chemicals (disinfectants, degreasers, glass cleaners, sanitisers) and biocidal wipes.
- **If Sorellon buys finished cleaning products from UK manufacturers**: Sorellon is a **distributor** — minimal direct REACH burden, just SDS pass-through.
- **If Sorellon buys finished cleaning products from EU manufacturers**: Sorellon may be classified as an **importer** under UK REACH — much heavier burden. [Source: https://www.hse.gov.uk/reach/duin.htm]
- **If Sorellon buys cleaning products from China**: Sorellon is **definitely the GB-side importer** — full importer obligations including substance registration (if any substance constituent crosses 1 tonne/year threshold).

#### 8.1.5. Registration / onboarding mechanics
- Distributor: no registration required, but maintain SDS register (electronic OK).
- Importer (Sorellon's likely classification for EU/China sourced products):
  - Determine if any substance crosses 1 tonne/year cumulative import threshold.
  - If yes, submit registration to HSE via **UK REACH IT** (https://reach-it.hse.gov.uk).
  - Alternative: rely on **Only Representative (OR)** appointed by your overseas supplier — they take over UK importer registration burden. Many EU manufacturers appoint OR services exactly to relieve UK importers.

#### 8.1.6. Documents to prepare
- Safety Data Sheets (SDS) for every chemical product — must be in English, dated, version-controlled.
- Compositional information (often confidential — request from supplier under NDA).
- Tonnage tracking per substance per calendar year.
- Annexes for Substances of Very High Concern (SVHC) on the UK candidate list — disclosure obligations on import.

#### 8.1.7. Recurring obligations
- Update SDS on revision (send new version to all customers from preceding 12 months).
- Track tonnages annually.
- Report any new SVHC notifications.
- **The 2026 transitional deadline**: if Sorellon's substances were registered under EU REACH pre-Brexit, the registration may be deferred to 27 October 2026, 2028 or 2030 depending on tonnage and hazard. [Source: https://www.hse.gov.uk/reach/duin.htm] But this is more relevant to importers established pre-2021 — Sorellon (incorporated 2025) cannot benefit from this and faces **fresh registration** if importing.

#### 8.1.8. Penalties for non-compliance
- Criminal offence under REACH Enforcement Regulations 2008.
- Unlimited fine on indictment; up to £20,000 summarily plus 3 months imprisonment.
- Goods can be detained / destroyed at port.

#### 8.1.9. Interaction with other regimes
- **GB CLP** (classification, labelling, packaging) — see 8.2.
- **GB BPR** (biocides) — see 8.5.
- **GPSR** — general product safety.
- **HMRC customs** — chemical commodity codes feed into REACH compliance checks.

#### 8.1.10. Reseller carve-outs
- If Sorellon ONLY distributes finished goods from a UK manufacturer or a UK-registered Only Representative of an overseas manufacturer: distributor obligations only (SDS pass-through). Substantial relief.
- If Sorellon imports directly: full importer obligations.

#### 8.1.11. Practical pitfalls
- **Misclassifying yourself as a distributor when you are actually the importer** — happens routinely with new entrants buying from European trade shows.
- **EU-format SDS** (16 sections per EU REACH) used without GB CLP labelling adjustments — common compliance failure.
- **Mixtures** — Sorellon's cleaning products are mostly mixtures; substance-level tonnage calculation requires aggregation across products.
- **Notification of Substances in Articles** (e.g. cleaning wipe products) — separate obligation if SVHC content >0.1% w/w and tonnes/year crosses 1.

#### 8.1.12. Verification flags
- `[VERIFY: Sorellon's actual import vs distribute split — which overseas suppliers have appointed UK Only Representatives?]`.
- `[VERIFY: aggregate tonnage of each substance Sorellon imports per calendar year — likely under 1 tonne/yr but should be tracked]`.
- `[VERIFY: GB-CLP vs EU-CLP harmonisation status as of May 2026]`.

#### 8.1.13. Recommended action sequence
1. Map every product line to: (a) UK-manufactured, (b) EU-manufactured (with OR? without?), (c) Chinese-manufactured.
2. For each non-UK supplier, request: GB OR contact (if any), GB-compliant SDS, classification info.
3. Set up a substance-by-substance tonnage tracker.
4. If any substance threatens to cross 1 tonne/yr → escalate to UK REACH IT registration or appoint OR.

---

### 14.2. GB CLP — Classification, Labelling & Packaging of chemicals

#### 8.2.1. What it is
**GB CLP** is the assimilated UK version of EU CLP Regulation (EC 1272/2008) — the GHS-aligned regime for classifying, labelling, and packaging hazardous chemicals. GB CLP is now diverging from EU CLP (the UK is updating independently). [Source: https://www.hse.gov.uk/chemical-classification/legal/clp-regulation.htm]

#### 8.2.2. Legal basis
- **GB CLP** = retained EU 1272/2008 + Chemicals (Health & Safety) and Genetically Modified Organisms (Contained Use) (Amendment etc.) (EU Exit) Regulations 2019.
- DEFRA + HSE joint competent authority role.

#### 8.2.3. Who is in scope
- Manufacturers, importers, downstream users, **distributors** — all in scope, with role-specific duties.

#### 8.2.4. Sorellon-specific exposure
- Every cleaning chemical Sorellon supplies (degreaser, disinfectant, bleach, sanitiser, glass cleaner, etc.) is in scope.
- Hygiene wipes with bactericidal claims also in scope (their wetting solution often contains hazardous actives).

#### 8.2.5. Registration / onboarding mechanics
- No "registration" — duty is **compliance**.
- For products with novel classifications: notify HSE within 1 month of first placing on GB market (C&L Notification).

#### 8.2.6. Documents to prepare
- GB CLP-compliant labels — in English, with: GB supplier identity (name, address, phone), product identifier, hazard pictograms (CLP), signal word, H-statements, P-statements, supplemental info.
- SDS aligned with label (Section 2 must match exactly).

#### 8.2.7. Recurring obligations
- Update labels & SDS on any classification change.
- Maintain consistency between SDS and label.

#### 8.2.8. Penalties for non-compliance
- CHIP/CLP Enforcement Regulations: criminal offences, unlimited fines on indictment.
- HSE inspectors empowered to seize/destroy mislabelled goods.

#### 8.2.9. Interaction with other regimes
- REACH SDS feeds CLP labels.
- Biocides (BPR) override CLP for the biocidal label aspects.
- Detergents Regulation overrides for surfactant disclosure on the label.

#### 8.2.10. Reseller carve-outs
- A distributor who relabels a product (e.g. removes manufacturer's German label, applies Sorellon's English label) **becomes a labeller** with full classification responsibility.
- A distributor who simply shrink-wraps the original manufacturer's label and resells: lighter obligation, but Sorellon's name and address must still be on the label as a GB economic operator (the **"name on the label"** principle is a recurring theme across reseller obligations).

#### 8.2.11. Practical pitfalls
- **Translation errors** when re-labelling — incorrect translation of an H-statement can be a criminal offence.
- **Bilingual labels** (Welsh) — not strictly mandatory for chemicals but increasingly demanded by Welsh public sector.
- **"Distributor" branding** — putting Sorellon's name and address on a product made by someone else can constitute Sorellon being treated as the manufacturer for liability purposes.

#### 8.2.12. Verification flags
- `[VERIFY: GB-CLP divergence from EU-CLP — confirm GB-specific H/P statements at May 2026]`.
- `[VERIFY: whether Sorellon plans to "white-label" any product (put Sorellon brand on third-party product)]`.

#### 8.2.13. Recommended action sequence
1. For each chemical product in catalogue: audit label vs GB-CLP requirements.
2. Build a master SDS register (digital, version-controlled).
3. If white-labelling planned: engage a regulatory consultant before launch.

---

### 14.3. MHRA — Medical device registration & distributor obligations

#### 8.3.1. What it is
**MHRA** = Medicines and Healthcare products Regulatory Agency. Responsible for medical devices market access in the UK. **Class I** = lowest-risk medical devices; **IIa / IIb / III** = increasing risk. Distributors do NOT register the device themselves — registration is done by the manufacturer or their UK Responsible Person (UKRP). [Source: https://www.regdesk.co/blog/mhra-guidance-on-md-regulations/]

#### 8.3.2. Legal basis
- **Medical Devices Regulations 2002** (SI 2002/618, "UK MDR 2002") — still the current GB framework.
- Future: draft Medical Devices (Amendment) Regulations 2026 notified to WTO 8 May 2026 — `[VERIFY: passage and commencement]`. [Source: https://www.gov.uk/government/news/mhra-invites-views-on-proposed-changes-to-medical-device-regulation]
- **Medical Devices (Post-market Surveillance Requirements) Regulations 2024** — in force 16 June 2025.

#### 8.3.3. Who is in scope
- **Manufacturers** + their UKRP: register devices.
- **Importers**: register their importer status.
- **Distributors**: NO registration, but heavy operational obligations.

#### 8.3.4. Sorellon-specific exposure
Sorellon distributes multiple medical devices:

- **Disposable examination gloves (non-sterile)**: **Class I** medical device under UK MDR 2002 (unless used in surgery → Class IIa). **DUAL classification**: also PPE under PPE Regulation 2016/425 (chemical protection / blood-borne pathogen).
- **Basic syringes (non-sterile, no active substance)**: **Class IIa**.
- **Catheters (basic urinary)**: **Class IIa**.
- **Basic dressings (non-sterile)**: **Class I**.
- **Hygiene wipes with claims (antimicrobial)**: NOT a medical device per se — likely a **biocidal product** (see 8.5).
- **Workwear (non-PPE)**: not a medical device.

#### 8.3.5. Registration / onboarding mechanics
- **NOT Sorellon's job** — the manufacturer (or their UKRP, if overseas) must register each device with MHRA.
- Sorellon's job: verify that the manufacturer/UKRP HAS registered, and retain proof.
- MHRA fee: £300/year per GMDN Level 2 category (no upfront fee from 1 April 2026 — annual subscription model). [Source: https://casusconsulting.com/services/uk/device-registration/]
- Registration renewed annually first year, then every 2 years.

#### 8.3.6. Documents to prepare
For EVERY medical device Sorellon distributes, retain:
- Manufacturer / UKRP registration confirmation (MHRA acknowledgement letter / portal screenshot).
- Declaration of Conformity (UKCA or CE) — manufacturer-issued.
- UKCA / CE certificate (for Class IIa+ — issued by UK Approved Body or EU Notified Body).
- Instructions for Use (IFU) in English.
- Label compliance — manufacturer name, UDI when applicable, batch/lot tracking info.
- Adverse event reporting procedure (Sorellon's internal SOP).

#### 8.3.7. Recurring obligations
- **Vigilance/incident reporting**: Sorellon as distributor must pass any complaint or adverse event from customers to the manufacturer/UKRP within 15 days. Serious incidents must be MHRA-notified by manufacturer in 15 days (down from 30 under old regime). [Source: https://www.hoganlovells.com/en/publications/be-vigilant-new-postmarket-surveillance-requirements-for-medical-devices-in-great-britain]
- **Traceability records**: keep records of from-whom-bought, to-whom-sold for every device for the lifetime of the device + retention period.
- **Cooperate with recalls / FSNs (Field Safety Notices)** — pass on to customers and provide back-reports.
- **Storage & transport** — maintain conditions per manufacturer's IFU (humidity, temperature, no contamination).
- **Pre-sale checks**: verify label, mark, sterility (if applicable), expiry date, packaging integrity.

#### 8.3.8. Penalties for non-compliance
- Criminal offences under UK MDR 2002.
- Distributor placing non-compliant device on market = unlimited fine on indictment.
- Reputational and commercial: NHS Trusts will refuse to deal with distributors who failed vigilance.

#### 8.3.9. Interaction with other regimes
- **UKCA / CE marking** — see 8.4.
- **PPE Regulation 2016/425** for dual-use gloves (see 8.6).
- **NHS Supply Chain frameworks** require demonstrable MHRA-compliance.

#### 8.3.10. Reseller carve-outs
- Distributor is NOT a manufacturer for MHRA purposes if it:
  - Does not modify the device, packaging, or label.
  - Does not place under its own brand.
  - Does not assemble or sterilise.
- If Sorellon white-labels (puts "Sorellon Gloves" on a Chinese manufacturer's product): Sorellon becomes the **legal manufacturer** under UK MDR 2002 and assumes the full manufacturer registration burden, UKRP if Sorellon's principals are in the UK (which they are, so no UKRP needed), full technical file ownership, etc. **This is a critical decision point**.

#### 8.3.11. Practical pitfalls
- **Chinese supplier "CE certificate"** = sometimes a self-declaration by the manufacturer, not a Notified Body certificate. For Class IIa+ this is invalid. Distributors are caught in the middle.
- **Lapsed CE certificates** in transition period — CE marked Class I devices: accepted in GB until 30 June 2028 OR certificate expiry, whichever earlier. CE marked IVDs: until 30 June 2030. [Source: https://www.scilife.io/blog/ukca-mark]
- **UDI requirements** are tightening — draft 2026 regulations make UDI compulsory across the device lifecycle.
- **Surgical vs examination gloves** — surgical = Class IIa (needs Notified Body), examination = Class I (manufacturer self-declares). Sorellon must be clear which it sells.

#### 8.3.12. Verification flags
- `[VERIFY: MHRA's current consultation on indefinite CE acceptance — submissions closed 10 April 2026; outcome and timing of decision]`. [Source: https://www.kennedyslaw.com/en/thought-leadership/article/2025/mhra-to-consult-on-indefinite-recognition-of-ce-marked-medical-devices-a-major-step-for-industry-and-patients-in-great-britain/]
- `[VERIFY: draft Medical Devices (Amendment) Regulations 2026 status — commencement date uncertain]`.
- `[VERIFY: for EACH device Sorellon sources from China, whether the supplier has a UKRP and is registered with MHRA]`.

#### 8.3.13. Recommended action sequence
1. Build a **device register** with columns: product, manufacturer, UKRP, MHRA registration number, conformity assessment route, classification, certificate expiry.
2. For each Chinese-sourced device: insist on UKRP appointment (or refuse to stock).
3. Implement a complaint/incident SOP — every customer complaint reviewed for vigilance reportability.
4. Set up automated MHRA Field Safety Notice monitoring.

---

### 14.4. UKCA / CE marking — conformity marking for GB market

#### 8.4.1. What it is
**UKCA** = UK Conformity Assessed mark; **CE** = European Conformity mark. Marks indicate product meets relevant UK / EU regulatory requirements. Post-Brexit, the UK initially planned to require UKCA exclusively, but has **indefinitely extended CE acceptance** for the vast majority of regulated products in GB. [Source: https://landmarkglobal.com/eu/en/news-insights/ukca-vs-ce-marking-in-2026/]

#### 8.4.2. Legal basis
- Various sectoral regulations (PPE, Toys, EMC, Medical Devices, etc.) all reference UKCA/CE acceptance.
- General CE acceptance extended via amendment SIs through 2023–2024.

#### 8.4.3. Who is in scope
- Manufacturers, importers, distributors of regulated products.

#### 8.4.4. Sorellon-specific exposure
**Sorellon's catalogue spans multiple marking regimes**:

| Product | Marking regime | Status May 2026 |
|---|---|---|
| Examination gloves | UKCA OR CE (MHRA + PPE) | CE accepted until 30 Jun 2028 OR cert expiry |
| Workwear (non-protective) | None | N/A |
| High-vis (Cat II PPE) | UKCA OR CE (PPE) | **CE accepted indefinitely** for PPE |
| Chemical-resistant gloves (Cat III PPE) | UKCA OR CE (PPE) | **CE accepted indefinitely** (with Notified Body number) |
| Syringes (Class IIa) | UKCA OR CE (Medical Devices) | CE accepted until 30 Jun 2028 OR cert expiry |
| Catheters (Class IIa) | UKCA OR CE | Same |
| Basic dressings (Class I MD) | UKCA OR CE | Same |
| Cleaning chemicals | No CE/UKCA — but GB-CLP labels | N/A |
| Biocidal wipes | No CE/UKCA — GB BPR authorisation | See 8.5 |
| Education stationery | None (unless toys) | N/A |
| Painting/maintenance kits | Variable | Per-product |

#### 8.4.5. Registration / onboarding mechanics
- No registration — distributor's job is **verification**.

#### 8.4.6. Documents to prepare
Distributor must, on request from market surveillance authority, be able to produce within reasonable time:
- Declaration of Conformity (DoC) — issued by manufacturer.
- Technical file location — distributor doesn't hold it, but must know where it is (UKRP or manufacturer).
- Notified Body certificate (for products requiring third-party assessment).
- Translation of DoC into English if not originally English.

#### 8.4.7. Recurring obligations
- Monitor any **withdrawals** of conformity by manufacturers.
- Update files when manufacturers re-certify.

#### 8.4.8. Penalties for non-compliance
- Distributor placing non-marked product on UK market = criminal offence under sectoral regulations.
- Fines: unlimited on indictment.

#### 8.4.9. Interaction with other regimes
- MHRA registration requires CE/UKCA proof.
- PPE Regulation requires CE/UKCA.
- General Product Safety overlays.

#### 8.4.10. Reseller carve-outs
- **Distributor's "verification" duty is real but proportionate** — Sorellon must take reasonable steps to confirm the mark and DoC exist; not required to re-test the product.
- However: if Sorellon places its own brand on the product → Sorellon becomes manufacturer for marking purposes.

#### 8.4.11. Practical pitfalls
- **Chinese "CE" stickers** sometimes mean "China Export" (a fake mark with different spacing) — Sorellon must verify the mark is genuine EU CE.
- **Notified Body number** must accompany the CE for products requiring third-party certification — absence of NB number on a Cat III PPE or Class IIa medical device = invalid.
- **"Name on the label"** — even if the product carries the manufacturer's mark and DoC, the GB market expects a GB economic operator's name/address. Sorellon's name as distributor is acceptable (with manufacturer's name above it).

#### 8.4.12. Verification flags
- `[VERIFY: indefinite CE recognition status — confirmed by government statements 2024-2025 for vast majority of categories; medical devices and construction have different timelines]`.
- `[VERIFY: PPE-specific CE acceptance timeline — search results suggest indefinite, but check official PPE-specific updates]`.

#### 8.4.13. Recommended action sequence
1. For each SKU: capture conformity mark (UKCA or CE), Notified Body number (if any), DoC reference.
2. For Chinese-sourced products: independently verify the CE mark is genuine (Notified Body lookup on Nando database).
3. Maintain DoC library digitally.
4. Build "GB importer/distributor label sticker" with Sorellon's name & address for products that need it.

---

### 14.5. UK Biocidal Products Regulation (GB BPR) — antimicrobial wipes & disinfectants

#### 8.5.1. What it is
**GB BPR** is the UK's biocides regime — the assimilated EU BPR (Regulation 528/2012) under GB law. The **HSE is the GB competent authority**. Any product with a **primary biocidal function** (kills, deters, renders harmless, prevents, or otherwise controls harmful organisms) is a biocidal product and must be **authorised** before placement on the UK market. [Source: https://www.hse.gov.uk/biocides/about/regulations-decisions.htm]

#### 8.5.2. Legal basis
- **Biocidal Products Regulations 2012** as amended.
- Biocidal Products (Health and Safety) (Amendment etc.) (EU Exit) Regulations 2019.

#### 8.5.3. Who is in scope
- Anyone placing a biocidal product on the GB market.
- Distributors selling biocidal products: must verify product is authorised AND active substance supplier is on the **GB Article 95 list**.

#### 8.5.4. Sorellon-specific exposure
**THIS IS THE SINGLE MOST DANGEROUS PRODUCT-REGULATORY EXPOSURE FOR SORELLON.**

Sorellon's hygiene wipes that claim **EN1276 (bactericidal)** or **EN14476 (virucidal)** are **biocidal products** under GB BPR.

- A wipe whose primary function is "wiping" with incidental cleaning = NOT biocidal (a "treated article" at most).
- A wipe whose primary function is "kills 99.9% of bacteria" / "virucidal" / "disinfects" = **biocidal product**.
- Marketing claims like "EN1276 tested" or "kills norovirus" on the label or website = biocidal claim → product is a biocidal product → must be GB-authorised before sale. [Source: https://www.rtcoopers.com/practice_biocides.php]

Cleaning chemicals with biocidal actives (chlorine, quats, alcohol-based disinfectants marketed as "kills bacteria") are similarly in scope.

#### 8.5.5. Registration / onboarding mechanics
- Product authorisation is the manufacturer's responsibility — but the distributor is **not absolved**.
- Sorellon must:
  - Confirm the active substance is approved for the relevant **product type** (PT1 human hygiene, PT2 disinfectants & algaecides, PT3 veterinary hygiene, PT4 food/feed area disinfectants — wipes are usually PT2).
  - Confirm the active substance supplier is on the **GB Article 95 list**. [Source: https://www.hse.gov.uk/biocides/uk-list-active-substances.htm]
  - Confirm the product has a UK authorisation (or is covered by transitional measures).
- During transitional period (active substance under review): products containing reviewed actives may remain on market under "national authorisation" or COPR (older Control of Pesticides Regulations regime).

#### 8.5.6. Documents to prepare
- Manufacturer's authorisation certificate (or transitional authorisation evidence).
- Active substance approval reference (HSE / ECHA).
- GB-CLP-compliant label including BPR-required statements.
- Product Assessment Report (PAR) for the product — manufacturer-issued.

#### 8.5.7. Recurring obligations
- Monitor for changes in active substance approval status — substances are reviewed on rolling basis.
- If active substance is **not approved** for the product type: product becomes illegal to place on market and Sorellon must withdraw.
- Notify HSE if any adverse event.

#### 8.5.8. Penalties for non-compliance
- Placing unauthorised biocidal product on market = **criminal offence** under the Biocidal Products and Chemicals (Appointment of Authorities and Enforcement) Regulations 2013.
- Unlimited fines on indictment; goods seizable.
- HSE has been actively enforcing — particularly around COVID-era surface disinfectants and hand sanitisers.
- Class actions / reputational damage if a "kills 99.9% of bacteria" claim is unsubstantiated.

#### 8.5.9. Interaction with other regimes
- **GB CLP** for labelling.
- **UK REACH** for the actives.
- **General Product Safety**.
- **Trading Standards** enforce locally.

#### 8.5.10. Reseller carve-outs
- A distributor relying entirely on the manufacturer's authorisation is permitted to sell IF the authorisation is valid AND the product is sold without modification (no rebranding, no new claims).
- If Sorellon adds claims like "great for hospitals" to a manufacturer's biocidal wipe → Sorellon becomes a "secondary supplier" with additional liability.

#### 8.5.11. Practical pitfalls
- **Marketing copy on Sorellon's website / catalogue** is the most common compliance failure. Listing "EN1276 tested" as a feature on a product that isn't BPR-authorised is itself an offence.
- **Chinese-sourced wipes** often have unverifiable BPR compliance.
- **Hand sanitisers** — surge of unauthorised products during COVID-19; HSE has been recovering ground via enforcement.
- **"Antibacterial" labelling on cleaning wipes** without underlying authorisation is the textbook trap.

#### 8.5.12. Verification flags
- `[VERIFY: for EACH antimicrobial wipe Sorellon stocks, the GB BPR authorisation status — active substance approved? In transitional measures? Product authorised?]`.
- `[VERIFY: GB Article 95 list status of each active substance Sorellon distributes — list is dynamic]`.
- `[VERIFY: whether Sorellon's website / marketing materials have made biocidal claims (kills bacteria, virucidal, etc.) that exceed product authorisation scope]`.

#### 8.5.13. Recommended action sequence
1. **IMMEDIATE**: audit Sorellon's catalogue for any product making biocidal claims (EN1276, EN14476, "kills", "antimicrobial", "antibacterial", "disinfectant", "sanitises").
2. For each: confirm GB BPR authorisation status or transitional cover.
3. Remove from sale any product where authorisation cannot be evidenced.
4. Audit marketing copy — pull any claims unsupported by authorisation.
5. Make GB BPR compliance a **gate** for any new biocidal product in Sorellon's catalogue.

---

### 14.6. UK PPE Regulation 2016/425 — personal protective equipment

#### 8.6.1. What it is
**Personal Protective Equipment (Enforcement) Regulations 2018** enforce assimilated EU Regulation 2016/425 in GB. Categorises PPE into:
- **Cat I**: minimal risk (sunglasses, gardening gloves, hi-vis where used for visibility only) — self-declaration by manufacturer.
- **Cat II**: medium risk — requires Notified/Approved Body type examination.
- **Cat III**: irreversible/serious risk — requires Notified Body type examination + ongoing surveillance.
[Source: https://www.gov.uk/government/publications/personal-protective-equipment-enforcement-regulations-2018/regulation-2016425-and-the-personal-protective-equipment-enforcement-regulations-2018-great-britain]

#### 8.6.2. Legal basis
- Personal Protective Equipment (Enforcement) Regulations 2018 (SI 2018/390) assimilating EU 2016/425.

#### 8.6.3. Who is in scope
- Manufacturers, importers, distributors of PPE.

#### 8.6.4. Sorellon-specific exposure
**Sorellon's PPE catalogue includes**:

- **Hi-vis garments**: Cat II (visibility = risk to life if not seen, despite seeming low-risk).
- **Workwear, gardening gloves**: Cat I.
- **Chemical-resistant gloves for cleaning**: Cat III (chemicals = irreversible harm).
- **Examination gloves used as PPE (blood-borne pathogen protection)**: Cat III AND Class I/IIa medical device (dual classification).
- **Safety footwear**: Cat II usually.
- **Respiratory PPE / FFP masks**: Cat III.

#### 8.6.5. Registration / onboarding mechanics
- No registration — distributor verifies compliance.

#### 8.6.6. Documents to prepare
- Declaration of Conformity (manufacturer-issued).
- EU/UK type-examination certificate (Cat II / III).
- For Cat III: ongoing Notified Body surveillance evidence.
- User instructions in English.
- Pictograms identifying protection categories on the product.

#### 8.6.7. Recurring obligations
- Storage and transport must not compromise PPE conformity.
- Notify market surveillance authority if non-conforming PPE is identified and take corrective action.
- Retain DoC for **10 years** after the last PPE was placed on the market.
- Cooperate with Office for Product Safety and Standards (OPSS) and Trading Standards.

#### 8.6.8. Penalties for non-compliance
- Criminal offences under Enforcement Regulations 2018.
- Unlimited fines on indictment; 3 months imprisonment summary.
- OPSS / Trading Standards can detain goods at port.

#### 8.6.9. Interaction with other regimes
- **MHRA** for dual-use gloves.
- **UKCA / CE marking**.
- **GB CLP** for any chemical decontamination chemicals supplied as part of PPE kit.
- **NHS Supply Chain** PPE framework compliance.

#### 8.6.10. Reseller carve-outs
- Same as 8.4 — distributor verification proportionate; no Sorellon-branded products → no manufacturer burden.

#### 8.6.11. Practical pitfalls
- **Type-examination certificate validity dates** — Cat II/III certificates are typically 5-year terms; expired = product not legally compliant.
- **Cat III ongoing surveillance** — Notified Body must perform annual audits; lapse can void the mark.
- **Disposable gloves dual classification trap**: medical examination glove ≠ industrial nitrile glove ≠ PPE-only glove. Sorellon must know which it is selling.

#### 8.6.12. Verification flags
- `[VERIFY: indefinite CE recognition for PPE — confirmed for Cat I; need to verify Cat II/III still subject to recognition rules]`.
- `[VERIFY: which of Sorellon's gloves are sold as medical, PPE, or both]`.

#### 8.6.13. Recommended action sequence
1. Categorise each PPE SKU into Cat I/II/III.
2. Capture certificate expiry dates.
3. For Cat II/III: confirm Notified/Approved Body in the certificate is still valid.
4. Build a renewal calendar.

---

### 14.7. General Product Safety Regulations 2005 (GPSR) — fallback safety net

#### 8.7.1. What it is
**GPSR 2005** applies a general safety duty to all consumer products not otherwise specifically regulated. Distributors must act with **due care** to ensure they don't supply unsafe products and must cooperate with enforcement authorities. [Source: https://www.gov.uk/government/publications/general-product-safety-regulations-2005/general-product-safety-regulations-2005-great-britain]

#### 8.7.2. Legal basis
- **GPSR 2005** (SI 2005/1803).
- Being supplemented/replaced under the **Product Regulation and Metrology Act 2025** — three consultations closed June 2026 with rolling implementation thereafter. [Source: https://www.osborneclarke.com/insights/uk-government-overhaul-product-safety-regulatory-framework]

#### 8.7.3. Who is in scope
- Distributors, producers, importers of consumer products.

#### 8.7.4. Sorellon-specific exposure
- Most consumer-facing products in scope, but Sorellon is **B2B**. GPSR applies primarily to products supplied to consumers; B2B-to-B2B sales attract reduced GPSR exposure (though sector-specific regulations like REACH/MHRA/BPR all still apply).
- Education stationery, painting kits — sometimes sold to schools who pass to children → consumer exposure.

#### 8.7.5. Registration / onboarding mechanics
- No registration.

#### 8.7.6. Documents to prepare
- Traceability records (from-whom-bought, to-whom-sold).
- Risk assessment for non-CE/UKCA marked products.

#### 8.7.7. Recurring obligations
- Maintain traceability.
- Cooperate with recalls / corrective action.
- Pass-on of hazard information.

#### 8.7.8. Penalties for non-compliance
- Criminal offences. Fines.

#### 8.7.9. Interaction with other regimes
- Where sectoral regs apply (PPE, MDR, BPR, REACH), they take precedence.
- GPSR fills the gap (e.g. workwear without protective function).

#### 8.7.10. Reseller carve-outs
- Distributors not required to actively test products; only to act on knowledge.

#### 8.7.11. Practical pitfalls
- **Big reform incoming under Product Regulation & Metrology Act 2025** — distributor obligations will expand.
- B2B-vs-B2C boundary unclear in some sales channels.

#### 8.7.12. Verification flags
- `[VERIFY: Product Regulation and Metrology Act 2025 secondary legislation publication date — consultation closed 23 June 2026; new distributor obligations likely 2027+]`.

#### 8.7.13. Recommended action sequence
1. Implement traceability SOP (lot-level tracking, customer-level invoice records).
2. Implement a corrective-action procedure.
3. Monitor PRMA 2025 secondary legislation.

---

### 14.8. Importer obligations — HMRC, customs, EORI, commodity codes

#### 8.8.1. What it is
When Sorellon imports goods directly (not buying from a UK-side intermediary), Sorellon becomes the legal importer. This triggers HMRC customs duties, VAT, commodity code declarations, and product-regulatory importer status. [Source: https://clearborder.co.uk/resource/hmrc-importer-requirements/]

#### 8.8.2. Legal basis
- Taxation (Cross-border Trade) Act 2018.
- Various customs SIs.
- HMRC's Customs Declaration Service (CDS) is mandatory since November 2023.

#### 8.8.3. Who is in scope
- Anyone importing goods into GB from outside.

#### 8.8.4. Sorellon-specific exposure
- Sorellon imports from EU and China per profile.
- Each consignment requires CDS declaration with: GB EORI, 10-digit commodity code, customs procedure code.

#### 8.8.5. Registration / onboarding mechanics
- **EORI number**: get a GB-prefixed EORI from https://www.gov.uk/eori. Free. Required before any import.
- **VAT registration**: if Sorellon expects taxable turnover > £90,000 (2026 threshold — `[VERIFY current threshold]`).
- **Customs intermediary** (freight forwarder) typically handles CDS submissions.

#### 8.8.6. Documents to prepare
- Commercial invoice.
- Packing list.
- Bill of lading / airway bill.
- Commodity code per item (HMRC Trade Tariff).
- Certificate of origin (if claiming preferential rate under TCA or other FTA).
- Conformity evidence (CE/UKCA marks for product-regulatory verification at port).

#### 8.8.7. Recurring obligations
- HMRC audit risk: HMRC can review past 3 years' declarations and recover duty + interest + penalties for false declarations.
- Maintain customs records for **6 years**.

#### 8.8.8. Penalties for non-compliance
- Customs duty + interest + penalty for misclassification.
- Anti-dumping duty exposure (Chinese steel: 30–60%; Chinese ceramics; some chemical products).
- Goods can be seized.

#### 8.8.9. Interaction with other regimes
- REACH at port: chemicals can be detained if no GB importer registration.
- MHRA: devices without UKCA/CE may be detained.
- IPAFFS for any animal-origin product (none in Sorellon's catalogue).

#### 8.8.10. Reseller carve-outs
- None as importer.

#### 8.8.11. Practical pitfalls
- **Commodity code errors** — especially for multi-functional products (e.g. is an "antibacterial hand wipe" a paper product (CN 4818) or a chemical product (CN 3402)? Different duty rates).
- **Anti-dumping** on Chinese-origin goods — Sorellon's stationery, certain plastics, certain steel-component PPE may attract additional duty.
- **VAT cashflow** — postponed VAT accounting (PVA) is the default for VAT-registered importers; helps cashflow.
- **Inco-terms** — DDP (delivered duty paid) shifts duty burden to supplier; FOB (free on board) leaves Sorellon as importer.

#### 8.8.12. Verification flags
- `[VERIFY: Sorellon's current EORI and VAT status]`.
- `[VERIFY: commodity codes for each major SKU — request from supplier and cross-check on UK Trade Tariff (https://www.trade-tariff.service.gov.uk)]`.
- `[VERIFY: anti-dumping exposure for Chinese-sourced PPE / stationery / cleaning products]`.

#### 8.8.13. Recommended action sequence
1. Obtain GB EORI immediately if not already done.
2. Engage a customs broker / freight forwarder for first 12 months.
3. Implement a commodity code library per SKU.
4. Set up post-import audit SOP — monthly review of declarations.

---

### 14.9. WEEE / Packaging waste / Plastic Packaging Tax — producer responsibility

#### 8.9.1. What it is
Three intertwined producer-responsibility regimes:
- **WEEE** (Waste Electrical & Electronic Equipment) — for products with batteries / electrical components.
- **EPR for Packaging** (replacing old packaging waste regime) — fees on packaging placed on UK market.
- **Plastic Packaging Tax (PPT)** — tax on plastic packaging with <30% recycled content.
[Sources: https://www.innovent-recycling.co.uk/weee-regulations-2026-uk-business-guide/, https://www.thewalpole.co.uk/news/packaging-epr-pepr-guidelines-2026, https://erp-recycling.org/uk/news-and-events/2026/01/plastic-packaging-tax-changes-in-2026/]

#### 8.9.2. Legal basis
- WEEE: Waste Electrical and Electronic Equipment Regulations 2013.
- EPR Packaging: Producer Responsibility Obligations (Packaging and Packaging Waste) Regulations 2024.
- PPT: Plastic Packaging Tax (Finance Act 2021).

#### 8.9.3. Who is in scope (Sorellon's product line)

**WEEE** — applicable if Sorellon distributes any EEE. Sorellon's catalogue may include:
- Battery-powered cleaning equipment (`[VERIFY]`).
- Electrical janitorial equipment (`[VERIFY]`).
- Most of Sorellon's catalogue is **NOT EEE** — exposure is **LOW**.

**EPR Packaging** — thresholds:
- £1M turnover + 25 tonnes packaging (small producer); £2M + 50 tonnes (large producer).
- Sorellon (pre-revenue) is currently below threshold; will likely remain so for first year.

**PPT** — 10-tonne plastic packaging threshold per 12-month period — Sorellon may approach this if importing high-volume packaged products.

#### 8.9.4. Sorellon-specific exposure
- **EPR Packaging**: HIGH future exposure once revenue scales. Plastic-heavy packaging on wipes/gloves/cleaning chemicals adds up fast.
- **PPT**: MEDIUM. Many plastic films and bottles imported with goods will be <30% recycled content → 10-tonne threshold may be hit faster than expected.
- **WEEE**: LOW unless cleaning equipment included.

#### 8.9.5. Registration / onboarding mechanics
- **EPR**: register with Environment Agency once threshold crossed.
- **PPT**: register with HMRC once 10-tonne threshold met.
- **WEEE**: join a compliance scheme (e.g. Valpak, ERP, B2B Compliance) if EEE producer.

#### 8.9.6. Documents to prepare
- Packaging data: type (plastic/paper/glass/metal/wood/composite), weight, recyclability rating, country of origin, sale destination (UK nation breakdown from 2026 reporting).
- Plastic recycled content evidence (certificates) — for PPT exemption claims.
- EEE category data.

#### 8.9.7. Recurring obligations
- **EPR**: quarterly or annual data reporting; pay producer fees based on recyclability (RAG rating from 2026-2027).
- **PPT**: quarterly returns.
- **WEEE**: tonnage reporting via compliance scheme.

#### 8.9.8. Penalties for non-compliance
- Fines, interest, recovery of unpaid taxes.

#### 8.9.9. Interaction with other regimes
- Many — esp. waste regulations, environment policy expectations from public sector buyers.

#### 8.9.10. Reseller carve-outs
- Under EPR: "distributor" exists as a separate role with reduced obligations (only liable when supplying unfilled packaging onward). Most Sorellon trade is **filled packaging onwards** so Sorellon is treated as a producer for that packaging.

#### 8.9.11. Practical pitfalls
- **Imported packaging is counted under PPT** — Sorellon importing wrapped goods from China is the "first owner" of that packaging in GB for PPT purposes.
- **Threshold creep** — easy to cross £1M / 25-tonne without realising.

#### 8.9.12. Verification flags
- `[VERIFY: any EEE in Sorellon's catalogue — battery-powered cleaning gear, etc.]`.
- `[VERIFY: PPT 10-tonne tracker — Sorellon's import volumes in plastic packaging]`.

#### 8.9.13. Recommended action sequence
1. Track packaging weights per import shipment from day one.
2. Track EEE per SKU (count to zero is the realistic position).
3. Plan EPR/PPT registration triggers into year-1 finance review.

---

### 14.X. Sorellon product-by-product regulatory exposure matrix

| Product family | UKCA/CE | MHRA | BPR | PPE Reg | REACH | CLP | GPSR | EPR/PPT |
|---|---|---|---|---|---|---|---|---|
| Disposable nitrile examination gloves | Required (CE/UKCA dual MD+PPE) | Class I or IIa | N/A | Cat III | N/A | N/A | N/A | Packaging |
| Surgical gloves (sterile) | Required (NB-certified) | Class IIa | N/A | Cat III | N/A | N/A | N/A | Packaging |
| Hi-vis workwear | CE/UKCA (Cat II PPE) | N/A | N/A | Cat II | N/A | N/A | N/A | Packaging |
| Chemical-resistant gloves | CE/UKCA (Cat III PPE) | N/A | N/A | Cat III | N/A | N/A | N/A | Packaging |
| Surface disinfectant wipes (EN1276/EN14476 claims) | N/A | N/A | **REQUIRED** | N/A | Active subst. via REACH | Required label | N/A | Packaging + PPT |
| Hand sanitiser | N/A | N/A | **REQUIRED** | N/A | Required (ethanol) | Required label | N/A | Packaging + PPT |
| Bleach / chlorine cleaner | N/A | N/A | If marketed as disinfectant | N/A | Required | Required label | N/A | Packaging + PPT |
| Glass cleaner (no biocidal claim) | N/A | N/A | N/A | N/A | If imported & ≥1t/yr | Required label | N/A | Packaging + PPT |
| Disposable syringes (basic) | Required (CE/UKCA) | Class IIa | N/A | N/A | N/A | N/A | N/A | Packaging |
| Urinary catheters | Required (CE/UKCA) | Class IIa | N/A | N/A | N/A | N/A | N/A | Packaging |
| Basic dressings (non-sterile) | Required (CE/UKCA) | Class I | N/A | N/A | N/A | N/A | N/A | Packaging |
| Janitorial paper products | N/A | N/A | N/A | N/A | N/A | N/A | Maybe | Packaging |
| Painting/maintenance kits | Variable | N/A | N/A | Maybe | If chemical | If chemical | Variable | Packaging |
| Education stationery | N/A (unless toy) | N/A | N/A | N/A | If chemical (e.g. glues) | If chemical | Maybe | Packaging |

---

## Most Critical Regulatory Exposures — Operator Priority Brief

The single highest-risk exposure is **GB BPR enforcement on Sorellon's antimicrobial wipes and cleaning products carrying EN1276 / EN14476 / "antibacterial" / "virucidal" claims**: each product must be GB-authorised with an HSE-approved active substance sourced from a GB Article 95-listed supplier, and unauthorised placement is a criminal offence with unlimited fines (HSE actively enforces post-COVID). Second tier risks: **(a) MHRA distributor obligations on Chinese-sourced gloves, syringes, catheters and dressings** — Sorellon must verify each manufacturer has a UK Responsible Person and MHRA registration, retain Declarations of Conformity, log every complaint as potential vigilance, and never white-label without assuming full manufacturer status; **(b) UK REACH importer reclassification** for any cleaning chemicals sourced from EU/China without a UK Only Representative — Sorellon becomes the importer with substance-level tonnage tracking and registration burden if any substance crosses 1 t/yr; **(c) Procurement Act 2023 mandatory exclusion grounds** including modern slavery and cartel infringement — disclosure failures on the Central Digital Platform trigger Fraud Act exposure; **(d) UKCA/CE conformity verification** — Chinese "CE" stickers may be invalid (China Export logo or self-declared Class IIa without Notified Body), and distributor liability is real. The **NHS Supply Chain PPE framework re-tender in Sep 2026** and **Hand Hygiene framework re-tender in Jun 2026** are time-critical commercial windows.

---

## 15. Financial & Operational Maturity

UK public procurement evaluators apply a "financial standing" test to every supplier. The threshold is normally a turnover-to-contract-value ratio (typically 2x or 3x annual turnover ≥ contract value), supported by filed accounts, bank references, and credit ratings. **Sorellon Limited has none of these in conventional form** — it's a ~10-month-old pre-revenue company with first accounts not due until 2027-04-10. This section covers the alternatives.

### 15.1. Business bank account

**Status for Sorellon**: Unknown from `sorellon.md` (the company-identity section doesn't list a bank). **Verify with operator** — most UK procurement applications require bank account details on the supplier registration form.

**Why it matters**:
- VAT registration prerequisite — HMRC requires business bank details on the VAT1 form
- ICO Data Protection Fee — required for direct debit payment
- Insurance brokers ask for it during underwriting
- Procurement portals ask for it for award payment routing
- Credit-rating agencies need it for company-data verification

**Options for a new UK Ltd**:

| Provider | Account type | Notes | Setup time |
|---|---|---|---|
| **Tide** | Business current account | Online-only; free Plus tier; fastest setup (often 1-2 days); accepts new Ltd's | 1-2 days |
| **Starling Bank Business** | Full business account | UK bank licence; free Sole Trader / Business plans; integrates with Xero/QuickBooks; rigorous KYC | 3-7 days |
| **Revolut Business** | Multi-currency business | Good for import payments (Sorellon imports from China/EU); higher fees on FX | 1-3 days |
| **Mettle (NatWest brand)** | UK SME-focused | Free; integrates with FreeAgent; UK High Street bank parent | 2-7 days |
| **Monzo Business** | Lite or Pro tiers | UK bank; Pro tier £5/mo; integrations | 1-3 days |
| **HSBC Kinetic / Barclays / NatWest** | Traditional High Street | Slower (2-4 weeks); often require in-branch ID check; older account = better for credit applications later | 2-4 weeks |

**Sorellon-specific notes**:
- Operator should set up at least ONE account NOW before applying for VAT/insurance/credit rating
- Strongly consider a fintech (Tide / Starling) for speed, plus optionally a High Street account for prestige (NatWest Mettle is the hybrid)
- KYC will require: Director ID (Zac Farrell's passport), PSC ID (Zac + Dieter), Companies House confirmation document, proof of registered office (utility bill or Companies House filing acceptable for residential), business activity description (use the SIC + product list from sorellon.md)
- For multi-currency import payments (China supplier), Revolut Business OR Wise Business is significantly cheaper than UK High Street banks on FX

**Sequencing**: Priority 0 — must be open before any other application. **Recommended choice: Tide Plus** for speed (often 24h), with optional Wise Business added later for FX.

[VERIFY: Tide's "Plus" free tier vs paid "Business" tier features 2026]

### 15.2. Credit rating / company credit file

**What it is**: A score (typically 0-100) assigned by a credit reference agency based on a company's filed accounts, payment behaviour, director history, sector risk, etc. UK public procurement bid evaluators check at least one of these scores; some explicitly require a minimum (often 50 or "low risk").

**Major UK CRAs**:

| Agency | URL | Self-service supplier portal | What they score |
|---|---|---|---|
| **Experian** | experian.co.uk/business | Yes — Experian Business Express | Detailed financial + payment + director |
| **Equifax** | equifax.co.uk/business | Yes — Equifax Business | Similar; less procurement penetration |
| **Creditsafe** | creditsafe.com/gb | Yes — public company report | Most procurement-friendly; Sorellon-specific score visible to anyone |
| **Dun & Bradstreet** | dnb.com/en-gb | Yes — D&B Hoovers | International (DUNS — Sorellon HAS a DUNS per sorellon.md: 233967763) |
| **Companies House Direct** | companieshouse.gov.uk | Free filings | No score; just filing data |

**Sorellon's likely starting state**:
- D&B: DUNS issued (233967763 per `sorellon.md`). D&B score likely "Insufficient data" or "Limited" — newly incorporated with no filed accounts
- Experian, Equifax, Creditsafe: minimal data (incorporation + officers) — score either absent or in the low/very-low risk band by default (a NEW UK Ltd with no negative signals often defaults to ~30-60 risk band depending on agency)
- For Sorellon's first 12-18 months, **most evaluators will see "limited credit information"** rather than a strong score

**Mitigations / what evaluators accept instead**:
- **Director financial standing statement** — signed declaration by Zac Farrell that the director has personal financial resources sufficient to support contract delivery. Specimen wording is in CCS supplier financial standing guidance.
- **Parent company guarantee** — N/A for Sorellon (no parent)
- **Personal guarantee from director** — common for new businesses but exposes Zac personally; usually only required for very large contracts
- **Independent audit / management accounts** — see §9.5
- **Bank reference letter** — explicit letter from bank confirming account in good standing; usually free or ~£20

**Specific action items for Sorellon**:
1. Claim and verify Sorellon's profile on each of the 4 agencies (free for the company itself)
2. Submit any updates (correct address, products, contact)
3. Set up monitoring alerts (free or low-cost) so the operator gets notified if a score drops
4. Build up trade payment history quickly — even small purchases from suppliers paid promptly improve scores faster than turnover

**Cost**:
- Experian Business Express: ~£24.99/month for ongoing monitoring
- Creditsafe: free basic; ~£50-100/month for full reports of self + others
- D&B: DUNS is free; DUNSRegistered is free; D&B Hoovers paid
- Equifax: free claim; paid monitoring ~£20/month

**Sequencing**: Priority 1 — after bank account, claim CRA profiles immediately. Takes <1 hour total across all 4 agencies.

[VERIFY: Current 2026 pricing on Experian Business Express, Creditsafe Premium]

### 15.3. Capital adequacy / minimum company net worth

**The problem**: Sorellon's `NEWINC` filing shows statement of capital = **GBP 1** (single share, model articles). This is technically valid but signals "shell company" to bid evaluators.

**What evaluators actually check**:
- Net assets ≥ contract value (sometimes ≥ 2x)
- Working capital ratio (current assets / current liabilities) ≥ 1.0
- Acid test ratio (cash + receivables / liabilities) ≥ 1.0
- Debt-to-equity ratio < 2.0

**For a pre-revenue Ltd with £1 share capital**, all of these fail.

**Options to demonstrate capital**:
1. **Director's loan to the company** — Zac Farrell lends Sorellon £X (some realistic figure, e.g., £20K-£50K) and records it as a director loan on the balance sheet. This shows working capital immediately. Loan should be documented (loan agreement, board minute).
2. **Increase share capital** — file SH01 form to allot additional shares; pay them up (real cash from director to company); update statement of capital. £1 → £20,000 is a one-time filing. Doesn't change ownership if Zac/Dieter retain proportions.
3. **Director's personal financial standing statement** — fall-back: declare on letterhead that director has personal resources sufficient for contract scale. Less convincing than capital on the balance sheet but accepted by many SME-friendly buyers.
4. **Capital adequacy ratio guidance** — Sorellon should aim for net assets ≥ £20,000 minimum before bidding for contracts above £50K, and ≥ £100,000 before bidding above £200K. These are rules-of-thumb, not statutory.

**Sorellon-specific notes**:
- Zac and Dieter need to discuss who can / should put working capital into the company
- A director's loan is the cleanest mechanism (reversible, doesn't dilute ownership)
- The loan should appear on the next set of accounts (first management accounts can be prepared even before statutory accounts are due)

**Sequencing**: Priority 2 — before bidding for contracts >£50K, or before being asked for financial standing evidence.

[VERIFY: SH01 filing fee and process — historically £15 paper / free online]

### 15.4. Sub-contracting / consortium options for pre-revenue Sorellon

For procurement opportunities where Sorellon doesn't yet have the financial standing / certifications to bid as prime, sub-contracting or consortium structures are legitimate workarounds.

**Sub-contracting (Sorellon as sub-tier supplier to a prime)**:
- Sorellon supplies goods to a larger prime contractor who bids the main contract
- Prime carries the contracting risk + financial standing requirements
- Sorellon's responsibility is only to the prime, not to the public buyer directly
- Sorellon builds trading history without needing certifications/insurance levels of the prime

**Consortium / joint venture (Sorellon partners with another supplier)**:
- Two or more SMEs jointly bid as a "consortium"
- One member acts as lead (handles administration)
- Each member's financial standing + certifications are aggregated for evaluation
- Sorellon's combined offer with a stronger partner can clear thresholds it couldn't alone
- More complex to administer; consortium agreement needs proper legal drafting (~£500-£1500 in legal fees)

**Where to find consortium partners**:
- **Federation of Small Businesses (FSB)** — fsb.org.uk — has member networks for SME collaboration
- **British Chambers of Commerce** — local chambers; relevant for regional procurement
- **CCS supplier journey events** — gov.uk events bringing SMEs together
- **Procurement networking events** — Bid Solutions, FacilitiesShow, IFSEC
- **Sector trade associations** — e.g., BICSc (cleaning industry), BHTA (healthcare products)

**Pre-bid step**: Sorellon should identify 2-3 potential consortium partners NOW (suppliers in adjacent categories who would benefit from joining offers) and have informal conversations.

**Sorellon-specific consortium opportunities**:
- A larger cleaning chemicals supplier might want a PPE/janitorial partner
- A facilities-management contractor might need a goods-supply tier supplier
- A small NHS-focused medical-supply company might value Sorellon's broader cleaning/janitorial range

**Sequencing**: Parallel to other readiness work. Start identifying partners in week 1-2; don't wait for formal certifications.

### 15.5. Annual accounts / management accounts strategy

**Statutory position**: Sorellon's first statutory accounts cover incorporation (2025-07-10) through 2026-07-31, due to Companies House by 2027-04-10. This is the standard timeline for a new Ltd. Filing them is mandatory.

**Problem for bidding**: Most procurement applications require either:
- "Last 2 years' filed accounts" — Sorellon won't have ANY filed accounts until 2027
- OR — "Recent management accounts signed by director" as an interim
- OR — for very new businesses, a "Director's financial standing statement"

**Recommended approach**:
1. **Engage an accountant** (essential) — for any procurement-grade financial documentation. Costs ~£500-£2000/year for a small Ltd. Online accountants like Crunch, FreeAgent's accountant network, or local chartered accountants work. **Verify with operator if accountant is already engaged.**
2. **Prepare interim management accounts** by ~6 months after incorporation showing:
   - Profit & Loss for period
   - Balance sheet
   - Director's commentary
   - Signed by director (Zac) and independently reviewed by accountant
3. **Optional: Independent letter of comfort** from accountant about the company's financial position — supplements management accounts
4. **First statutory accounts** — file with Companies House by 2027-04-10. Once filed, they become the de facto financial standing evidence for the next ~18 months.

**Cost**:
- Accountant retainer: ~£100-£200/month for a basic small-Ltd package
- Management accounts preparation: ~£200-£500 per set
- Independent letter of comfort: ~£100-£250

**Sequencing**: Engage accountant in week 1. Have first management accounts ready by month 6 (around Jan 2026 if engaged now).

### 15.6. Case studies / contract references strategy

Sorellon has NO contract references yet (per sorellon.md §"User-Confirmed Status Updates" line 379). Most procurement evaluations ask for **3 contract references** showing similar size, scope, and timeframe.

**What "contract reference" means in UK procurement**:
- Contract value (often must be ≥ 25-50% of the contract being bid)
- Contract dates (must be recent — typically within 3-5 years)
- Customer name + contact (so buyer can call to verify)
- Brief description of scope
- Whether successfully completed

**Sorellon options**:
1. **Build a "near references" file** — Zac Farrell's prior employment history if it includes wholesale / procurement work. Any prior business of his. Personal references aren't the same as company references but can supplement.
2. **First contracts strategy** — deliberately bid for very small public contracts (sub-£10K) that ask only for testimonial-style references. Successful delivery becomes Reference 1. Repeat 2 more times. Then bid larger.
3. **Private-sector references** — if Sorellon does ANY B2B private-sector business while building public history, those become valid references for most public procurement evaluations. A small business-to-business order delivered well, with the buyer willing to be a reference, counts.
4. **Sub-contractor references** — if Sorellon sub-supplies via a prime (per §9.4), the prime can provide a "sub-tier reference" describing Sorellon's reliability. Not as strong as direct contract reference but better than nothing.

**Sorellon-specific note**: The fastest path to 3 valid references is the small-contract strategy (#2). Target Dynamic Purchasing Systems (DPS) and lots under £10K initially.

**Sequencing**: Ongoing from week 1. Plan to have first 3 references within ~12 months.

---

## 16. Standalone Policies & SQ/PQQ Documents

UK Selection Questionnaire (SQ — formerly PQQ) responses typically require named standalone documents. Sorellon already has a strong 11-policy suite (per `sorellon.md` §"Policy And Evidence Register"). This section covers the GAPS — documents commonly asked but not yet in Sorellon's pack.

### 16.1. Documents Sorellon ALREADY has (no action needed)

These are READY per `sorellon.md`:
- ✓ Anti-Bribery and Corruption Policy (1 Jan 2025, reviewed Mar 2026)
- ✓ Business Continuity Policy
- ✓ Carbon Reduction Plan (DRAFT — baseline numbers TBC; see §10.7 below)
- ✓ Environmental Policy
- ✓ Equality and Diversity Policy
- ✓ Health and Safety Policy
- ✓ Information Security and Cyber Security Policy
- ✓ Modern Slavery Statement
- ✓ Privacy Policy
- ✓ Terms and Conditions of Supply
- ✓ Whistleblowing Policy

**Note**: All are dated 1 Jan 2025 with March 2026 reviews. Buyers generally accept policies up to 12 months old. Sorellon needs to re-review and re-date all of these by **January 2027** at the latest to avoid the "stale policy" risk in mid-2027 bids.

### 16.2. Social Value statement (NEW — needs drafting)

**What it is**: A written commitment by the supplier describing how its operations create social, environmental, and economic value beyond the contract itself. Required under the Procurement Act 2023's **Social Value Model** (originally PPN 06/20, retained and strengthened).

**Why it matters**: Public procurement evaluations now weight Social Value at minimum 10% (most central government), often 20-30% (local authorities, NHS). Without a Social Value statement, Sorellon scores 0 on this section.

**Recommended structure** (~3-5 pages):
1. **Mission alignment** — How Sorellon's purpose ties to UK public-sector priorities (responsible sourcing, supporting NHS/education/local government)
2. **Theme 1 — Tackling economic inequality** — Commitments around creating new jobs, training apprentices, supporting SME sub-suppliers
3. **Theme 2 — Fighting climate change** — Tie to Carbon Reduction Plan; specific commitments (e.g., consolidated deliveries to reduce road miles; recyclable packaging)
4. **Theme 3 — Equal opportunity** — Tie to Equality & Diversity Policy; commitments around supplier diversity, inclusive hiring
5. **Theme 4 — Wellbeing** — Employee mental health support; community engagement; product safety (PPE quality)
6. **Theme 5 — COVID-19 recovery** (still referenced in some templates) — Supporting public-sector resilience
7. **Measurement commitments** — How Sorellon will report progress (annually, against KPIs)

**Sources**:
- Cabinet Office PPN 06/20 (Social Value Model): gov.uk/government/publications/procurement-policy-note-0620-taking-account-of-social-value-in-the-award-of-central-government-contracts
- Social Value Portal: socialvalueportal.com (commercial tooling for measuring SV)
- TOMs Framework: socialvalueportal.com/toms (the "Themes Outcomes Measures" framework)

**Cost**: £0 if Sorellon drafts in-house (recommended; ~4-8 hours of director time). £500-£2000 for a consultant-drafted version.

**Sequencing**: Priority 1 — needed for nearly every UK public bid. Should be drafted within 30 days of starting bid pipeline.

### 16.3. Quality Management System (QMS) procedure — for buyers who don't require ISO 9001

**What it is**: A documented procedure describing how Sorellon manages product/service quality across the order lifecycle (supplier selection, order placement, goods inwards inspection, delivery quality, customer complaint handling, continuous improvement).

**Why it matters**: Many SME-friendly procurement evaluations accept a documented QMS procedure as an alternative to ISO 9001 certification (which is expensive — see Agent 3's research for ISO 9001 costing). The procedure must be:
- Written down (not just claimed)
- Show clear ownership (named roles)
- Have a feedback loop (corrective actions, continuous improvement)
- Be evidenced in practice (records of the procedure being followed)

**Recommended structure**:
1. Quality policy statement (1 page — high-level commitment)
2. Roles and responsibilities (sole director currently; expandable as company grows)
3. Process maps for: supplier qualification, order intake, sourcing, inspection, dispatch, post-delivery
4. Records to be kept (supplier audits, batch records, complaint logs)
5. Internal audit schedule (quarterly self-audit by director)
6. Management review (annual)
7. Corrective and preventive action workflow

**Cost**: £0 in-house (~8-16 hours director time). £500-£1500 if consultant-drafted.

**Sources**:
- ISO 9001:2015 standard structure (free summaries available; the standard itself is ~£100 from BSI Shop)
- Made Simple template providers (Quality Management System Ltd, ISO 9001 Made Simple) — but be cautious about "self-certified ISO 9001" claims; only use templates as inspiration

**Sequencing**: Priority 2 — needed within 60 days. Can be drafted in parallel with Cyber Essentials work.

### 16.4. GDPR Data Processing Agreement (DPA) template

**What it is**: A template Data Processing Agreement (Art 28 GDPR) that Sorellon can offer when public buyers ask "do you have a DPA template you'd use with us as data controller?"

**Why it matters**: Public buyers process personal data and need their suppliers to commit to specific security + handling obligations. Without a DPA template, Sorellon either signs the buyer's version (unknown terms) or delays the bid.

**Recommended sources**:
- **ICO Standard Contractual Clauses (SCCs)** — UK ICO publishes free model contractual clauses for controller-to-processor scenarios. URL: ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/international-transfers/international-data-transfer-agreement-and-guidance
- **EU SCCs (Commission Decision 2021/914)** — still valid in UK for transfers but explicitly the UK ICO SCCs are preferred for UK-only scenarios
- **Free SME templates** — Sponge ICO-approved DPAs; Termly; SEQ Legal templates (~£99 for branded version)

**Sorellon-specific notes**:
- Sorellon is mostly a goods supplier — personal data flows are limited (buyer's order contact details + delivery addresses + possibly recipient names)
- Article 30 record (RoPA) needs to align with the DPA — list the categories of personal data Sorellon processes for each buyer
- Sub-processors: Sorellon uses Google Workspace (Gmail, Drive — Google is a sub-processor) and Netlify (website hosting — Netlify is a sub-processor). DPA template MUST list these.

**Cost**: £0 if using ICO templates + customisation. £99-£500 if using a commercial template provider.

**Sequencing**: Priority 2 — needed within 60 days. Quick to draft once Article 30 RoPA is done.

### 16.5. Complaints procedure (standalone document)

**What it is**: A written procedure describing how Sorellon handles customer complaints, including timescales, escalation paths, and outcome reporting.

**Why it matters**: Some buyers ask for this explicitly in SQ Section "Customer Care". Sorellon's existing T&C document may reference complaints informally but a standalone procedure is the safer bid asset.

**Recommended structure** (~1-2 pages):
1. Scope (all customers, all order types)
2. Channels (email sales@sorellon.com; phone 07950265101; via portal where applicable)
3. Acknowledgement timeline (e.g., 1 business day)
4. Investigation timeline (e.g., 5 business days for non-urgent; 24h for urgent / safety-related)
5. Resolution levels (rep — director escalation — formal complaint)
6. Outcome reporting (customer informed in writing; corrective action recorded in QMS)
7. Complaint records retention (3 years minimum)
8. Continuous improvement feedback loop

**Cost**: £0 in-house (~2-4 hours).

**Sequencing**: Priority 3 — within 90 days. Easy to draft.

### 16.6. Safeguarding policy (for education / healthcare contexts)

**What it is**: A written policy describing how Sorellon protects vulnerable people (children, vulnerable adults) it might encounter in delivering goods to schools, care homes, NHS facilities.

**Why it matters**: Mandatory for any bid involving delivery to:
- Schools (state, MAT, academy, independent)
- Children's homes / care leavers
- Care homes for vulnerable adults
- NHS facilities (some categories)
- HMP / probation services

Some buyers reject bids without a current safeguarding policy out of hand.

**Recommended structure** (~2-4 pages):
1. Scope and commitment statement
2. Definitions (vulnerable adult, child, safeguarding concern, abuse, exploitation)
3. Named safeguarding lead (currently Zac Farrell — sole director — must designate himself)
4. DBS checks — Sorellon's position (sole employee; no staff working with vulnerable people; goods delivery only; no direct service contact) — this needs to be stated clearly
5. Procedures for staff entering safeguarded premises (e.g., never alone with children/vulnerable adults; sign-in protocols; ID badges)
6. Reporting concerns (Local Authority Designated Officer / LADO route for children; Local Safeguarding Adults Board for adults; police 999 for immediate harm)
7. Training requirements (basic safeguarding awareness)
8. Policy review cadence (annual)

**Sources**:
- UK Government safeguarding guidance: gov.uk/government/publications/working-together-to-safeguard-children
- NHS safeguarding adults frameworks: nhs.uk/safeguarding
- LSCB / LADO regional contacts

**Cost**: £0 in-house (~4-6 hours).

**Sequencing**: Priority 3 — within 90 days, ESPECIALLY before any education/care/NHS bid.

### 16.7. Supply chain / ethical sourcing standalone statement

**What it is**: A standalone (not embedded in another policy) document committing to supply chain due diligence, ethical sourcing, modern-slavery awareness, and supplier auditing.

**Why it matters**: Increasingly mandatory in public bids especially for goods supply. Even though Sorellon has a Modern Slavery Statement, a separate "Ethical Sourcing Statement" is often a separate evaluation tickbox.

**Recommended structure** (~2-3 pages):
1. Commitment statement (responsible sourcing, no exploitation, transparency)
2. Supplier qualification criteria (vetting before onboarding)
3. Code of conduct (what suppliers must commit to: labour rights, environmental, anti-bribery, modern slavery)
4. Audit programme (frequency, scope, what triggers an audit)
5. Country / category risk-rating approach (especially for China-sourced products — higher-scrutiny country)
6. Remediation approach if a violation is found
7. Cross-references to: Modern Slavery Statement, Carbon Reduction Plan, Environmental Policy
8. Reporting cadence

**Sources**:
- Ethical Trading Initiative (ETI) Base Code: ethicaltrade.org/eti-base-code
- BSCI / amfori — commercial frameworks
- Achilles supplier-due-diligence frameworks

**Cost**: £0 in-house (~4-6 hours).

**Sequencing**: Priority 2 — within 60 days.

### 16.8. Logistics / delivery capability statement

**What it is**: A document describing Sorellon's logistics + delivery model — couriers used, warehousing arrangements, lead times by category, multi-site delivery capabilities, returns process, emergency escalation.

**Why it matters**: Public buyers (especially NHS, councils with multi-site requirements) need confidence in delivery reliability. Sorellon currently has NO formal warehousing (per sorellon.md §"Operational Notes" — warehousing/consolidation may be used where commercially appropriate) — this needs to be honestly stated.

**Recommended structure** (~2-4 pages):
1. Order intake and processing (channels, hours, response times)
2. Sourcing model (just-in-time vs stockholding; supplier diversity; multi-source for critical items)
3. Warehousing — current state honest statement: "We operate a flexible warehousing model with consolidation partners; we do not currently hold strategic stockholding. We can arrange dedicated stockholding for committed supply contracts." Or alternative truthful framing.
4. Courier / logistics partners used (DPD, Parcelforce, Royal Mail Tracked, dedicated couriers for fragile / temperature-controlled)
5. Multi-site delivery capability (geographic coverage; split delivery; consolidated delivery)
6. Standard lead times by category (with caveat that they're indicative)
7. Returns / replacement process
8. Emergency / urgent order escalation (24h dispatch options)
9. Tracking and proof of delivery

**Sources**:
- Examples from Sorellon's website (capability-statement.pdf) likely already covers some of this — pull from there.
- BICSc supplier guidance for cleaning/janitorial supply (bicsc.org.uk)

**Cost**: £0 in-house (~3-5 hours).

**Sequencing**: Priority 2 — within 60 days.

### 16.9. Carbon Reduction Plan baseline data completion

**Status**: Sorellon has a Carbon Reduction Plan in DRAFT (per sorellon.md). Baseline emissions figures (Scope 1, Scope 2, material Scope 3) are TBC. The plan was published March 2026 with a 2025-2030 commitment period.

**Why this matters now**: Per PPN 06/21, ALL UK central government contracts valued ≥£5M (and many smaller) require:
- A published Carbon Reduction Plan
- Baseline emissions for Scope 1, Scope 2, and material Scope 3 categories
- Reduction targets aligned with Net Zero 2050 (preferably earlier)
- Annual public reporting

A CRP with TBC baseline is a "draft" — it's not procurement-ready.

**Recommended approach**:
1. **Calculate baseline emissions** for the 2025 calendar year:
   - Scope 1: direct emissions (any company vehicles, on-site fuel use). Sorellon likely has zero Scope 1 since it doesn't own vehicles.
   - Scope 2: indirect energy (electricity at registered office). If Sorellon doesn't pay electricity (residential address), Scope 2 ≈ 0.
   - Scope 3 categories — material ones for Sorellon: business travel (Cat 6); upstream transportation (Cat 4 — goods Sorellon imports); waste in operations (Cat 5).
2. **Use a free calculator**:
   - SME Climate Hub (smeclimatehub.org) — free template
   - Carbon Trust SME calculator
   - The Net Zero Reporting Standard (NRS) for SMEs (gov.uk publication)
3. **Publish on Sorellon's website** as a versioned PDF (already linked from website per sorellon.md)
4. **Update CRP** with finalised baseline figures, dated re-issue

**Cost**: £0 if Sorellon calculates in-house. £500-£2000 for a consultant-prepared calculation.

**Sequencing**: Priority 2 — within 60 days. CRP is on the website but its credibility depends on baseline numbers being real.

### 16.10. Document maintenance cadence

Once all policies are in place, Sorellon needs a maintenance discipline:

| Document | Review cadence | Update trigger | Owner |
|---|---|---|---|
| Anti-Bribery, Equality, H&S, InfoSec, Whistleblowing, Privacy, Modern Slavery, Business Continuity, Environmental, T&C | Annual | Major regulatory change OR yearly anniversary | Zac Farrell (director) |
| Carbon Reduction Plan | Annual update (per PPN 06/21); 5-yearly major review | Each year's reporting requirement | Zac |
| Social Value Statement | Annual (align with bids cycle) | New theme additions / target updates | Zac |
| QMS procedure | Quarterly internal audit; annual review | Any complaint / corrective action triggers earlier review | Zac |
| DPA template | Annual; immediate if sub-processors change | New sub-processor onboarded / GDPR guidance change | Zac |
| Complaints procedure | Annual | Any complaint that exposes a gap | Zac |
| Safeguarding | Annual | LADO / DBS regime change | Zac |
| Ethical sourcing statement | Annual | New high-risk supplier onboarded | Zac |
| Logistics capability | Semi-annual | New delivery partner / warehouse arrangement | Zac |

**Centralised location**: Recommend Sorellon set up a Google Drive folder `Sorellon — Bid Documents/<YYYY-MM>/` with the current version of every document, dated. Each bid response pulls from this single source of truth.

**Calendar reminders**: Set Google Calendar yearly recurring reminders for the major review dates. The annual-review burden is ~2-4 days per year of director time. Plan for this.

---

## 17. Open Questions for Dieter

These are decisions only the operator (Dieter) can make. The research describes the trade-offs; the choice is operational/strategic. Answers will drive whether specific path-B options become path-A in the 90-day plan.

### 17.1. Strategic scope

**Q1**: Does Sorellon intend to bid for MoD or defence-prime contracts in years 1-2?

- If YES: register full JOSCAR (low-thousands/yr) and budget for Cyber Essentials Plus (~£2,250) early
- If NO: free JOSCAR SME Portal entry only; defer CE+

**Q2**: Does Sorellon intend to bid for utilities-sector contracts (Anglian Water, National Grid, Severn Trent, etc.)?

- If YES: Achilles UVDB Silver+ in months 6-12 (~£800/yr)
- If NO: skip

**Q3**: Is Sorellon's target buyer mix predominantly local authorities (district/borough councils), or NHS, or central government?

- LA-heavy: prioritise Constructionline + SMAS + Living Wage
- NHS-heavy: prioritise Atamis + NHS Evergreen + ISO 13485-equivalent demonstrations
- Central-gov-heavy: prioritise CDP/FTS + Cyber Essentials + ISO 9001 path

**Q4**: Is Sorellon open to **consortium / sub-contractor** roles (vs always bidding as prime) for the first 12 months?

- If YES: more bids accessible faster (prime covers financial standing); fewer references needed
- If NO: must invest more upfront in financial standing + insurance limits

### 17.2. Hiring intent

**Q5**: Does Sorellon intend to hire any employees (full-time or part-time) in the next 12 months?

- If YES: trigger HMRC PAYE setup ~4 weeks before first payday; trigger Employers' Liability insurance (statutory £5M); trigger auto-enrolment pension preparation; trigger Living Wage compliance for the new hire
- If NO: Zac remains sole director-on-salary (or pure dividends); skip PAYE entirely

**Q6**: If hiring, will any role involve direct contact with vulnerable people (children, care leavers, vulnerable adults in NHS facilities)?

- If YES: DBS checks required; safeguarding policy becomes operational (not just bid-asset); additional training records
- If NO: Sorellon's bid-asset safeguarding policy (item 10.6) suffices

### 17.3. Capital

**Q7**: Is Zac (or Dieter) willing and able to inject ~£20,000-£50,000 working capital into Sorellon as a director's loan?

- If YES: clean fix for capital adequacy concerns; required for bids above ~£50k contract value
- If NO: rely on director's financial-standing statements (less convincing to buyers) + sub-contractor route

**Q8**: Is the company's current £1 share capital intentional, or should it be increased via SH01 (additional share allotment, ~£15 filing fee)?

- Increasing share capital is the cleanest signal of seriousness but commits the capital permanently
- Director's loan is reversible

### 17.4. Banking + accounting

**Q9**: Does Sorellon currently have a UK business bank account? If yes, with which bank?

- This is genuinely the most important pre-revenue operational question — many of the research items have a bank-account prerequisite
- If NO: action item is week 1 of the 90-day plan

**Q10**: Is Sorellon currently working with an accountant?

- If YES: name? Have they been briefed on the bid-readiness timeline?
- If NO: engage one in week 1 (recommended firms: a local chartered accountant familiar with public-sector clients, OR an online accountant like Crunch / FreeAgent partner network)

### 17.5. Product line specifics — drives regulatory exposure

**Q11**: Does Sorellon currently hold (or has it sold) any stock of **antimicrobial surface wipes** with EN1276 / EN14476 / "antibacterial" / "virucidal" claims?

- If YES: GB BPR compliance review is URGENT — verify each product has GB authorisation + active substance is HSE-approved + supplier on GB Article 95 list. Unauthorised placement is a criminal offence.
- If NO: refrain from carrying these products until BPR-compliance audit complete

**Q12**: Where does Sorellon source PPE, gloves, syringes, dressings — UK, EU, China, or mixed?

- UK only: MHRA / REACH burden is light (distributor obligations only)
- EU with appointed UK Only Representative (OR): also relatively light
- EU without OR, or China direct: Sorellon is the importer for UK REACH and the medical-device-importer for MHRA — significantly heavier obligations
- The split per product matters; do a per-product audit

**Q13**: Has Sorellon ever "white-labelled" or re-branded any product (replaced the manufacturer's label with Sorellon's label)?

- If YES: Sorellon assumes manufacturer-level liability under UKCA/CE, REACH/CLP, MHRA, BPR — much heavier regulatory exposure
- If NO: distributor-level only

### 17.6. Insurance specifics (more questions once agent 2 returns)

**Q14**: What is Sorellon's projected **first-year turnover**? (Range OK)

- Drives insurance underwriting tier
- Pre-revenue with "expected £0-£50k year-1" vs "expected £50k-£200k year-1" vs "expected £200k+" all get different quotes

**Q15**: Has Zac or Dieter EVER had a business insurance claim, county court judgment (CCJ), or fraud-related disclosure?

- Required for honest insurance underwriting
- Not asking about personal life — only items material to business / director-fiduciary duty

### 17.7. Certifications choice

**Q16**: Is Sorellon willing to commit to a Living Wage minimum hourly rate for any future contracted-in cleaners or part-time staff?

- Required for Living Wage Foundation accreditation
- Practical implication: must use Living Wage cleaners (not the cheapest cleaning company) if Sorellon ever has cleaners coming to its premises

**Q17**: Is Sorellon willing to commit to **NO third-country sourcing for products with documented forced-labour risk** (e.g. Xinjiang region cotton)?

- Some bids ask for written commitments on this — fast-becoming a procurement-rejection criterion
- Affects PPE (cotton-based wipes), some workwear

### 17.8. Procurement Act 2023 transition specifics

**Q18**: Has Sorellon already started its Find a Tender (CDP) registration journey?

- The platform requires personal GOV.UK One Login PER PERSON (not per company); only company admin can finalise
- Decide: Dieter as admin (operator-friendly) or Zac (director sign-off oriented)?

### 17.9. Operational decisions

**Q19**: Sorellon's registered office (Avantgarde Tower, Apartment 144) is a residential apartment. Is this a long-term arrangement, or will Sorellon move to a commercial address?

- Residential reg office triggers HMRC anti-fraud flags, insurance underwriting questions, and some procurement-portal "premises audit" failures (Constructionline Platinum, Achilles Verify)
- If staying: prepare a brief explainer ("UK SME, no physical retail/wholesale operations, deliveries direct-to-customer from supplier") for bid forms

**Q20**: Does Sorellon hold any current commercial relationships (signed supplier agreements / MOUs / letters of intent) with manufacturers in PPE / cleaning / hygiene / healthcare consumables?

- Required for VAT registration "intent to trade" evidence
- Required for NHS Supply Chain / framework applications ("supplier of record" model)
- Required for credit-rating uplift over time

### 17.10. Documentation reality check

**Q21**: For each of the 10 already-held policies (Anti-Bribery through Whistleblowing) — when was the LAST time each was actually reviewed and re-dated? (sorellon.md says March 2026 — confirm this matches the actual PDF metadata)

**Q22**: Is the Carbon Reduction Plan complete enough to be cited in bids today, or are the baseline numbers still TBC?

- If still TBC: prioritise baseline calculation in week 11 of 90-day plan
- If complete: ready to cite immediately

### 17.11. Spelling and identity hygiene

**Q23**: Confirm the canonical company brand spelling is **double-L** (`Sorellon`, `sorellon.com`, `sales@sorellon.com`) for ALL future documents, registrations, and bid responses. The L6 docs throughout use single-L (`sorelon` / `sorelon.com` / `sales@sorelon.com`) — this is a known typo and the operator-confirmed canonical spelling per the smoke-test session is double-L.

- Action: every new artifact (insurance, certifications, procurement portal registrations) MUST use the double-L spelling
- L6 docs cleanup is a deferred bead (separate work)

### Priority for answering

Suggested answer order:
- 🔴 **MUST answer to proceed**: Q9 (bank), Q11 (BPR exposure), Q12 (sourcing), Q14 (turnover)
- 🟡 **Should answer within 2 weeks**: Q1-Q4 (strategic scope), Q5-Q6 (hiring), Q7-Q8 (capital), Q10 (accountant), Q15 (claim history), Q19 (registered office), Q20 (supplier relationships)
- 🟢 **Can answer as items come up**: Q13 (white-labelling), Q16-Q17 (commitments), Q18 (admin choice), Q21-Q22 (policy status), Q23 (spelling confirmation already received)

Sorellon's bid-readiness 90-day plan EXECUTES IN PARALLEL with answering these — but red-tier questions should land within week 1 to avoid rework.

---

## 18. Appendix — External Sources Cited

All inline `[Source: <URL>]` citations consolidated. Operator should re-verify any time-sensitive claim (pricing, regulatory commencement) against the live source before action — research date is 2026-05-25; gov.uk and regulator pages update frequently.

### Primary regulator + government sources (deduplicated from inline citations across §10-§16)

- [Source: <URL>]
- [Source: https://accendo.org.uk/9001-and-iso-14001-costs-2/]
- [Source: https://accfirm.co.uk/public-liability-insurance-uk/]
- [Source: https://acuitylaw.com/rise-in-ico-fees-data-protection-fees-february/]
- [Source: https://amtivo.com/uk/standards/iso-14001/insights/how-much-does-certification-cost/]
- [Source: https://amtivo.com/uk/ukas-accredited/]
- [Source: https://assets.crowncommercial.gov.uk/wp-content/uploads/RM6240-Framework-Schedule-1-SpecIfication.pdf]
- [Source: https://assets.publishing.service.gov.uk/media/67af78c8a75f02dffca29bd8/PPN_014_Cyber_essentials_scheme.pdf]
- [Source: https://assets.publishing.service.gov.uk/media/68873308cec9ccd515ae09b0/Transparency_in_supply_chains_a_practical_guide.pdf]
- [Source: https://bcorporation.uk/b-corp-certification/the-certification-process/pricing/]
- [Source: https://brightercompliance.co.uk/ukas-vs-non-ukas-iso-certification/]
- [Source: https://casusconsulting.com/services/uk/device-registration/]
- [Source: https://cefasttrack.com/blog/cyber-essentials-cost-2026]
- [Source: https://clearborder.co.uk/resource/hmrc-importer-requirements/]
- [Source: https://cloud.google.com/terms/data-processing-addendum]
- [Source: https://cms.law/en/gbr/legal-updates/section-54-modern-slavery-act-where-do-we-go-from-here]
- [Source: https://companieshouse.blog.gov.uk/2026/01/16/understanding-identity-verification-for-people-with-significant-control-pscs/]
- [Source: https://compareyourbusinesscosts.co.uk/combined-business-insurance]
- [Source: https://compareyourbusinesscosts.co.uk/iso-certification-uk]
- [Source: https://connection-technologies.co.uk/blog/cyber-essentials-cost-uk-2026]
- [Source: https://connection-technologies.co.uk/blog/cyber-essentials-renewal-uk-2026]
- [Source: https://connection-technologies.co.uk/blog/cyber-insurance-uk-2026]
- [Source: https://ecovadis.com/]
- [Source: https://england.nhs.uk/nhs-commercial/central-commercial-function-ccf/evergreen/]
- [Source: https://gdpr-info.eu/art-27-gdpr/]
- [Source: https://gosuperscript.com/business-insurance/business-type/startup-insurance/]
- [Source: https://hellios.com/hubfs/JOSCAR%20Supplier%20Compliance%20Management_An%20ASC%20Suppliers%20Guide.pdf?hsLang=en]
- [Source: https://hellios.com/joscar]
- [Source: https://help.simplybusiness.co.uk/hc/en-gb/articles/12666795779357-Give-us-a-call]
- [Source: https://hertsprocurement.nhs.uk/joomla/index.php?option=com_content&view=article&id=5&Itemid=10]
- [Source: https://hightable.io/iso-27001-certification-cost/]
- [Source: https://iasme.co.uk/cyber-essentials/]
- [Source: https://ico.org.uk/about-the-ico/our-information/penalty-notices-issued-for-non-payment-of-data-protection-fee/]
- [Source: https://ico.org.uk/for-organisations/data-protection-fee/data-protection-fee-self-assessment/shared/results/result-companies-house-not-trading/]
- [Source: https://ico.org.uk/for-organisations/data-protection-fee/data-protection-fee/]
- [Source: https://ico.org.uk/for-organisations/data-protection-fee/data-protection-fee/penalties/]
- [Source: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/accountability-and-governance/contracts-and-liabilities-between-controllers-and-processors-multi/what-needs-to-be-included-in-the-contract/]
- [Source: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/accountability-and-governance/documentation/what-do-we-need-to-document-under-article-30-of-the-gdpr/]
- [Source: https://ico.org.uk/for-organisations/uk-gdpr-guidance-and-resources/accountability-and-governance/documentation/who-needs-to-document-their-processing-activities/]
- [Source: https://iso27001cost.com/cost-uk]
- [Source: https://justquoteme.co.uk/cyber-liability-insurance-the-complete-2026-guide-for-uk-businesses/]
- [Source: https://landmarkglobal.com/eu/en/news-insights/ukca-vs-ce-marking-in-2026/]
- [Source: https://ltd-companies.co.uk/resources/register-for-paye-uk/]
- [Source: https://mbridgetechnologies.cloud/blog/vat-threshold-increase-april-2026]
- [Source: https://mtd.digital/mtd-income-tax/mtd-vat-complete-guide/]
- [Source: https://netsecgroup.io/guides/ppn-014-government-contracts-cyber-essentials]
- [Source: https://psip.co.uk/blog/ccs-rm6378-facilities-management-framework]
- [Source: https://psip.co.uk/blog/uk-public-sector-procurement-portals]
- [Source: https://supplierregistration.cabinetoffice.gov.uk/]
- [Source: https://uk.markel.com/insurance]
- [Source: https://www.achilles.com/features/uvdb-membership-pricing-and-features/]
- [Source: https://www.achilles.com/uvdb/]
- [Source: https://www.allianz-trade.com/en_GB/insights/grow-your-business/what-is-a-b-corp-and-why-should-my-sme-become-one.html]
- [Source: https://www.axa.co.uk/business-insurance/]
- [Source: https://www.axa.co.uk/business-insurance/public-liability/product-liability-insurance/]
- [Source: https://www.axaconnect.co.uk/commercial-lines/branch-traded/business-combined/]
- [Source: https://www.beaconrisk.co.uk/news/123-is-constructionline-expensive-constructionline-fees-explained]
- [Source: https://www.beaconrisk.co.uk/news/150-how-much-does-safecontractor-cost-and-is-it-worth-it]
- [Source: https://www.bevanbrittan.com/insights/articles/2024/the-procurement-act-2023-now-due-to-take-effect-24-february-2025/]
- [Source: https://www.biba.org.uk/find-insurance/broker-directory/towergate-insurance-london/]
- [Source: https://www.chas.co.uk/blog/public-sector-procurement-now-specifies-the-common-assessment-standard/]
- [Source: https://www.clacks.gov.uk/document/7034.doc]
- [Source: https://www.compliancegate.com/united-kingdom-product-liability-insurance/]
- [Source: https://www.complys.co.uk/blog/chas-vs-safecontractor-vs-smas]
- [Source: https://www.constructionline.co.uk/products-services/memberships/sub-contractor-memberships/]
- [Source: https://www.constructionline.co.uk/wp-content/uploads/2026/02/CL-Gold-Platinum-Supplier-Checklist-Guide-February-2026.pdf]
- [Source: https://www.crowncommercial.gov.uk/agreements/RM6240]
- [Source: https://www.crowncommercial.gov.uk/agreements/RM6331]
- [Source: https://www.doubleyoursustainability.com/blog/true-cost-of-ecovadis]
- [Source: https://www.everywhen.co.uk/locations/london]
- [Source: https://www.fca.org.uk/firms/professional-indemnity-insurance/distributors]
- [Source: https://www.find-tender.service.gov.uk/Search]
- [Source: https://www.gca.gov.uk/]
- [Source: https://www.gov.uk/contracts-finder]
- [Source: https://www.gov.uk/corporation-tax]
- [Source: https://www.gov.uk/government/news/companies-house-fees-are-changing-from-1-february-2026]
- [Source: https://www.gov.uk/government/news/mhra-invites-views-on-proposed-changes-to-medical-device-regulation]
- [Source: https://www.gov.uk/government/publications/disability-confident-guidance-for-levels-1-2-and-3]
- [Source: https://www.gov.uk/government/publications/general-product-safety-regulations-2005/general-product-safety-regulations-2005-great-britain]
- [Source: https://www.gov.uk/government/publications/personal-protective-equipment-enforcement-regulations-2018/regulation-2016425-and-the-personal-protective-equipment-enforcement-regulations-2018-great-britain]
- [Source: https://www.gov.uk/government/publications/ppn-009-tackling-modern-slavery-in-government-supply-chains/ppn-009-guidance-on-tackling-modern-slavery-in-government-supply-chains-html]
- [Source: https://www.gov.uk/government/publications/ppn-014-cyber-essentials-scheme/ppn-014-cyber-essentials-scheme-html]
- [Source: https://www.gov.uk/government/publications/procurement-act-2023-guidance-documents-define-phase/guidance-below-threshold-contracts-html]
- [Source: https://www.gov.uk/government/publications/procurement-act-2023-short-guides/suppliers-how-to-register-your-organisation-and-first-administrator-on-find-a-tender-in-three-easy-steps-html]
- [Source: https://www.gov.uk/government/publications/procurement-act-2023-short-guides/the-procurement-act-2023-a-short-guide-for-suppliers-html]
- [Source: https://www.gov.uk/guidance/add-your-modern-slavery-statement-to-the-statement-registry]
- [Source: https://www.gov.uk/guidance/confirmation-statement-guidance]
- [Source: https://www.gov.uk/guidance/publish-an-annual-modern-slavery-statement]
- [Source: https://www.gov.uk/guidance/verify-your-identity-for-companies-house]
- [Source: https://www.gov.uk/register-employer]
- [Source: https://www.gov.uk/register-for-vat]
- [Source: https://www.hellopolaris.co.uk/blog/what-happens-when-declared-turnover-no-longer-matches-reality/]
- [Source: https://www.hiscox.co.uk/broker/contact]
- [Source: https://www.hiscox.co.uk/business-blog/employers-liability-act]
- [Source: https://www.hiscox.co.uk/business-insurance/product-liability-insurance]
- [Source: https://www.hiscox.co.uk/business-insurance/public-liability-insurance]
- [Source: https://www.hiscox.co.uk/business-insurance/start-up-businesses]
- [Source: https://www.hoganlovells.com/en/publications/be-vigilant-new-postmarket-surveillance-requirements-for-medical-devices-in-great-britain]
- [Source: https://www.hoganlovells.com/en/publications/uk-procurement-law-further-procurement-act-2023-provisions-in-force-1-january-2026-and-1-april-2026]
- [Source: https://www.hse.gov.uk/biocides/about/regulations-decisions.htm]
- [Source: https://www.hse.gov.uk/biocides/uk-list-active-substances.htm]
- [Source: https://www.hse.gov.uk/chemical-classification/legal/clp-regulation.htm]
- [Source: https://www.hse.gov.uk/pubns/hse40.htm]
- [Source: https://www.hse.gov.uk/reach/]
- [Source: https://www.hse.gov.uk/reach/duin.htm]
- [Source: https://www.identitysolutions.ncino.com/blog/costs-for-identity-verification-of-shareholders]
- [Source: https://www.insure24.co.uk/blog/insurance-needed-for-government-and-public-sector-contracts/]
- [Source: https://www.insure24.co.uk/blog/product-liability-insurance-for-chemical-products-uk-a-practical-guide/]
- [Source: https://www.insure24.co.uk/blog/product-recall-insurance-for-chemical-manufacturers-explained/]
- [Source: https://www.intelance.co.uk/cyber-essentials-plus-certification-uk/]
- [Source: https://www.isms.online/cyber-essentials/cost/]
- [Source: https://www.isms.online/cyber-essentials/plus-requirements/]
- [Source: https://www.jmginsurance.co.uk/blog/how-much-does-cyber-liability-insurance-cost-in-the-uk/]
- [Source: https://www.kennedyslaw.com/en/thought-leadership/article/2025/mhra-to-consult-on-indefinite-recognition-of-ce-marked-medical-devices-a-major-step-for-industry-and-patients-in-great-britain/]
- [Source: https://www.kingsbridge.co.uk/blog/contractors/contractor-guides/employers-liability-requirements-and-exemptions/]
- [Source: https://www.legislation.gov.uk/ukpga/1969/57]
- [Source: https://www.legislation.gov.uk/ukpga/2015/30/section/54]
- [Source: https://www.legislation.gov.uk/ukpga/2023/54/contents]
- [Source: https://www.livingstonesinsurance.co.uk/blog/public-liability-insurance-limits-uk-how-much-do-you-need/]
- [Source: https://www.livingwage.org.uk/accreditation]
- [Source: https://www.livingwage.org.uk/faqs]
- [Source: https://www.localbusinessmagazine.co.uk/companies-house-identity-verification-rollout/]
- [Source: https://www.lw.com/en/insights/uk-government-issues-updated-modern-slavery-act-transparency-guidance-key-takeaways-for-businesses]
- [Source: https://www.markeluk.com/business-insurance/public-liability-insurance]
- [Source: https://www.ncsc.gov.uk/cyberessentials/overview]
- [Source: https://www.nfp.co.uk/commercial-insurance/by-sector/wholesale-insurance/]
- [Source: https://www.osborneclarke.com/insights/uk-government-overhaul-product-safety-regulatory-framework]
- [Source: https://www.policybee.co.uk/blog/does-cyber-insurance-cover-gdpr]
- [Source: https://www.premierline.co.uk/about-us/contact-us/]
- [Source: https://www.premierline.co.uk/about-us/insurers/hiscox-insurance/]
- [Source: https://www.qinetiq.com/-/media/dfadf4f2226943008cf047d61841088a.ashx]
- [Source: https://www.qualitycompanyformations.co.uk/blog/registering-as-an-employer/]
- [Source: https://www.regdesk.co/blog/mhra-guidance-on-md-regulations/]
- [Source: https://www.riskbriefing.co.uk/businesses-facing-unaccredited-iso-certificates-being-rejected/]
- [Source: https://www.rkmsuk.co.uk/blog/non-iaf-iso-certificates-uk/]
- [Source: https://www.rkmsuk.co.uk/iso-consultancy/how-much-does-iso-9001-cost/]
- [Source: https://www.rkmsuk.co.uk/iso-consultancy/the-difference-between-ukas-and-non-ukas-certification/]
- [Source: https://www.rtcoopers.com/practice_biocides.php]
- [Source: https://www.sccl.nhs.uk/wp-content/uploads/2023/10/Appendix-4-NHS-Supply-Chain-Framework-Agreement-for-the-Supply-of-Goods.pdf]
- [Source: https://www.sccl.nhs.uk/wp-content/uploads/2024/10/VWS_Framework-Agreement.docx_Redacted.pdf]
- [Source: https://www.scilife.io/blog/ukca-mark]
- [Source: https://www.sell2wales.gov.wales/]
- [Source: https://www.simplybusiness.co.uk/business-insurance/limited-company-insurance/]
- [Source: https://www.simplybusiness.co.uk/business-insurance/product-liability-insurance/]
- [Source: https://www.simplybusiness.co.uk/business-insurance/professional-indemnity-insurance/]
- [Source: https://www.simplybusiness.co.uk/business-insurance/public-liability-insurance/]
- [Source: https://www.simplybusiness.co.uk/business-insurance/wholesale-insurance/]
- [Source: https://www.simplybusiness.co.uk/legal/important-information/]
- [Source: https://www.smartqhse.com/safety-blog/iso-45001-certification-cost-guide]
- [Source: https://www.starlingbank.com/resources/business-guides/benefits-of-being-vat-registered/]
- [Source: https://www.supplychain.nhs.uk/categories/]
- [Source: https://www.supplychain.nhs.uk/product-information/contract-launch-brief/cleaning-supplies-equipment-and-associated-products/]
- [Source: https://www.supplychain.nhs.uk/product-information/contract-launch-brief/environmental-decontamination/]
- [Source: https://www.supplychain.nhs.uk/product-information/contract-launch-brief/hand-hygiene-and-associated-products-and-services/]
- [Source: https://www.supplychain.nhs.uk/product-information/contract-launch-brief/single-use-personal-protective-equipment-and-medical-consumables/]
- [Source: https://www.supplychain.nhs.uk/suppliers/master-indemnity-agreement-approved-suppliers/]
- [Source: https://www.supplychain.nhs.uk/suppliers/portal/]
- [Source: https://www.taxually.com/blog/is-voluntary-vat-registration-right-for-your-business]
- [Source: https://www.tendersgo.com/post/united-kingdom-procurement-act-2023-new-rules-thresholds-cdp-registrat-6317]
- [Source: https://www.thp.co.uk/uk-vat-threshold-2026-a-complete-guide-for-growing-businesses/]
- [Source: https://www.tradedirectinsurance.co.uk/insurance/public-liability/faq/is-it-law-to-have-public-liability-insurance/]
- [Source: https://www.tussell.com/insights/what-is-supplier-debarment-procurement-act-2023]
- [Source: https://www.xero.com/uk/programme/making-tax-digital/]
- [Source: https://www.yourcompanyformations.co.uk/blog/company-utr-number-explained/]
- [Source: https://youriso.co.uk/cost-of-iso-9001/]
- [Source: https://youriso.co.uk/iso-45001-certification-cost/]

### Internal references

- `sorellon.md` (project root) — operator-maintained company profile source of truth
- `docs/research/2026-05-25-sorellon-bidding-readiness-research.md` — original comprehensive research doc (this FINAL doc supersedes)
- `docs/research/2026-05-25-sorellon-mvs-tailored-workflow.md` — tailored workflow companion (this FINAL doc supersedes)
- `docs/handoff-2026-05-25-l6-p7-live-demo.md` — L6 Phase 7 live-demo handoff for fresh agent session
- `docs/pipeline/l6-request-pipeline-automation/live-demo/2026-05-25-smoke-test.md` — Session 1 smoke-test diary
- `packages/bidding/company-profile-schema.ts` — bid readiness gate TypeScript schema
- `packages/intelligence/src/logistics/freight-engine.ts:52` — FreightIncoterm modelling
- `skills/landed-cost/freightos-freight-estimate/SKILL.md` — Freightos skill specification
- `docs/strategy/layers/layer-0-gold-set-evaluation.md:96-122` — pipeline category whitelist + no-list
- `docs/strategy/layers/layer-5-expanded-commercial-intelligence.md` — commercial intelligence + margin gates
- `docs/strategy/layers/layer-6-request-pipeline-automation.md` — bid workflow + readiness gates

---

**End of master document.** This is the single source of truth for Sorellon's UK public procurement bidding readiness. Re-read §1 weekly during the first 8 weeks; consult Part II sections when you need detail on a specific item.
