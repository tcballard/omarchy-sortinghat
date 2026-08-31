//! Single-writer durable state journal.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

pub const MAX_ACTIVE: i64 = 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Proposed,
    Approved,
    Copying,
    Published,
    SourceRemoved,
    Completed,
    Undoing,
    Undone,
    FailedSafely,
    NeedsAttention,
    Ignored,
}

impl State {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Approved => "approved",
            Self::Copying => "copying",
            Self::Published => "published",
            Self::SourceRemoved => "source_removed",
            Self::Completed => "completed",
            Self::Undoing => "undoing",
            Self::Undone => "undone",
            Self::FailedSafely => "failed_safely",
            Self::NeedsAttention => "needs_attention",
            Self::Ignored => "ignored",
        }
    }

    fn parse(value: &str) -> Result<Self, JournalError> {
        match value {
            "proposed" => Ok(Self::Proposed),
            "approved" => Ok(Self::Approved),
            "copying" => Ok(Self::Copying),
            "published" => Ok(Self::Published),
            "source_removed" => Ok(Self::SourceRemoved),
            "completed" => Ok(Self::Completed),
            "undoing" => Ok(Self::Undoing),
            "undone" => Ok(Self::Undone),
            "failed_safely" => Ok(Self::FailedSafely),
            "needs_attention" => Ok(Self::NeedsAttention),
            "ignored" => Ok(Self::Ignored),
            _ => Err(JournalError::CorruptState),
        }
    }

    const fn terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Undone | Self::FailedSafely | Self::Ignored
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Entry {
    pub id: Uuid,
    pub revision: u64,
    pub state: State,
    pub source_token: Vec<u8>,
    pub destination_token: Vec<u8>,
    pub size: u64,
    pub sha256: Option<String>,
}

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("database: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("active journal limit reached")]
    Limit,
    #[error("entry not found")]
    NotFound,
    #[error("revision conflict")]
    RevisionConflict,
    #[error("invalid state transition")]
    InvalidTransition,
    #[error("corrupt state")]
    CorruptState,
    #[error("numeric value out of range")]
    Range,
}

pub struct Journal {
    connection: Connection,
}

