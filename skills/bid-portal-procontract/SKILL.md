---
skill_id: bid-portal/procontract
skill_version: 0.1.0
interaction_profile: pty_interactive
result_schema_version: 1.0.0
checkpoint_schema_version: 1.0.0
model_profile: portal_form_high_reliability
agent_cli: claude
side_effect_class: external_mutation
validation_policy: always
requires_cdp: true
cdp_default_port: 9222
source_structure_version: procontract-bid-portal-2026-05-22
promotion_stage: draft
worker_behavior_version: bid-portal/procontract@0.1.0+1.0
version_change_affects_behavior: true
---

# bid-portal/procontract

## Use when
Use this skill when the daemon has already approved a scoped L6 bid workflow for a Procontract opportunity and needs a PTY browser agent to perform the portal-specific bid response flow.

## Boundary
This Phase 4 skill frontmatter is the frozen metadata contract consumed by the daemon registry, runner, and validator routing. The executable playbook, result schema, checkpoint schema, canary inputs, golden assertions, and validator rubric are added by the dependent Phase 4 beads.
