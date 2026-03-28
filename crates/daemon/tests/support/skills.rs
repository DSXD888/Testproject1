#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, ensure};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use serde_yaml::Value as YamlValue;
use tempfile::TempDir;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintedSkillMetadata {
    pub skill_id: String,
    pub skill_version: String,
    pub interaction_profile: String,
    pub result_schema_version: String,
    pub model_profile: String,
    pub side_effect_class: String,
    pub max_pages: Option<u32>,
    pub max_scroll_cycles: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintedSkill {
    pub metadata: LintedSkillMetadata,
    pub skill_markdown: String,
}

#[derive(Debug, Deserialize)]
struct SkillFrontmatter {
    skill_id: Option<String>,
    skill_version: Option<String>,
    interaction_profile: Option<String>,
    result_schema_version: Option<String>,
    model_profile: Option<String>,
    side_effect_class: Option<String>,
    #[serde(rename = "maxPages")]
    max_pages: Option<u32>,
    #[serde(rename = "maxScrollCycles")]
    max_scroll_cycles: Option<u32>,
}

#[allow(dead_code)]
pub struct TestSkillDir {
    _temp: TempDir,
    root: PathBuf,
}

#[allow(dead_code)]
impl TestSkillDir {
    pub fn from_fixture(source: &Path) -> Result<Self> {
        Self::from_fixture_filtered(source, None)
    }

    pub fn from_fixture_filtered(source: &Path, skip_relative: Option<&str>) -> Result<Self> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("skill");
        copy_dir(source, &root, skip_relative)?;
        Ok(Self { _temp: temp, root })
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        &self.root
    }

    pub fn overwrite(&self, relative: &str, contents: &str) -> Result<()> {
        let target = self.root.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&target, contents)
            .with_context(|| format!("writing test skill file {}", target.display()))?;
        Ok(())
    }

    pub fn read_skill_markdown(&self) -> Result<String> {
        fs::read_to_string(self.root.join("SKILL.md")).context("reading test SKILL.md")
    }
}

pub fn repo_root() -> Result<PathBuf> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .ok_or_else(|| anyhow!("failed to resolve repository root"))
}

pub fn repo_skill_dir(relative: &str) -> Result<PathBuf> {
    Ok(repo_root()?.join("skills").join(relative))
}

pub fn lint_skill_directory(path: &Path) -> Result<LintedSkill> {
    let paths = SkillFilePaths::new(path);
    validate_required_files(&paths)?;

    let skill_markdown = fs::read_to_string(&paths.skill_md)
        .with_context(|| format!("reading {}", paths.skill_md.display()))?;
    let (frontmatter, body) = split_frontmatter(&skill_markdown)?;
    let metadata = validate_metadata(frontmatter)?;
    validate_sections(body)?;
    validate_run_manifest_instructions(body)?;
    validate_skill_specific_bounds(&metadata)?;
    validate_json_file(&paths.result_schema, "result.schema.json")?;
    validate_json_file(&paths.checkpoint_schema, "checkpoint.schema.json")?;
    validate_canary_inputs(&paths.canary_inputs)?;
    validate_golden_assertions(&paths.golden_assertions)?;

    Ok(LintedSkill {
        metadata,
        skill_markdown,
    })
}

fn validate_metadata(frontmatter: SkillFrontmatter) -> Result<LintedSkillMetadata> {
    let skill_id = required(frontmatter.skill_id, "skill_id")?;
    let skill_version = required(frontmatter.skill_version, "skill_version")?;
    let interaction_profile = required(frontmatter.interaction_profile, "interaction_profile")?;
    let result_schema_version =
        required(frontmatter.result_schema_version, "result_schema_version")?;
    let model_profile = required(frontmatter.model_profile, "model_profile")?;
    let side_effect_class = required(frontmatter.side_effect_class, "side_effect_class")?;

    Ok(LintedSkillMetadata {
        skill_id,
        skill_version,
        interaction_profile,
        result_schema_version,
        model_profile,
        side_effect_class,
        max_pages: frontmatter.max_pages,
        max_scroll_cycles: frontmatter.max_scroll_cycles,
    })
}

struct SkillFilePaths {
    skill_md: PathBuf,
    result_schema: PathBuf,
    checkpoint_schema: PathBuf,
    canary_inputs: PathBuf,
    golden_assertions: PathBuf,
}

impl SkillFilePaths {
    fn new(root: &Path) -> Self {
        Self {
            skill_md: root.join("SKILL.md"),
            result_schema: root.join("result.schema.json"),
            checkpoint_schema: root.join("checkpoint.schema.json"),
            canary_inputs: root.join("canary.inputs.jsonl"),
            golden_assertions: root.join("golden.assertions.yaml"),
        }
    }
}

