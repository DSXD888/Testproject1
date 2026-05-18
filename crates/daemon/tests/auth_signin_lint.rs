//! `ut_asln_001..099` — structural lint for the `auth-signin/{site}/` skill
//! family and the production skills that delegate to it.
//!
//! The auth-signin family is a NEW skill family (introduced 2026-05-17 per
//! `docs/handoff-2026-05-17-cdp-auto-signin.md`) that performs automatic
//! sign-in on session-expired detection. Each site-specific skill is short
//! and inline-composes the click sequence; the shared
//! `auth-signin/google-account-chooser/` sub-skill encapsulates the Google
//! account-picker half. The 8 production skills that drive logged-in CDP
//! sessions each contain a "Session Recovery" subsection mentioning the
//! matching `auth-signin/<site>` path.
//!
//! These lints are intentionally STRUCTURAL — they assert presence of the
//! skill files, mandatory frontmatter, the `bin/ensure-cdp-9222` invocation,
//! and the cross-reference from production skills. They do NOT exercise live
//! Chrome / Google OAuth — that's the operator's responsibility per the
//! `TEST_REPORT.md` template at `docs/testing/2026-05-17-cdp-auto-signin-test-report.md`.

#![forbid(unsafe_code)]

#[path = "support/skills.rs"]
#[allow(dead_code)]
mod skills_support;

use anyhow::{Result, ensure};

use skills_support::{load_skill_metadata, repo_skill_dir};

const AUTH_SIGNIN_SKILLS: &[&str] = &[
    "auth-signin/google-account-chooser",
    "auth-signin/ebay",
    "auth-signin/alibaba",
    "auth-signin/searates",
    "auth-signin/freightos",
    "auth-signin/procontract",
];

/// Mapping of production skills to the auth-signin skill they must delegate to.
/// Used by `ut_asln_004` to assert that the production skill body contains the
/// matching `auth-signin/<site>` substring (the Session Recovery section).
const PRODUCTION_SKILL_AUTH_SIGNIN_MAPPING: &[(&str, &str)] = &[
    ("marketplace-search/ebay-uk", "auth-signin/ebay"),
    ("marketplace-search/alibaba", "auth-signin/alibaba"),
    (
        "landed-cost/searates-freight-estimate",
        "auth-signin/searates",
    ),
    ("landed-cost/shipment-sizing", "auth-signin/alibaba"),
    ("source-health/alibaba", "auth-signin/alibaba"),
    (
        "procurement-ingestion/procontract-discover",
        "auth-signin/procontract",
    ),
    (
        "procurement-ingestion/procontract-enrich-tender",
        "auth-signin/procontract",
    ),
    (
        "procurement-ingestion/procontract-register-interest",
        "auth-signin/procontract",
    ),
];

#[test]
fn ut_asln_001_auth_signin_skills_exist_and_have_skill_md() -> Result<()> {
    for skill_id in AUTH_SIGNIN_SKILLS {
        let dir = repo_skill_dir(skill_id)?;
        let skill_md = dir.join("SKILL.md");
        ensure!(
            skill_md.exists(),
            "auth-signin skill missing SKILL.md: {skill_id} (expected at {})",
            skill_md.display()
        );
        let result_schema = dir.join("result.schema.json");
        ensure!(
            result_schema.exists(),
            "auth-signin skill missing result.schema.json: {skill_id} (expected at {})",
            result_schema.display()
        );
    }
    Ok(())
}

#[test]
fn ut_asln_002_auth_signin_skills_declare_cdp_metadata() -> Result<()> {
    // auth-signin skills are CDP-driven (they drive Chrome via the operator's
    // persistent profile). Every one must declare requires_cdp:true and the
    // canonical port 9222.
    for skill_id in AUTH_SIGNIN_SKILLS {
        let skill = load_skill_metadata(&repo_skill_dir(skill_id)?)?;
        ensure!(
            skill.metadata.requires_cdp,
            "{skill_id} must declare requires_cdp:true in its YAML frontmatter"
        );
        ensure!(
            skill.metadata.cdp_default_port == Some(9222),
            "{skill_id} must declare cdp_default_port:9222 in its YAML frontmatter (got {:?})",
            skill.metadata.cdp_default_port
        );
    }
    Ok(())
}

