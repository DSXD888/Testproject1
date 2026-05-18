---
skill_id: procurement-ingestion/procontract-register-interest
skill_version: 1.1.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: external_mutation
source_structure_version: procontract-interest-2026-04-13
requires_cdp: true
cdp_default_port: 9222
maxPages: 5
maxScrollCycles: 1
---

# procurement-ingestion/procontract-register-interest

## Use when
Use this skill when the daemon has already pre-filtered ProContract opportunities and needs the browser agent to register interest on the qualifying ones.

## Do not use when
Do not use this skill to discover new opportunities or to extract RFx document packs.

## Inputs
- `source_id`: should be `procontract-interest`
- `contracts_to_register`: array of qualifying contracts the daemon already approved
- `pre_filter_evaluated`: optional full evaluation list including skipped records for reporting

## Outputs
- `result.json` containing a `registrations[]` array and a `summary` object
- One result item per evaluated contract when `pre_filter_evaluated` is supplied

## Side Effects

This skill performs a real external mutation by clicking `Register interest in this opportunity` on a live procurement portal. That action is authorized.

## Failure Codes
- AGENT_AUTH_CHALLENGE
- PAGE_SHAPE_DRIFT
- SOURCE_SOFT_BLOCKED
- AGENT_PARTIAL_FAILURE
- AGENT_STALLED

---

## Playbook

You are a procurement portal action agent. The daemon has already decided which contracts are safe to register. Your job is to carry out the portal action and report what happened.

### Mandatory CDP Discipline

This skill must attach to the operator's already-authenticated persistent browser. Do not launch a fresh browser profile, do not attempt portal login, do not solve captchas, do not log out, and do not close the operator browser.

Before any portal navigation:

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, write a valid blocked `result.json` immediately:

```json
{
  "cursor": "{}",
  "continuation_required": false,
  "registrations": [],
  "summary": {
    "total_evaluated": 0,
    "registered": 0,
    "already_registered": 0,
    "skipped_service": 0,
    "skipped_expiry": 0,
    "failed": 0
  },
  "status": "blocked",
  "blockers": [
    {
      "code": "CDP_CONNECT_FAILED",
      "message": "ProContract register-interest requires the operator persistent CDP browser on localhost:9222. Start Chrome/Chromium with remote debugging, sign in manually, then rerun.",
      "severity": "blocked"
    }
  ]
}
```

Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

If the portal shows `Log In`, `Session expired`, captcha, or another auth challenge, emit a valid blocked result with blocker code `PROCONTRACT_SESSION_EXPIRED` or `AGENT_AUTH_CHALLENGE`. The operator must restore the session manually in the persistent CDP browser.

Before any external mutation, confirm all preconditions:

- CDP preflight succeeded.
- The current page is authenticated and not an auth challenge.
- The target contract came from `contracts_to_register`.
- The advert has been checked for an already-registered state before clicking.

### Step 1: Read the daemon input

Read:

- `contracts_to_register`: the contracts you must action
- `pre_filter_evaluated`: optional evaluation report for all checked contracts

If `pre_filter_evaluated` is present:

- Any item already marked `action: "skipped"` should be copied through to the final `registrations[]` unchanged.
- Only perform portal actions for the items in `contracts_to_register`.

If `contracts_to_register` is empty:

- Write a valid empty result with copied skipped items, if any.
- Do not attempt portal navigation beyond optional authenticated-session verification.

### Step 2: Open the authorized advert in the persistent browser

For each contract URL:

```bash
agent-browser --cdp ${CDP_PORT} open "{source_url}"
agent-browser --cdp ${CDP_PORT} wait 2000
```

If the page is `Log In`, `Session expired`, captcha, or any auth challenge, stop and write the blocked result described in Mandatory CDP Discipline. Do not click any mutation control when auth state is uncertain.

### Step 3: Detect already-registered state before clicking

The live advert page exposes the already-registered state clearly:

- A section heading `Expression of interest registered`
- Fields such as `Date` and `Workgroup`

If that section already exists before you click:

- Do not click the interest link again
- Emit:
  - `action: "already_registered"`
  - `registered_at`: if visible on the page

### Step 4: Perform the real registration

When the advert still shows `Register interest in this opportunity` and no prior registration section:

```bash
agent-browser --cdp ${CDP_PORT} click "Register interest in this opportunity"
agent-browser --cdp ${CDP_PORT} wait 3000
agent-browser --cdp ${CDP_PORT} snapshot -i
```

Success is indicated by a modal/dialog containing:

- `Expression of interest successful`
- `What happens next?`
- A link to the `activities centre`

Also verify that the page now contains `Expression of interest registered`.

Emit:

- `action: "registered"`
- `registered_at`: current observed UTC time if the page does not show a timestamp yet

### Step 5: Handle failures

If registration cannot be completed, emit:

- `action: "failed"`
- `failure_reason`: short machine-readable reason such as:
  - `button_not_found`
  - `session_expired`
  - `confirmation_missing`
  - `portal_error`

### Step 6: Build the result contract

Write `result.json` with this top-level shape:

```json
{
  "cursor": "{\"last_processed_at\":\"2026-04-13T15:00:00Z\"}",
  "continuation_required": false,
  "registrations": [],
  "summary": {
    "total_evaluated": 0,
    "registered": 0,
    "already_registered": 0,
    "skipped_service": 0,
    "skipped_expiry": 0,
    "failed": 0
  }
}
```

Per-item fields:

```text
notice_id:             required
process_id:            required if known from input
source_url:            required for acted items
action:                one of registered | already_registered | skipped | failed
skip_reason:           required when action == skipped
failure_reason:        required when action == failed
pre_filter_result:     copy from daemon input when present
registered_at:         ISO 8601 timestamp when action is registered/already_registered
```

### Step 7: Leave the persistent browser open

Do not close or reset the operator browser. Leave the last navigated ProContract page available for operator inspection.

---

## Canary

Verify that a qualifying ProContract advert can be opened through an already-authenticated CDP session, that the `Register interest in this opportunity` control is discoverable, and that the already-registered state can be distinguished from the pre-registration state.

## Run Manifest Instructions

Produce a run manifest with:

- manifest_version
- skill_id
- skill_version
- job_id
- session_id
- execution_mode
- started_at
- completed_at
- outcome_class
- items_processed
- items_skipped
- items_failed
- warnings_count
- agent_claimed_pages_visited
- agent_claimed_source_url
- agent_claimed_fingerprint_state
- agent_claimed_locale_state
- agent_claimed_currency_state

Narrative sections:

- Summary
- Steps Taken
- Decisions Made
- Warnings
- Evidence