fn validate_required_files(paths: &SkillFilePaths) -> Result<()> {
    ensure!(paths.skill_md.is_file(), "SKILL.md missing");
    ensure!(paths.result_schema.is_file(), "result.schema.json missing");
    ensure!(
        paths.checkpoint_schema.is_file(),
        "checkpoint.schema.json missing"
    );
    ensure!(paths.canary_inputs.is_file(), "canary.inputs.jsonl missing");
    ensure!(
        paths.golden_assertions.is_file(),
        "golden.assertions.yaml missing"
    );
    Ok(())
}

fn validate_sections(body: &str) -> Result<()> {
    for section in [
        "Use when",
        "Do not use when",
        "Inputs",
        "Outputs",
        "Non-goals",
    ] {
        ensure!(
            !extract_section(body, section)?.trim().is_empty(),
            "missing required section `{section}`"
        );
    }

    ensure!(
        !extract_section(body, "Failure Codes")?.trim().is_empty(),
        "failure-code enumeration missing"
    );
    ensure!(
        !extract_section(body, "Resume Strategy")?.trim().is_empty(),
        "resume strategy missing"
    );
    Ok(())
}

fn validate_run_manifest_instructions(body: &str) -> Result<()> {
    let manifest_section = extract_section(body, "Run Manifest Instructions")?;
    ensure!(
        !manifest_section.trim().is_empty(),
        "run-manifest instructions missing"
    );

    for token in [
        "manifest_version",
        "skill_id",
        "skill_version",
        "job_id",
        "session_id",
        "execution_mode",
        "Summary",
        "Steps Taken",
        "Decisions Made",
        "Warnings",
        "Evidence",
    ] {
        ensure!(
            manifest_section.contains(token),
            "run-manifest instructions missing `{token}`"
        );
    }
    Ok(())
}

fn validate_skill_specific_bounds(metadata: &LintedSkillMetadata) -> Result<()> {
    if metadata.interaction_profile == "pty_interactive" {
        ensure!(metadata.max_pages.is_some(), "maxPages missing");
        ensure!(
            metadata.max_scroll_cycles.is_some(),
            "maxScrollCycles missing"
        );
    }
    Ok(())
}

fn validate_json_file(path: &Path, label: &str) -> Result<()> {
    let parsed: JsonValue = serde_json::from_str(
        &fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?,
    )
    .with_context(|| format!("parsing {label}"))?;
    ensure!(
        !parsed.as_object().is_none_or(serde_json::Map::is_empty),
        "{label} must be non-empty"
    );
    Ok(())
}

fn validate_canary_inputs(path: &Path) -> Result<()> {
    let canary_inputs =
        fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut canary_count = 0_u32;
    for line in canary_inputs.lines().filter(|line| !line.trim().is_empty()) {
        let _: JsonValue = serde_json::from_str(line).context("parsing canary.inputs.jsonl")?;
        canary_count += 1;
    }
    ensure!(canary_count > 0, "canary.inputs.jsonl must be non-empty");
    Ok(())
}

fn validate_golden_assertions(path: &Path) -> Result<()> {
    let parsed: YamlValue = serde_yaml::from_str(
        &fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?,
    )
    .context("parsing golden.assertions.yaml")?;
    ensure!(
        !matches!(parsed, YamlValue::Null),
        "golden.assertions.yaml must be non-empty"
    );
    Ok(())
}

fn required(value: Option<String>, field: &str) -> Result<String> {
    let value = value.ok_or_else(|| anyhow!("missing required metadata field `{field}`"))?;
    ensure!(
        !value.trim().is_empty(),
        "metadata field `{field}` is empty"
    );
    Ok(value)
}

fn split_frontmatter(markdown: &str) -> Result<(SkillFrontmatter, &str)> {
    let rest = markdown
        .strip_prefix("---\n")
        .ok_or_else(|| anyhow!("SKILL.md missing YAML frontmatter"))?;
    let (frontmatter, body) = rest
        .split_once("\n---\n")
        .ok_or_else(|| anyhow!("SKILL.md missing frontmatter terminator"))?;
    let frontmatter = serde_yaml::from_str(frontmatter).context("parsing SKILL.md frontmatter")?;
    Ok((frontmatter, body))
}

fn extract_section<'a>(body: &'a str, title: &str) -> Result<&'a str> {
    let heading = format!("## {title}");
    let start = body
        .find(&heading)
        .ok_or_else(|| anyhow!("missing required section `{title}`"))?;
    let section_body = &body[start + heading.len()..];
    let end = section_body.find("\n## ").unwrap_or(section_body.len());
    Ok(section_body[..end].trim())
}

#[allow(dead_code)]
fn copy_dir(source: &Path, destination: &Path, skip_relative: Option<&str>) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let relative = path
            .strip_prefix(source)
            .context("stripping source prefix")?;
        if skip_relative.is_some_and(|skip| relative == Path::new(skip)) {
            continue;
        }
        let target = destination.join(relative);
        if path.is_dir() {
            copy_dir(&path, &target, None)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&path, &target)?;
        }
    }
    Ok(())
}
