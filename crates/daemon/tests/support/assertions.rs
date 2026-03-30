#![forbid(unsafe_code)]
#![allow(clippy::duplicate_mod)]
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow, bail, ensure};
use serde::de::DeserializeOwned;

pub struct ManifestAssert {
    path: PathBuf,
    frontmatter_yaml: String,
    sections: BTreeMap<String, String>,
}

impl ManifestAssert {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let markdown = fs::read_to_string(&path)
            .with_context(|| format!("reading manifest {}", path.display()))?;
        let (frontmatter_yaml, body) = split_frontmatter(&markdown)?;
        let sections = parse_sections(body);
        Ok(Self {
            path,
            frontmatter_yaml: frontmatter_yaml.to_owned(),
            sections,
        })
    }

    pub fn frontmatter<T: DeserializeOwned>(&self) -> Result<T> {
        serde_yaml::from_str(&self.frontmatter_yaml)
            .with_context(|| format!("parsing frontmatter for {}", self.path.display()))
    }

    pub fn section(&self, title: &str) -> Result<&str> {
        self.sections
            .get(title)
            .map(String::as_str)
            .ok_or_else(|| anyhow!("missing section `{title}`"))
    }

    pub fn require_non_empty_sections(&self, titles: &[&str]) -> Result<()> {
        for title in titles {
            ensure!(
                !self.section(title)?.trim().is_empty(),
                "section `{title}` must not be empty"
            );
        }
        Ok(())
    }

    pub fn appendix_yaml<T: DeserializeOwned>(&self) -> Result<T> {
        let section = self.section("Daemon Evidence Appendix")?;
        let fenced = extract_fenced_block(section)?;
        serde_yaml::from_str(fenced).context("parsing appendix YAML")
    }

    pub fn disagreement_lines(&self) -> Result<Vec<String>> {
        let section = self.section("Claim/Observation Disagreements")?;
        Ok(section
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .map(str::to_owned)
            .collect())
    }

    pub fn file_size(&self) -> Result<usize> {
        Ok(fs::read(&self.path)
            .with_context(|| format!("reading {}", self.path.display()))?
            .len())
    }
}

fn split_frontmatter(markdown: &str) -> Result<(&str, &str)> {
    let rest = markdown
        .strip_prefix("---\n")
        .ok_or_else(|| anyhow!("manifest missing opening frontmatter delimiter"))?;
    let Some((frontmatter, body)) = rest.split_once("\n---\n") else {
        bail!("manifest missing closing frontmatter delimiter");
    };
    Ok((frontmatter, body))
}

fn parse_sections(body: &str) -> BTreeMap<String, String> {
    let mut sections = BTreeMap::new();
    let mut current_title: Option<String> = None;
    let mut current_body = String::new();

    for line in body.lines() {
        if let Some(title) = line.strip_prefix("## ") {
            if let Some(previous) = current_title.take() {
                sections.insert(previous, current_body.trim().to_owned());
                current_body.clear();
            }
            current_title = Some(title.trim().to_owned());
            continue;
        }

        if !current_body.is_empty() {
            current_body.push('\n');
        }
        current_body.push_str(line);
    }

    if let Some(previous) = current_title {
        sections.insert(previous, current_body.trim().to_owned());
    }

    sections
}

fn extract_fenced_block(section: &str) -> Result<&str> {
    let (_, after_open) = section
        .split_once("```")
        .ok_or_else(|| anyhow!("section missing fenced block"))?;
    let after_language = after_open
        .split_once('\n')
        .map_or(after_open, |(_, remainder)| remainder);
    let (fenced, _) = after_language
        .split_once("\n```")
        .ok_or_else(|| anyhow!("section missing fenced block terminator"))?;
    Ok(fenced)
}
