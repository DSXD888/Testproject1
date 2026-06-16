---
skill_id: source-health/alibaba
skill_version: 2026.03.30-1
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
validator_overlay_version: 2026.03.30-1
source_structure_version: alibaba-2026-03-30
model_profile: browse_stable_low_cost
agent_cli: claude
side_effect_class: none
promotion_stage: stable
worker_behavior_version: source-health/alibaba@2026.03.30-1+1.0.0
validator_behavior_version: 2026.03.30-1
version_change_affects_behavior: true
gate_a_family_key: source-health
gate_a_family_prequalified: false
gate_b_exact_slice_key: source-health/alibaba::alibaba::source-health/alibaba@2026.03.30-1+1.0.0::2026.03.30-1
gate_b_exact_slice_ready: false
requires_cdp: true
cdp_default_port: 9222
maxPages: 1
maxScrollCycles: 0
---

# source-health/alibaba

## Use when
Use this skill when the daemon needs a lightweight Alibaba source-health canary that proves reachability, authentication, locale/currency correctness, and a minimal search result before expensive supplier enrichment runs proceed.

## Do not use when
Do not use this skill for full supplier extraction, auth refresh, pricing capture, relation resolution, manifest generation, or any workflow that persists authoritative listing evidence.

## Inputs
- A source identifier of `alibaba`.
- An auth-state handle when available.
- Expected locale and currency expectations for the marketplace.
- A known-good minimal search query.
- A small time budget appropriate for a canary.

## Outputs
- A six-check source-health result (`reachability`, `fingerprint`, `auth_state`, `locale`, `challenge_rate`, `minimal_extraction`).
- A summarized health outcome (`green`, `amber`, `red`) with failure classification when relevant.
- Minimal sample metadata proving the known-good search still yields a usable listing.

## Non-goals
- Full supplier enrichment or listing evidence persistence.
- Session establishment or login repair.
- Cross-source cache sharing or relation decisions.

## Failure Codes
- AGENT_AUTH_CHALLENGE
- SOURCE_CAPTCHA_BLOCKED
- SOURCE_SOFT_BLOCKED
- PAGE_SHAPE_DRIFT
- LOCALE_OR_CURRENCY_DRIFT
- AGENT_SKILL_FAILED
- AGENT_STALLED
- AGENT_RUN_FAILED

## Resume Strategy
Resume is a fresh canary run. If the last canary failed, restart from the Alibaba landing/search page with the current auth-state handle, compare the observed fingerprint to the last passing baseline, and do not continue from a partially blocked page state.

## Pagination Bounds
- `maxPages`: 1
- `maxScrollCycles`: 0

## Canary
Verify Alibaba reachability, authenticated state, expected locale/currency, absence of challenge markers, and the presence of at least one usable result from a known-good search without advancing into full extraction.

## Playbook

### Mandatory CDP Discipline

This canary must attach to the operator's already-authenticated persistent Alibaba browser session. Do not launch a fresh browser profile, do not attempt login, do not solve captchas, do not log out, and do not close the operator browser.

Before navigation:

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"

# Ensure CDP is up. The daemon's pre-flight gate SHOULD have done this already,
# but defense in depth: bin/ensure-cdp-9222 is idempotent and fast (<1s no-op
# when CDP is already up; otherwise launches Chrome with the operator's
# persistent profile at ~/.chrome-automation and polls up to
# 15s). Returns non-zero only if launch genuinely failed. Absolute path because
# the skill subprocess cwd is the per-skill workdir, not the project root.
~/Desktop/testproject1/bin/ensure-cdp-9222

# Defensive reachability check. If this fails (helper exited non-zero, or a
# rare post-launch race), follow the "If CDP is unreachable" guidance below:
# write the skill-specific blocked result.json and stop.
curl -sfS -m 3 "http://localhost:${CDP_PORT}/json/version"
```

If CDP is unreachable, write a valid source-health result with `health_state: "red"`, `checks.reachability: "failed"`, `checks.auth_state: "blocked"`, and `failure_classification: "CDP_CONNECT_FAILED"`. Every browser command must use:

```bash
agent-browser --cdp ${CDP_PORT} ...
```

### Session Recovery (auto-signin on session-expired)

If the health check detects that the operator's persistent Chrome profile is signed OUT of Alibaba, attempt auto-signin via `auth-signin/alibaba/` BEFORE writing a red health verdict. Source-health's purpose is to report whether the source is fundamentally healthy; transient session expiry that auto-signin can fix is NOT a red verdict — it's a recoverable warning.

This is a change from the previous behaviour where any auth challenge resulted in `failure_classification: "AGENT_AUTH_CHALLENGE"` and a red verdict. Now, auth challenges first attempt recovery; only unrecoverable challenges (captcha, 2FA, binding flow) result in red.

**Session-expired detection signals (Alibaba, shared with marketplace-search/alibaba):**
- URL redirects to `passport.alibaba.com/login.htm` or `login.alibaba.com` after opening `https://www.alibaba.com/`.
- The header shows `Sign In` link (vs. account menu when signed in).
- The page-level auth probe (e.g., checking for `__USER_ID__` in window globals, or account menu presence) reports unauthenticated state.

