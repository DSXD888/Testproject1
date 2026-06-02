#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, bail, ensure};
use reqwest::Client;
use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};

use super::db::{LogicalColumn, LogicalSchema, normalize_sql_type};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MigrationApplyMode {
    Postgres,
    StaticSql,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedMigrations {
    pub mode: MigrationApplyMode,
    pub paths: Vec<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct SupabaseLocalStack {
    repo_root: PathBuf,
    api_url: String,
    db_url: Option<String>,
    anon_key: Option<String>,
    service_role_key: Option<String>,
}

impl SupabaseLocalStack {
    pub fn start_or_reuse() -> Result<Self> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf)
            .context("daemon crate must live two levels below repository root")?;
        let api_url = env::var("SUPABASE_URL")
            .or_else(|_| env::var("NEXT_PUBLIC_SUPABASE_URL"))
            .unwrap_or_else(|_| "http://127.0.0.1:54321".to_owned());

        Ok(Self {
            repo_root,
            api_url,
            db_url: env::var("L4_SUPABASE_DB_URL").ok(),
            anon_key: env::var("SUPABASE_ANON_KEY")
                .or_else(|_| env::var("NEXT_PUBLIC_SUPABASE_ANON_KEY"))
                .ok(),
            service_role_key: env::var("SUPABASE_SERVICE_ROLE_KEY").ok(),
        })
    }

    pub fn apply_migrations(&self) -> Result<AppliedMigrations> {
        let paths = self.migration_paths()?;
        ensure!(
            paths.len() == 10,
            "expected exactly ten Supabase migrations, found {}",
            paths.len()
        );

        if let Some(db_url) = self.db_url.as_deref().filter(|_| psql_available()) {
            self.reset_schema()?;
            for migration in &paths {
                run_psql(db_url, ["-v", "ON_ERROR_STOP=1", "-f"], Some(migration))?;
            }
            return Ok(AppliedMigrations {
                mode: MigrationApplyMode::Postgres,
                paths,
            });
        }

        let combined_sql = Self::migration_sql(&paths)?;
        ensure!(
            combined_sql.contains("CREATE TABLE procurement_processes"),
            "Supabase projection migration bundle did not contain projection DDL"
        );
        Ok(AppliedMigrations {
            mode: MigrationApplyMode::StaticSql,
            paths,
        })
    }

    pub fn reset_schema(&self) -> Result<()> {
        let Some(db_url) = self.db_url.as_deref() else {
            return Ok(());
        };
        if !psql_available() {
            return Ok(());
        }

        run_psql(
            db_url,
            [
                "-v",
                "ON_ERROR_STOP=1",
                "-c",
                "DROP SCHEMA public CASCADE; CREATE SCHEMA public; GRANT ALL ON SCHEMA public TO postgres; GRANT ALL ON SCHEMA public TO public;",
            ],
            None,
        )
    }

    pub fn api_url(&self) -> &str {
        &self.api_url
    }

    pub fn db_url(&self) -> Option<&str> {
        self.db_url.as_deref()
    }

    pub fn anon_key(&self) -> Option<&str> {
        self.anon_key.as_deref()
    }

    pub fn service_role_key(&self) -> Option<&str> {
        self.service_role_key.as_deref()
    }

    pub fn viewer_client(&self) -> Result<Option<Client>> {
        Self::client_with_bearer(self.anon_key())
    }

    pub fn operator_client(&self) -> Result<Option<Client>> {
        Self::client_with_bearer(self.anon_key())
    }

    pub fn admin_session(&self) -> Result<Option<Client>> {
        Self::client_with_bearer(self.service_role_key())
    }

    pub fn daemon_client(&self) -> Result<Option<Client>> {
        Self::client_with_bearer(self.service_role_key())
    }

    pub fn assert_rls_denied<T, E: std::fmt::Display>(
        result: &std::result::Result<T, E>,
        table: &str,
        role: &str,
    ) -> Result<()> {
        ensure!(
            result.is_err(),
            "expected RLS denial for role `{role}` on `{table}`"
        );
        Ok(())
    }

    pub fn logical_schema_from_migrations(&self, table_names: &[&str]) -> Result<LogicalSchema> {
        let paths = self.migration_paths()?;
        let combined_sql = Self::migration_sql(&paths)?;
        logical_schema_from_sql(&combined_sql, table_names)
    }

    fn migration_paths(&self) -> Result<Vec<PathBuf>> {
        let migration_dir = self.repo_root.join("infra/supabase/migrations");
        let mut paths = fs::read_dir(&migration_dir)
            .with_context(|| format!("failed to read {}", migration_dir.display()))?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<std::io::Result<Vec<_>>>()?;
        paths.retain(|path| path.extension().is_some_and(|extension| extension == "sql"));
        paths.sort();
        Ok(paths)
    }

    fn migration_sql(paths: &[PathBuf]) -> Result<String> {
        let mut combined_sql = String::new();
        for path in paths {
            combined_sql.push_str(&fs::read_to_string(path)?);
            combined_sql.push('\n');
        }
        Ok(combined_sql)
    }

    fn client_with_bearer(bearer: Option<&str>) -> Result<Option<Client>> {
        let Some(bearer) = bearer else {
            return Ok(None);
        };
        let mut headers = HeaderMap::new();
        let header = HeaderValue::from_str(&format!("Bearer {bearer}"))
            .context("invalid Supabase bearer token header")?;
        headers.insert(AUTHORIZATION, header);
        Ok(Some(Client::builder().default_headers(headers).build()?))
    }
}

