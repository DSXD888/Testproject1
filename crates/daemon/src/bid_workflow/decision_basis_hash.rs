use std::fmt::Display;

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};

use super::types::BidScopePayload;

#[derive(Debug, Clone, PartialEq, Eq)]
struct QuoteVersionVector {
    quote_id: String,
    row_version: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CandidateFreshnessVector {
    candidate_id: String,
    classification_fresh_until: String,
    classification_expired: bool,
    l5_vector_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct L5BlockerVector {
    candidate_id: String,
    blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompanyProfileVersionVector {
    profile_id: String,
    remote_revision: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DecisionBasisSnapshot {
    pipeline_assessment_hash: String,
    quote_versions: Vec<QuoteVersionVector>,
    company_profile: CompanyProfileVersionVector,
    candidate_freshness: Vec<CandidateFreshnessVector>,
    l5_blockers: Vec<L5BlockerVector>,
}

pub fn compute(conn: &Connection, scope: &BidScopePayload) -> Result<String> {
    let snapshot = load_snapshot(conn, scope)?;
    Ok(snapshot.hash())
}

impl DecisionBasisSnapshot {
    fn hash(&self) -> String {
        let mut hasher = Sha256::new();

        update_field(&mut hasher, "pa", &self.pipeline_assessment_hash);

        hasher.update(b"qv[");
        for quote in &self.quote_versions {
            update_field(&mut hasher, &quote.quote_id, &quote.row_version);
        }
        hasher.update(b"]");

        hasher.update(b"cp[");
        update_field(&mut hasher, "id", &self.company_profile.profile_id);
        update_field(
            &mut hasher,
            "remote_revision",
            &self.company_profile.remote_revision,
        );
        hasher.update(b"]");

        hasher.update(b"cl[");
        for candidate in &self.candidate_freshness {
            hasher.update(candidate.candidate_id.as_bytes());
            hasher.update(b"{");
            update_field(
                &mut hasher,
                "classification_fresh_until",
                &candidate.classification_fresh_until,
            );
            update_field(
                &mut hasher,
                "classification_expired",
                &candidate.classification_expired,
            );
            update_field(&mut hasher, "l5_vector_hash", &candidate.l5_vector_hash);
            hasher.update(b"}");
        }
        hasher.update(b"]");

        hasher.update(b"bl[");
        for candidate in &self.l5_blockers {
            hasher.update(candidate.candidate_id.as_bytes());
            hasher.update(b"{");
            for blocker in &candidate.blockers {
                update_field(&mut hasher, "blocker", blocker);
            }
            hasher.update(b"}");
        }
        hasher.update(b"]");

        format!("{:x}", hasher.finalize())
    }
}

fn load_snapshot(conn: &Connection, scope: &BidScopePayload) -> Result<DecisionBasisSnapshot> {
    let pipeline_assessment_hash = load_pipeline_assessment_hash(conn, &scope.process_id)?;
    let quote_versions = load_quote_versions(conn, &scope.selected_quote_ids)?;
    let company_profile = load_company_profile_version(conn, &scope.company_profile_version)?;
    let candidate_freshness =
        load_candidate_freshness_vectors(conn, &scope.candidate_ids, &scope.selected_quote_ids)?;
    let l5_blockers =
        load_l5_blocker_vectors(conn, &scope.candidate_ids, &scope.selected_quote_ids)?;

    Ok(DecisionBasisSnapshot {
        pipeline_assessment_hash,
        quote_versions,
        company_profile,
        candidate_freshness,
        l5_blockers,
    })
}

fn load_pipeline_assessment_hash(conn: &Connection, process_id: &str) -> Result<String> {
    conn.query_row(
        "\
        SELECT decision_basis_hash
        FROM pipeline_assessments
        WHERE process_id = ?1
        ",
        params![process_id],
        |row| row.get::<_, Option<String>>(0),
    )
    .optional()
    .context("loading pipeline_assessments.decision_basis_hash")?
    .flatten()
    .map(|value| value.trim().to_owned())
    .filter(|value| !value.is_empty())
    .ok_or_else(|| anyhow!("missing pipeline_assessments.decision_basis_hash for `{process_id}`"))
}

fn load_quote_versions(conn: &Connection, quote_ids: &[String]) -> Result<Vec<QuoteVersionVector>> {
    let mut vectors = Vec::with_capacity(quote_ids.len());
    for quote_id in quote_ids {
        let row_version = conn
            .query_row(
                "\
                SELECT row_version
                FROM supplier_quotes
                WHERE id = ?1
                ",
                params![quote_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .with_context(|| format!("loading supplier quote row_version for `{quote_id}`"))?
            .ok_or_else(|| anyhow!("missing selected supplier quote `{quote_id}`"))?;
        vectors.push(QuoteVersionVector {
            quote_id: quote_id.clone(),
            row_version,
        });
    }
    vectors.sort_by(|left, right| left.quote_id.cmp(&right.quote_id));
    Ok(vectors)
}

fn load_company_profile_version(
    conn: &Connection,
    company_profile_version: &str,
) -> Result<CompanyProfileVersionVector> {
    conn.query_row(
        "\
        SELECT id, remote_revision
        FROM shared_company_profile
        WHERE (id = ?1 OR CAST(remote_revision AS TEXT) = ?1)
          AND deleted_at IS NULL
        ORDER BY id
        LIMIT 1
        ",
        params![company_profile_version],
        |row| {
            Ok(CompanyProfileVersionVector {
                profile_id: row.get(0)?,
                remote_revision: row.get(1)?,
            })
        },
    )
    .optional()
    .context("loading shared_company_profile version")?
    .ok_or_else(|| anyhow!("missing shared_company_profile version `{company_profile_version}`"))
}

fn load_candidate_freshness_vectors(
    conn: &Connection,
    candidate_ids: &[String],
    selected_quote_ids: &[String],
) -> Result<Vec<CandidateFreshnessVector>> {
    let mut vectors = Vec::with_capacity(candidate_ids.len());
    for candidate_id in candidate_ids {
        let row = load_landed_cost_basis_row(conn, candidate_id, selected_quote_ids)?;
        let classification_fresh_until = row
            .classification_fresh_until
            .clone()
            .unwrap_or_else(|| "missing".to_owned());
        vectors.push(CandidateFreshnessVector {
            candidate_id: candidate_id.clone(),
            classification_expired: classification_fresh_until_expired(&classification_fresh_until),
            classification_fresh_until,
            l5_vector_hash: row.l5_vector_hash(),
        });
    }
    vectors.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    Ok(vectors)
}

fn load_l5_blocker_vectors(
    conn: &Connection,
    candidate_ids: &[String],
    selected_quote_ids: &[String],
) -> Result<Vec<L5BlockerVector>> {
    let mut vectors = Vec::with_capacity(candidate_ids.len());
    for candidate_id in candidate_ids {
        let row = load_landed_cost_basis_row(conn, candidate_id, selected_quote_ids)?;
        let mut blockers = row.active_blockers()?;
        blockers.sort();
        blockers.dedup();
        vectors.push(L5BlockerVector {
            candidate_id: candidate_id.clone(),
            blockers,
        });
    }
    vectors.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    Ok(vectors)
}

#[derive(Debug, Clone)]
struct LandedCostBasisRow {
    candidate_id: String,
    estimate_id: String,
    supplier_quote_id: String,
    decision_grade: String,
    blockers_json: String,
    tariff_checked_at: Option<String>,
    measures_fresh_until: Option<String>,
    classification_fresh_until: Option<String>,
    trade_remedy_watch_state: Option<String>,
    trade_remedy_cases_json: Option<String>,
    duty_breakdown_json: Option<String>,
    measure_interaction_json: Option<String>,
    fx_rate_checked_at: Option<String>,
    customs_fx_rate: String,
    commercial_fx_rate: String,
    fx_basis_delta_bps: i64,
}

impl LandedCostBasisRow {
    fn l5_vector_hash(&self) -> String {
        let mut hasher = Sha256::new();
        update_field(&mut hasher, "candidate_id", &self.candidate_id);
        update_field(&mut hasher, "estimate_id", &self.estimate_id);
        update_field(&mut hasher, "supplier_quote_id", &self.supplier_quote_id);
        update_field(
            &mut hasher,
            "tariff_checked_at",
            opt(self.tariff_checked_at.as_ref()),
        );
        update_field(
            &mut hasher,
            "measures_fresh_until",
            opt(self.measures_fresh_until.as_ref()),
        );
        update_field(
            &mut hasher,
            "trade_remedy_watch_state",
            opt(self.trade_remedy_watch_state.as_ref()),
        );
        update_field(
            &mut hasher,
            "trade_remedy_cases_json",
            opt(self.trade_remedy_cases_json.as_ref()),
        );
        update_field(
            &mut hasher,
            "duty_breakdown_json",
            opt(self.duty_breakdown_json.as_ref()),
        );
        update_field(
            &mut hasher,
            "measure_interaction_json",
            opt(self.measure_interaction_json.as_ref()),
        );
        update_field(
            &mut hasher,
            "fx_rate_checked_at",
            opt(self.fx_rate_checked_at.as_ref()),
        );
        update_field(&mut hasher, "customs_fx_rate", &self.customs_fx_rate);
        update_field(&mut hasher, "commercial_fx_rate", &self.commercial_fx_rate);
        update_field(&mut hasher, "fx_basis_delta_bps", &self.fx_basis_delta_bps);
        format!("{:x}", hasher.finalize())
    }

    fn active_blockers(&self) -> Result<Vec<String>> {
        let mut blockers = parse_blocker_codes(&self.blockers_json)?;
        if self.decision_grade == "broker_required" {
            blockers.push("decision_grade=broker_required".to_owned());
        }
        if let Some(classification_fresh_until) = self.classification_fresh_until.as_deref() {
            if classification_fresh_until_expired(classification_fresh_until) {
                blockers.push("classification_fresh_until_expired".to_owned());
            }
        } else {
            blockers.push("classification_fresh_until_missing".to_owned());
        }
        Ok(blockers)
    }
}

fn load_landed_cost_basis_row(
    conn: &Connection,
    candidate_id: &str,
    selected_quote_ids: &[String],
) -> Result<LandedCostBasisRow> {
    let quote_placeholders = std::iter::repeat_n("?", selected_quote_ids.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "\
        SELECT
          id,
          product_candidate_id,
          supplier_quote_id,
          decision_grade,
          blockers,
          tariff_checked_at,
          measures_fresh_until,
          classification_fresh_until,
          trade_remedy_watch_state,
          trade_remedy_cases_json,
          duty_breakdown_json,
          measure_interaction_json,
          fx_rate_checked_at,
          CAST(customs_fx_rate AS TEXT),
          CAST(commercial_fx_rate AS TEXT),
          fx_basis_delta_bps
        FROM landed_cost_estimates
        WHERE product_candidate_id = ?
          AND supplier_quote_id IN ({quote_placeholders})
        ORDER BY row_version DESC, updated_at DESC, id DESC
        LIMIT 1
        "
    );
    let params = std::iter::once(&candidate_id as &dyn rusqlite::ToSql)
        .chain(
            selected_quote_ids
                .iter()
                .map(|quote_id| quote_id as &dyn rusqlite::ToSql),
        )
        .collect::<Vec<_>>();

    conn.query_row(&sql, params.as_slice(), |row| {
        Ok(LandedCostBasisRow {
            estimate_id: row.get(0)?,
            candidate_id: row.get(1)?,
            supplier_quote_id: row.get(2)?,
            decision_grade: row.get(3)?,
            blockers_json: row.get(4)?,
            tariff_checked_at: row.get(5)?,
            measures_fresh_until: row.get(6)?,
            classification_fresh_until: row.get(7)?,
            trade_remedy_watch_state: row.get(8)?,
            trade_remedy_cases_json: row.get(9)?,
            duty_breakdown_json: row.get(10)?,
            measure_interaction_json: row.get(11)?,
            fx_rate_checked_at: row.get(12)?,
            customs_fx_rate: row.get(13)?,
            commercial_fx_rate: row.get(14)?,
            fx_basis_delta_bps: row.get(15)?,
        })
    })
    .optional()
    .with_context(|| format!("loading landed_cost_estimates basis row for `{candidate_id}`"))?
    .ok_or_else(|| {
        anyhow!(
            "missing landed_cost_estimates row for candidate `{candidate_id}` and selected quotes"
        )
    })
}

fn parse_blocker_codes(blockers_json: &str) -> Result<Vec<String>> {
    let value = serde_json::from_str::<serde_json::Value>(blockers_json)
        .with_context(|| format!("parsing landed_cost_estimates.blockers `{blockers_json}`"))?;
    let Some(items) = value.as_array() else {
        return Err(anyhow!(
            "landed_cost_estimates.blockers must be a JSON array, got `{blockers_json}`"
        ));
    };

    let mut blockers = Vec::new();
    for item in items {
        match item {
            serde_json::Value::String(code) if !code.trim().is_empty() => {
                blockers.push(code.trim().to_owned());
            }
            serde_json::Value::Object(object) => {
                if let Some(code) = object
                    .get("code")
                    .and_then(serde_json::Value::as_str)
                    .map(str::trim)
                    .filter(|code| !code.is_empty())
                {
                    blockers.push(code.to_owned());
                }
            }
            _ => {}
        }
    }
    Ok(blockers)
}

fn classification_fresh_until_expired(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed == "missing" {
        return true;
    }
    DateTime::parse_from_rfc3339(trimmed).map_or(true, |fresh_until| {
        fresh_until.with_timezone(&Utc) <= Utc::now()
    })
}

fn update_field<T>(hasher: &mut Sha256, key: &str, value: &T)
where
    T: Display + ?Sized,
{
    hasher.update(key.as_bytes());
    hasher.update(b"=");
    hasher.update(value.to_string().as_bytes());
    hasher.update(b";");
}

fn opt(value: Option<&String>) -> &str {
    value.map_or("missing", String::as_str)
}