**Recovery flow:**

The canonical click sequence below was inlined from `skills/auth-signin/alibaba/SKILL.md` §Workflow on 2026-05-17. Selectors validated live against the persistent Chrome on port 9222 the same day. If you need to update the sequence, edit the source-of-truth at `skills/auth-signin/alibaba/SKILL.md` first and re-inline here. This block is identical to the inlined recovery in `skills/marketplace-search/alibaba/SKILL.md` — both skills share the same Alibaba session and selectors.

```bash
CDP_PORT="${AGENT_BROWSER_CDP_PORT:-9222}"

# 1. Open Alibaba login page directly
agent-browser --cdp ${CDP_PORT} open "https://login.alibaba.com/"
agent-browser --cdp ${CDP_PORT} wait 2500

# 2. Check for slide-CAPTCHA. If present, map to failure_classification:
#    "SOURCE_CAPTCHA_BLOCKED" (red verdict) and stop — do not attempt OAuth
#    (Alibaba rate-limits aggressive signin retries via slide CAPTCHA).
SLIDE_CAPTCHA=$(agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const sliderIframe = document.querySelector("iframe[src*=nocaptcha], iframe[src*=sufei]");
  const sliderDom = document.querySelector("[id*=nc_], .nc-container, [class*=slider-track]");
  return JSON.stringify({slide_captcha: !!(sliderIframe || sliderDom)});
})()
')

# 3. Click "Continue with Google" — Alibaba uses <a attr-type="google" attr-customurl="...">
#    The JS click handler reads attr-customurl, so synthetic .click() may not navigate;
#    read attr-customurl directly and navigate via window.location.assign.
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const stable = document.querySelector("a[attr-type=google][attr-customurl]");
  if (stable) {
    const target = stable.getAttribute("attr-customurl");
    if (target) { window.location.assign(target); return JSON.stringify({clicked: true, via: "attr-customurl"}); }
  }
  const candidates = [
    document.querySelector("a[href*=\"accounts.google.com/o/oauth2\"]"),
    document.querySelector("button.google-signin-button, [data-spm*=google i]"),
    ...Array.from(document.querySelectorAll("button, a, [role=button]")).filter(el => /continue with google|sign in with google/i.test(el.innerText || ""))
  ].filter(Boolean);
  const btn = candidates[0];
  if (!btn) return JSON.stringify({clicked: false, reason: "no_google_button_found"});
  btn.click();
  return JSON.stringify({clicked: true, via: "fallback"});
})()
'
agent-browser --cdp ${CDP_PORT} wait 4000

# 4. Confirm we landed on Google's chooser. If not, map to SESSION_REFRESH_FAILED.
agent-browser --cdp ${CDP_PORT} eval 'JSON.stringify({on_google: window.location.hostname.endsWith("accounts.google.com"), url: window.location.href})'

# 5. Click the Dieter row on Google's chooser. (Mirror of auth-signin/google-account-chooser.)
agent-browser --cdp ${CDP_PORT} eval '
(() => {
  const target = "dieter.x.s.danicek@gmail.com";
  if (!window.location.hostname.endsWith("accounts.google.com")) return JSON.stringify({page_shape: "not_on_google"});
  if (document.querySelector("iframe[src*=recaptcha]")) return JSON.stringify({page_shape: "captcha"});
  const bodyText = (document.body && document.body.innerText) || "";
  if (/Verify it.s you|2-Step Verification/.test(bodyText)) return JSON.stringify({page_shape: "twofa_interstitial"});
  if (document.querySelector("input[type=email][name=identifier], input#identifierId")) return JSON.stringify({page_shape: "email_entry_form"});
  const row = document.querySelector(`[data-identifier="${target}"]`)
    || document.querySelector(`[data-email="${target}"]`)
    || Array.from(document.querySelectorAll("li[role=link], div[role=link], div[role=button]"))
        .find(el => (el.innerText || "").toLowerCase().includes(target.toLowerCase()));
  if (!row) return JSON.stringify({page_shape: "chooser_target_missing"});
  row.click();
  return JSON.stringify({page_shape: "chooser_target_found", clicked: true});
})()
'
agent-browser --cdp ${CDP_PORT} wait 5000

# 6. Poll for redirect back to alibaba.com and verify signed-in. Detect each
#    escalation case (binding, region, identity verification, slide CAPTCHA).
agent-browser --cdp ${CDP_PORT} eval '
(async () => {
  const deadline = Date.now() + 18000;
  while (Date.now() < deadline) {
    const url = window.location.href;
    const hostname = window.location.hostname;
    const bodyText = (document.body && document.body.innerText) || "";
    if (/bind.*alibaba account|link your google account/i.test(bodyText)) return JSON.stringify({status: "binding_required", url});
    if (/select your country|change.*region|account region/i.test(bodyText)) return JSON.stringify({status: "region_prompt", url});
    if (/verify your identity/i.test(bodyText) && /phone number|send.*code/i.test(bodyText)) return JSON.stringify({status: "identity_verification_required", url});
    if (url.includes("/iv/verify_mode.htm")) return JSON.stringify({status: "identity_verification_required", url});
    if (document.querySelector("iframe[src*=nocaptcha], [id*=nc_], .nc-container")) return JSON.stringify({status: "slide_captcha", url});
    const onAlibaba = hostname.endsWith("alibaba.com") && !hostname.startsWith("passport.") && !hostname.startsWith("login.");
    if (onAlibaba) {
      const accountMenu = document.querySelector("[class*=account-menu], [class*=my-alibaba], [class*=user-info]");
      if (accountMenu) return JSON.stringify({status: "signed_in", url, verified_via: "account_menu_present"});
      return JSON.stringify({status: "signed_in_weak", url, verified_via: "passport_redirect_absent"});
    }
    await new Promise(r => setTimeout(r, 500));
  }
  return JSON.stringify({status: "timeout", url: window.location.href});
})()
'
```

