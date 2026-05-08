#![forbid(unsafe_code)]

use std::io;

use anyhow::{Context, Result, anyhow, ensure};
use rusqlite::{Connection, OptionalExtension, Row, Transaction, params, types::Type};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProvisionalValidationStatus {
    Pending,
    Approved,
    Rejected,
}

impl ProvisionalValidationStatus {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
        }
    }

    fn from_db(value: &str) -> Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            _ => Err(anyhow!("invalid provisional validation status `{value}`")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProvisionalWorkerOutput {
    pub id: String,
    pub worker_job_id: String,
    pub payload_json: String,
    pub result_contract_blob_path: Option<String>,
    pub validation_status: ProvisionalValidationStatus,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InsertProvisionalOutput<'a> {
    pub id: &'a str,
    pub worker_job_id: &'a str,
    pub payload_json: &'a str,
    pub result_contract_blob_path: Option<&'a str>,
}

pub struct ProvisionalWorkerOutputStore;

impl ProvisionalWorkerOutputStore {
    pub fn insert_provisional(
        tx: &Transaction<'_>,
        request: InsertProvisionalOutput<'_>,
    ) -> Result<ProvisionalWorkerOutput> {
        request.validate()?;
        ensure!(
            Self::lookup_by_worker_job_id(tx, request.worker_job_id)?.is_none(),
            "provisional output already exists for worker_job_id `{}`",
            request.worker_job_id
        );

        tx.execute(
            "\
            INSERT INTO provisional_worker_outputs (
              id, worker_job_id, payload_json, result_contract_blob_path, validation_status
            ) VALUES (?1, ?2, ?3, ?4, 'pending')
            ",
            params![
                request.id,
                request.worker_job_id,
                request.payload_json,
                request.result_contract_blob_path
            ],
        )
        .with_context(|| {
            format!(
                "inserting provisional worker output `{}` for job `{}`",
                request.id, request.worker_job_id
            )
        })?;

        Self::lookup_by_id(tx, request.id)?.ok_or_else(|| {
            anyhow!(
                "inserted provisional output `{}` was not readable",
                request.id
            )
        })
    }

    pub fn mark_approved(
        tx: &Transaction<'_>,
        provisional_id: &str,
    ) -> Result<ProvisionalWorkerOutput> {
        Self::mark_status(tx, provisional_id, ProvisionalValidationStatus::Approved)
    }

    pub fn mark_rejected(
        tx: &Transaction<'_>,
        provisional_id: &str,
    ) -> Result<ProvisionalWorkerOutput> {
        Self::mark_status(tx, provisional_id, ProvisionalValidationStatus::Rejected)
    }

    pub fn lookup_by_worker_job_id(
        conn: &Connection,
        worker_job_id: &str,
    ) -> Result<Option<ProvisionalWorkerOutput>> {
        ensure_non_empty("worker_job_id", worker_job_id)?;
        let mut stmt = conn
            .prepare(
                "\
                SELECT id, worker_job_id, payload_json, result_contract_blob_path,
                       validation_status, created_at, updated_at
                FROM provisional_worker_outputs
                WHERE worker_job_id = ?1
                ORDER BY created_at DESC, id DESC
                LIMIT 2
                ",
            )
            .context("preparing provisional worker output lookup by worker_job_id")?;
        let rows = stmt
            .query_map(params![worker_job_id], provisional_row)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .with_context(|| {
                format!("loading provisional worker output for job `{worker_job_id}`")
            })?;
        ensure!(
            rows.len() <= 1,
            "multiple provisional outputs found for worker_job_id `{worker_job_id}`"
        );
        Ok(rows.into_iter().next())
    }

    fn mark_status(
        tx: &Transaction<'_>,
        provisional_id: &str,
        next_status: ProvisionalValidationStatus,
    ) -> Result<ProvisionalWorkerOutput> {
        ensure_non_empty("provisional_id", provisional_id)?;
        let current = Self::lookup_by_id(tx, provisional_id)?.ok_or_else(|| {
            anyhow!("provisional worker output `{provisional_id}` does not exist")
        })?;
        ensure!(
            current.validation_status == ProvisionalValidationStatus::Pending,
            "provisional worker output `{}` is already `{}`",
            current.id,
            current.validation_status.as_str()
        );

        tx.execute(
            "\
            UPDATE provisional_worker_outputs
            SET validation_status = ?1, updated_at = CURRENT_TIMESTAMP
            WHERE id = ?2
            ",
            params![next_status.as_str(), provisional_id],
        )
        .with_context(|| {
            format!(
                "marking provisional worker output `{provisional_id}` as `{}`",
                next_status.as_str()
            )
        })?;

        Self::lookup_by_id(tx, provisional_id)?.ok_or_else(|| {
            anyhow!("updated provisional worker output `{provisional_id}` was not readable")
        })
    }

    fn lookup_by_id(
        conn: &Connection,
        provisional_id: &str,
    ) -> Result<Option<ProvisionalWorkerOutput>> {
        ensure_non_empty("provisional_id", provisional_id)?;
        conn.query_row(
            "\
            SELECT id, worker_job_id, payload_json, result_contract_blob_path,
                   validation_status, created_at, updated_at
            FROM provisional_worker_outputs
            WHERE id = ?1
            ",
            params![provisional_id],
            provisional_row,
        )
        .optional()
        .with_context(|| format!("loading provisional worker output `{provisional_id}`"))
    }
}

impl InsertProvisionalOutput<'_> {
    fn validate(self) -> Result<()> {
        ensure_non_empty("id", self.id)?;
        ensure_non_empty("worker_job_id", self.worker_job_id)?;
        ensure_non_empty("payload_json", self.payload_json)?;
        serde_json::from_str::<serde_json::Value>(self.payload_json)
            .with_context(|| format!("parsing provisional payload JSON for `{}`", self.id))?;
        if let Some(path) = self.result_contract_blob_path {
            ensure_non_empty("result_contract_blob_path", path)?;
        }
        Ok(())
    }
}

fn provisional_row(row: &Row<'_>) -> rusqlite::Result<ProvisionalWorkerOutput> {
    let validation_status: String = row.get(4)?;
    Ok(ProvisionalWorkerOutput {
        id: row.get(0)?,
        worker_job_id: row.get(1)?,
        payload_json: row.get(2)?,
        result_contract_blob_path: row.get(3)?,
        validation_status: ProvisionalValidationStatus::from_db(&validation_status)
            .map_err(|_| invalid_status_error(4, &validation_status))?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn invalid_status_error(index: usize, status: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        index,
        Type::Text,
        Box::new(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid provisional validation status `{status}`"),
        )),
    )
}

fn ensure_non_empty(field: &str, value: &str) -> Result<()> {
    ensure!(!value.trim().is_empty(), "{field} must not be empty");
    Ok(())
}