#[test]
fn ut_asln_003_auth_signin_skills_invoke_ensure_helper() -> Result<()> {
    // Mirrors the rule from the prior auto-launch handoff
    // (`ut_skl_302_cdp_skills_invoke_ensure_helper`): every CDP-requiring
    // skill must invoke `bin/ensure-cdp-9222` somewhere in its body so a
    // Chrome that has crashed mid-execution can be relaunched without operator
    // intervention. New auth-signin skills must satisfy the same rule.
    for skill_id in AUTH_SIGNIN_SKILLS {
        let skill = load_skill_metadata(&repo_skill_dir(skill_id)?)?;
        ensure!(
            skill.skill_markdown.contains("bin/ensure-cdp-9222"),
            "{skill_id} declares requires_cdp:true but does not invoke bin/ensure-cdp-9222 in its body"
        );
    }
    Ok(())
}

#[test]
fn ut_asln_004_production_skills_reference_matching_auth_signin() -> Result<()> {
    // Each production skill that drives a logged-in CDP session must contain
    // a Session Recovery section referencing its matching auth-signin skill.
    // The substring check is intentionally loose — it tolerates the
    // production skill describing the integration in narrative form, code
    // comments, or block quotes. Adding the auth-signin path verbatim is the
    // simplest way to satisfy this lint.
    for (production_skill, expected_auth_signin) in PRODUCTION_SKILL_AUTH_SIGNIN_MAPPING {
        let skill = load_skill_metadata(&repo_skill_dir(production_skill)?)?;
        ensure!(
            skill.skill_markdown.contains(expected_auth_signin),
            "{production_skill} must reference {expected_auth_signin} in its Session Recovery section (no matching substring found)"
        );
    }
    Ok(())
}

#[test]
fn ut_asln_005_auth_signin_skills_declare_local_state_side_effect() -> Result<()> {
    // auth-signin skills mutate the persistent Chrome profile's session
    // cookies / localStorage — that's their entire purpose. Same as the
    // existing auth-refresh/* family (per `ut_skl_009`), they MUST declare
    // side_effect_class: local_state so the daemon knows the run is not
    // purely observational.
    for skill_id in AUTH_SIGNIN_SKILLS {
        let skill = load_skill_metadata(&repo_skill_dir(skill_id)?)?;
        ensure!(
            skill.metadata.side_effect_class == "local_state",
            "{skill_id} must declare side_effect_class:local_state (got {:?}) — auth-signin mutates persistent Chrome profile session state",
            skill.metadata.side_effect_class
        );
    }
    Ok(())
}

#[test]
fn ut_asln_006_chooser_subskill_is_referenced_by_oauth_signins() -> Result<()> {
    // The four Google-OAuth auth-signin skills (ebay, alibaba, searates,
    // freightos) all delegate the account-picker half to the shared
    // `auth-signin/google-account-chooser/` sub-skill. Each must reference
    // it by name (even if the click sequence is inlined for runtime
    // performance, the SKILL.md must cite the canonical source). The
    // procontract skill is exempt — it uses password autofill, not OAuth.
    const OAUTH_SIGNINS: &[&str] = &[
        "auth-signin/ebay",
        "auth-signin/alibaba",
        "auth-signin/searates",
        "auth-signin/freightos",
    ];
    for skill_id in OAUTH_SIGNINS {
        let skill = load_skill_metadata(&repo_skill_dir(skill_id)?)?;
        ensure!(
            skill.skill_markdown.contains("google-account-chooser"),
            "{skill_id} must reference auth-signin/google-account-chooser as the canonical chooser implementation"
        );
    }
    Ok(())
}