After the recovery sequence completes, map the poll `status` back to this skill's health-result taxonomy:

1. **If `status: "signed_in"` (or `"signed_in_weak"`):** continue with the source-health checks as normal. Report `health_state: "green"` (or `"amber"` if any other health dimension is degraded). The successful auto-signin counts as a non-blocking event — record it as a warning in the health result (`auto_signin_invoked: true`) so the operator knows their session was silently refreshed.

2. **If `status: "slide_captcha"`:** write a red health result with `health_state: "red"`, `checks.auth_state: "blocked"`, `failure_classification: "SOURCE_CAPTCHA_BLOCKED"`, and a message naming the slide CAPTCHA.

3. **If `status` is `"binding_required"`, `"region_prompt"`, `"identity_verification_required"`, `"timeout"`, or the Google chooser returned `"twofa_interstitial"` / `"chooser_target_missing"` / `"captcha"`:** write a red health result with `health_state: "red"`, `checks.auth_state: "blocked"`, `failure_classification: "SESSION_REFRESH_FAILED"`, and an explanatory message naming the auto-signin failure mode (binding, region prompt, 2FA, identity verification, etc.). The operator must intervene.

Open Alibaba through CDP:

```bash
agent-browser --cdp ${CDP_PORT} open "https://www.alibaba.com/"
agent-browser --cdp ${CDP_PORT} wait 2000
agent-browser --cdp ${CDP_PORT} snapshot
```

Check whether the persistent session is authenticated, verify locale/currency expectations, and perform only the minimal known-good search needed for source health. If a sign-in page, expired-session page, or captcha appears, write a structured red health result with `failure_classification: "AGENT_AUTH_CHALLENGE"` or `"SOURCE_CAPTCHA_BLOCKED"` and stop.

## Run Manifest Instructions
Run Manifest Requirements

You must produce a run manifest that the daemon can finalize into the durable `run-manifest.md` artifact.

Required YAML frontmatter fields:
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

Required narrative sections:
- Summary
- Steps Taken
- Decisions Made
- Warnings
- Evidence

Major claims must be anchored to observable evidence such as result-contract fields, daemon appendix fields, or artifact refs. Do not rely on operator memory or hidden context.
