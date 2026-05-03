#![forbid(unsafe_code)]

use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const RUN_MANIFEST_INSTRUCTION_BLOCK: &str = "\
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

Major claims must be anchored to observable evidence such as result-contract fields, daemon appendix fields, or artifact refs. Do not rely on operator memory or hidden context.";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptRendererInput {
    pub skill_id: String,
    pub skill_version: String,
    pub skill_markdown: String,
    pub requires_cdp: bool,
    pub cdp_default_port: Option<u16>,
    pub job_id: String,
    pub session_id: String,
    pub execution_mode: String,
    pub job_parameters: Value,
    pub resume_token: Option<Value>,
    pub company_profile: Option<String>,
}

pub struct PromptRenderer;

impl PromptRenderer {
    pub fn render(input: &PromptRendererInput) -> Result<String> {
        validate_input(input)?;

        let job_parameters = serde_json::to_string_pretty(&input.job_parameters)?;
        let resume_block = input
            .resume_token
            .as_ref()
            .map(serde_json::to_string_pretty)
            .transpose()?
            .map(|token| format!("## Resume Token\n```json\n{token}\n```\n\n"))
            .unwrap_or_default();

        let company_profile_block = input.company_profile.as_ref().map_or_else(
            || "## Company Profile\nNo company profile was provided for this run.\n\n".to_owned(),
            |profile| format!("## Company Profile\n{profile}\n\n"),
        );
        let cdp_contract_block = if input.requires_cdp {
            format!(
                "\
## Persistent CDP Browser Contract
This skill requires the operator persistent CDP browser. The daemon sets `AGENT_BROWSER_CDP_PORT={cdp_port}` for this run.
Use `agent-browser --cdp ${{AGENT_BROWSER_CDP_PORT}}` for every browser command.
Do not close the operator browser. If CDP is unreachable, write a blocked `result.json`; do not launch a browser.

",
                cdp_port = input.cdp_default_port.unwrap_or(9222)
            )
        } else {
            String::new()
        };

        Ok(format!(
            "\
You are executing the bound Layer 1 skill under daemon supervision.

## Execution Context
- skill_id: {skill_id}
- skill_version: {skill_version}
- job_id: {job_id}
- session_id: {session_id}
- execution_mode: {execution_mode}

{cdp_contract_block}\
## Bound SKILL.md
```md
{skill_markdown}
```

## Job Parameters
```json
{job_parameters}
```

{resume_block}{company_profile_block}## Run Manifest Instructions
{run_manifest_instructions}
",
            skill_id = input.skill_id,
            skill_version = input.skill_version,
            job_id = input.job_id,
            session_id = input.session_id,
            execution_mode = input.execution_mode,
            cdp_contract_block = cdp_contract_block,
            skill_markdown = input.skill_markdown.trim(),
            job_parameters = job_parameters,
            resume_block = resume_block,
            company_profile_block = company_profile_block,
            run_manifest_instructions = RUN_MANIFEST_INSTRUCTION_BLOCK,
        ))
    }
}

fn validate_input(input: &PromptRendererInput) -> Result<()> {
    ensure!(
        !input.skill_id.trim().is_empty(),
        "skill_id must not be empty"
    );
    ensure!(
        !input.skill_version.trim().is_empty(),
        "skill_version must not be empty"
    );
    ensure!(
        !input.skill_markdown.trim().is_empty(),
        "skill_markdown must not be empty"
    );
    ensure!(!input.job_id.trim().is_empty(), "job_id must not be empty");
    ensure!(
        !input.session_id.trim().is_empty(),
        "session_id must not be empty"
    );
    ensure!(
        !input.execution_mode.trim().is_empty(),
        "execution_mode must not be empty"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use serde_json::json;

    use super::{PromptRenderer, PromptRendererInput};

    fn input(requires_cdp: bool) -> PromptRendererInput {
        PromptRendererInput {
            skill_id: "test-skill".to_owned(),
            skill_version: "1.0.0".to_owned(),
            skill_markdown: "# Test Skill\n\nBody".to_owned(),
            requires_cdp,
            cdp_default_port: Some(9222),
            job_id: "job-test".to_owned(),
            session_id: "session-test".to_owned(),
            execution_mode: "pty".to_owned(),
            job_parameters: json!({}),
            resume_token: None,
            company_profile: None,
        }
    }

    #[test]
    fn render_includes_persistent_cdp_contract_when_required() -> Result<()> {
        let rendered = PromptRenderer::render(&input(true))?;

        assert!(rendered.contains("## Persistent CDP Browser Contract"));
        assert!(rendered.contains("AGENT_BROWSER_CDP_PORT=9222"));
        assert!(rendered.contains("agent-browser --cdp ${AGENT_BROWSER_CDP_PORT}"));
        assert!(rendered.contains("write a blocked `result.json`"));
        Ok(())
    }

    #[test]
    fn render_omits_persistent_cdp_contract_when_not_required() -> Result<()> {
        let rendered = PromptRenderer::render(&input(false))?;

        assert!(!rendered.contains("## Persistent CDP Browser Contract"));
        assert!(!rendered.contains("AGENT_BROWSER_CDP_PORT=9222"));
        Ok(())
    }
}