impl Journal {
    pub fn open(path: &Path) -> Result<Self, JournalError> {
        let connection = Connection::open(path)?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "synchronous", "FULL")?;
        connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS entries (
                id TEXT PRIMARY KEY,
                revision INTEGER NOT NULL,
                state TEXT NOT NULL,
                source_token BLOB NOT NULL,
                destination_token BLOB NOT NULL,
                size INTEGER NOT NULL,
                sha256 TEXT,
                updated_at INTEGER NOT NULL DEFAULT (unixepoch())
            );
            CREATE INDEX IF NOT EXISTS entries_state ON entries(state);",
        )?;
        Ok(Self { connection })
    }

    pub fn create(&mut self, entry: &Entry) -> Result<(), JournalError> {
        if self.active_count()? >= MAX_ACTIVE {
            return Err(JournalError::Limit);
        }
        if entry.state != State::Proposed || entry.revision != 1 {
            return Err(JournalError::InvalidTransition);
        }
        let revision = i64::try_from(entry.revision).map_err(|_| JournalError::Range)?;
        let size = i64::try_from(entry.size).map_err(|_| JournalError::Range)?;
        self.connection.execute(
            "INSERT INTO entries(id, revision, state, source_token, destination_token, size, sha256)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![entry.id.to_string(), revision, entry.state.as_str(), entry.source_token,
                entry.destination_token, size, entry.sha256],
        )?;
        Ok(())
    }

    pub fn get(&self, id: Uuid) -> Result<Entry, JournalError> {
        self.connection
            .query_row(
                "SELECT revision, state, source_token, destination_token, size, sha256
                 FROM entries WHERE id = ?1",
                [id.to_string()],
                |row| {
                    let revision: i64 = row.get(0)?;
                    let raw_state: String = row.get(1)?;
                    let size: i64 = row.get(4)?;
                    Ok((
                        revision,
                        raw_state,
                        row.get(2)?,
                        row.get(3)?,
                        size,
                        row.get(5)?,
                    ))
                },
            )
            .optional()?
            .ok_or(JournalError::NotFound)
            .and_then(
                |(revision, raw_state, source_token, destination_token, size, sha256)| {
                    Ok(Entry {
                        id,
                        revision: u64::try_from(revision).map_err(|_| JournalError::Range)?,
                        state: State::parse(&raw_state)?,
                        source_token,
                        destination_token,
                        size: u64::try_from(size).map_err(|_| JournalError::Range)?,
                        sha256,
                    })
                },
            )
    }

    pub fn transition(
        &mut self,
        id: Uuid,
        revision: u64,
        next: State,
    ) -> Result<Entry, JournalError> {
        let current = self.get(id)?;
        if current.revision != revision {
            return Err(JournalError::RevisionConflict);
        }
        if !allowed(current.state, next) {
            return Err(JournalError::InvalidTransition);
        }
        let next_revision = revision.checked_add(1).ok_or(JournalError::Range)?;
        let revision_sql = i64::try_from(revision).map_err(|_| JournalError::Range)?;
        let next_revision_sql = i64::try_from(next_revision).map_err(|_| JournalError::Range)?;
        let changed = self.connection.execute(
            "UPDATE entries SET state=?1, revision=?2, updated_at=unixepoch()
             WHERE id=?3 AND revision=?4",
            params![
                next.as_str(),
                next_revision_sql,
                id.to_string(),
                revision_sql
            ],
        )?;
        if changed != 1 {
            return Err(JournalError::RevisionConflict);
        }
        self.get(id)
    }

    fn active_count(&self) -> Result<i64, JournalError> {
        Ok(self.connection.query_row(
            "SELECT count(*) FROM entries WHERE state NOT IN ('completed','undone','failed_safely','ignored')",
            [], |row| row.get(0))?)
    }

    pub fn prune_terminal(&mut self, retain: usize) -> Result<usize, JournalError> {
        let retain = i64::try_from(retain).map_err(|_| JournalError::Range)?;
        Ok(self.connection.execute(
            "DELETE FROM entries WHERE id IN (
               SELECT id FROM entries WHERE state IN ('completed','undone','failed_safely','ignored')
               ORDER BY updated_at DESC, rowid DESC LIMIT -1 OFFSET ?1
             )", [retain])?)
    }
}

fn allowed(from: State, to: State) -> bool {
    if to == State::NeedsAttention {
        return !from.terminal();
    }
    matches!(
        (from, to),
        (State::Proposed, State::Approved | State::Ignored)
            | (
                State::Approved,
                State::Copying | State::Published | State::FailedSafely
            )
            | (State::Copying, State::Published | State::FailedSafely)
            | (State::Published, State::SourceRemoved | State::FailedSafely)
            | (State::SourceRemoved, State::Completed)
            | (State::Completed, State::Undoing)
            | (State::Undoing, State::Undone | State::FailedSafely)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_and_transition_are_enforced() {
        let dir = tempfile::tempdir().unwrap();
        let mut journal = Journal::open(&dir.path().join("journal.sqlite3")).unwrap();
        let id = Uuid::new_v4();
        journal
            .create(&Entry {
                id,
                revision: 1,
                state: State::Proposed,
                source_token: b"s".to_vec(),
                destination_token: b"d".to_vec(),
                size: 4,
                sha256: None,
            })
            .unwrap();
        assert!(matches!(
            journal.transition(id, 9, State::Approved),
            Err(JournalError::RevisionConflict)
        ));
        let approved = journal.transition(id, 1, State::Approved).unwrap();
        assert_eq!(approved.revision, 2);
        assert!(matches!(
            journal.transition(id, 2, State::Completed),
            Err(JournalError::InvalidTransition)
        ));
    }
}