pub fn logical_schema_from_sql(sql: &str, table_names: &[&str]) -> Result<LogicalSchema> {
    let mut schema = LogicalSchema::default();
    for table_name in table_names {
        let block = create_table_block(sql, table_name)?;
        let mut columns = parse_columns(&block)?;
        ensure!(
            !columns.is_empty(),
            "expected Supabase table `{table_name}` to define at least one column"
        );
        columns.sort_by(|left, right| left.name.cmp(&right.name));
        schema.insert((*table_name).to_owned(), columns);
    }
    Ok(schema)
}

fn create_table_block(sql: &str, table_name: &str) -> Result<String> {
    let lower_sql = sql.to_ascii_lowercase();
    let needles = [
        format!("create table if not exists {table_name}"),
        format!("create table {table_name}"),
    ];
    let start = needles
        .iter()
        .filter_map(|needle| lower_sql.find(needle))
        .min()
        .with_context(|| format!("missing CREATE TABLE block for `{table_name}`"))?;
    let open = sql[start..]
        .find('(')
        .map(|offset| start + offset)
        .with_context(|| format!("missing opening parenthesis for `{table_name}`"))?;
    let close = matching_close_parenthesis(sql, open)
        .with_context(|| format!("missing closing parenthesis for `{table_name}`"))?;
    Ok(sql[open + 1..close].to_owned())
}

fn matching_close_parenthesis(sql: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_single_quote = false;
    for (offset, character) in sql[open..].char_indices() {
        match character {
            '\'' => in_single_quote = !in_single_quote,
            '(' if !in_single_quote => depth += 1,
            ')' if !in_single_quote => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_columns(block: &str) -> Result<Vec<LogicalColumn>> {
    let mut columns = Vec::new();
    for definition in split_top_level_commas(block) {
        let definition = definition.trim();
        if definition.is_empty() || is_table_constraint(definition) {
            continue;
        }
        columns.push(parse_column(definition)?);
    }
    Ok(columns)
}

fn split_top_level_commas(block: &str) -> Vec<String> {
    let mut definitions = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut in_single_quote = false;

    for character in block.chars() {
        match character {
            '\'' => {
                in_single_quote = !in_single_quote;
                current.push(character);
            }
            '(' if !in_single_quote => {
                depth += 1;
                current.push(character);
            }
            ')' if !in_single_quote => {
                depth = depth.saturating_sub(1);
                current.push(character);
            }
            ',' if depth == 0 && !in_single_quote => {
                definitions.push(current.trim().to_owned());
                current.clear();
            }
            _ => current.push(character),
        }
    }

    if !current.trim().is_empty() {
        definitions.push(current.trim().to_owned());
    }

    definitions
}

fn is_table_constraint(definition: &str) -> bool {
    let lower = definition.trim_start().to_ascii_lowercase();
    matches!(
        lower.split_whitespace().next(),
        Some("primary" | "unique" | "check" | "foreign" | "constraint")
    )
}

fn parse_column(definition: &str) -> Result<LogicalColumn> {
    let mut tokens = definition.split_whitespace();
    let name = tokens
        .next()
        .map(clean_identifier)
        .context("column definition missing name")?;

    let mut type_tokens = Vec::new();
    for token in tokens {
        let lower = token.to_ascii_lowercase();
        if matches!(
            lower.as_str(),
            "not" | "null" | "default" | "primary" | "references" | "check" | "unique" | "collate"
        ) {
            break;
        }
        type_tokens.push(token);
    }

    ensure!(
        !type_tokens.is_empty(),
        "column `{name}` definition missing type"
    );
    let raw_type = type_tokens.join(" ");
    let lower_definition = definition.to_ascii_lowercase();
    let nullable =
        !lower_definition.contains("not null") && !lower_definition.contains("primary key");

    Ok(LogicalColumn {
        logical_type: normalize_sql_type(&raw_type, &name),
        name,
        nullable,
    })
}

fn clean_identifier(identifier: &str) -> String {
    identifier
        .trim()
        .trim_matches('"')
        .trim_matches('`')
        .to_owned()
}

fn psql_available() -> bool {
    Command::new("psql").arg("--version").output().is_ok()
}

fn run_psql<const N: usize>(db_url: &str, args: [&str; N], file_arg: Option<&Path>) -> Result<()> {
    let mut command = Command::new("psql");
    command.arg(db_url);
    command.args(args);
    if let Some(path) = file_arg {
        command.arg(path);
    }
    let output = command.output().context("failed to run psql")?;
    if output.status.success() {
        return Ok(());
    }

    bail!(
        "psql failed: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    )
}
