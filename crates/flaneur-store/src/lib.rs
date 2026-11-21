// SPDX-License-Identifier: AGPL-3.0-or-later

//! Local storage of Flâneur, in one SQLite file.
//!
//! The store holds what must survive a restart and work offline: resume positions and history,
//! subscriptions of services without accounts, settings, and the writes waiting to reach a
//! service. Secrets never go here; the app keeps sessions in the OS keyring.

use std::path::Path;

use flaneur_model::{ChannelRef, ChannelSummary, Item, ItemRef, Progress, ServiceId, Timestamp};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Serialize, de::DeserializeOwned};

/// Schema migrations, applied in order; `PRAGMA user_version` records how many ran.
const MIGRATIONS: &[&str] = &[include_str!("../migrations/0001_initial.sql")];

/// Error of a store operation.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// SQLite failed.
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    /// A stored JSON value no longer matches its type.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// Result of a store operation.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// One entry of the local history.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    /// The item.
    pub r#ref: ItemRef,
    /// The item as last seen, when it was stored.
    pub item: Option<Item>,
    /// Resume position and duration.
    pub progress: Progress,
    /// Counted as watched.
    pub watched: bool,
}

/// A write queued for a service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboxEntry {
    /// Row id, to acknowledge it.
    pub id: i64,
    /// Target service.
    pub service: ServiceId,
    /// JSON operation.
    pub operation: String,
    /// Failed attempts so far.
    pub attempts: u32,
}

/// The local database.
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Opens (or creates) the database file and migrates it.
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::init(Connection::open(path)?)
    }

    /// An in-memory database, for tests.
    pub fn in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", true)?;
        let mut store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&mut self) -> Result<()> {
        let done: i64 = self
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))?;
        let tx = self.conn.transaction()?;
        for (version, sql) in (1_i64..).zip(MIGRATIONS) {
            if version > done {
                tx.execute_batch(sql)?;
                tx.pragma_update(None, "user_version", version)?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Reads a setting.
    pub fn setting<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let json: Option<String> = self
            .conn
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?;
        json.map(|j| serde_json::from_str(&j))
            .transpose()
            .map_err(Into::into)
    }

    /// Writes a setting.
    pub fn set_setting<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT (key) DO UPDATE SET value = excluded.value",
            params![key, serde_json::to_string(value)?],
        )?;
        Ok(())
    }

    /// Records playback progress. A newer record replaces an older one; an older one is ignored,
    /// so a late sync from a service never moves the resume position back.
    pub fn record_progress(
        &self,
        r#ref: &ItemRef,
        item: Option<&Item>,
        progress: Progress,
        watched: bool,
    ) -> Result<()> {
        let item_json = item.map(serde_json::to_string).transpose()?;
        self.conn.execute(
            "INSERT INTO history (service, item_id, item, position, duration, watched, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (service, item_id) DO UPDATE SET
                item = coalesce(excluded.item, history.item),
                position = excluded.position,
                duration = coalesce(excluded.duration, history.duration),
                watched = excluded.watched OR history.watched,
                updated_at = excluded.updated_at
             WHERE excluded.updated_at >= history.updated_at",
            params![
                r#ref.service.0,
                r#ref.id,
                item_json,
                progress.position,
                progress.duration,
                watched,
                progress.updated_at
            ],
        )?;
        Ok(())
    }

    /// The local history entry of an item.
    pub fn progress(&self, r#ref: &ItemRef) -> Result<Option<HistoryEntry>> {
        self.conn
            .query_row(
                "SELECT service, item_id, item, position, duration, watched, updated_at
                 FROM history WHERE service = ?1 AND item_id = ?2",
                params![r#ref.service.0, r#ref.id],
                row_to_entry,
            )
            .optional()?
            .map(finish_entry)
            .transpose()
    }

    /// The local history, most recent first.
    pub fn history(&self, limit: u32, before: Option<Timestamp>) -> Result<Vec<HistoryEntry>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT service, item_id, item, position, duration, watched, updated_at
             FROM history WHERE updated_at < ?1 ORDER BY updated_at DESC LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![before.unwrap_or(i64::MAX), limit], row_to_entry)?;
        rows.map(|r| finish_entry(r?)).collect()
    }

    /// Follows a channel locally.
    pub fn add_local_subscription(&self, channel: &ChannelSummary, now: Timestamp) -> Result<()> {
        self.conn.execute(
            "INSERT INTO local_subscriptions (service, channel_id, channel, added_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT (service, channel_id) DO UPDATE SET channel = excluded.channel",
            params![
                channel.r#ref.service.0,
                channel.r#ref.id,
                serde_json::to_string(channel)?,
                now
            ],
        )?;
        Ok(())
    }

    /// Unfollows a local channel.
    pub fn remove_local_subscription(&self, channel: &ChannelRef) -> Result<()> {
        self.conn.execute(
            "DELETE FROM local_subscriptions WHERE service = ?1 AND channel_id = ?2",
            params![channel.service.0, channel.id],
        )?;
        Ok(())
    }

    /// The channels followed locally on one service, oldest first.
    pub fn local_subscriptions(&self, service: &ServiceId) -> Result<Vec<ChannelSummary>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT channel FROM local_subscriptions WHERE service = ?1 ORDER BY added_at",
        )?;
        let rows = stmt.query_map([&service.0], |row| row.get::<_, String>(0))?;
        rows.map(|json| Ok(serde_json::from_str(&json?)?)).collect()
    }

    /// Queues a write for a service.
    pub fn enqueue(&self, service: &ServiceId, operation: &str, now: Timestamp) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO outbox (service, operation, created_at) VALUES (?1, ?2, ?3)",
            params![service.0, operation, now],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    /// The queued writes, oldest first.
    pub fn outbox(&self, limit: u32) -> Result<Vec<OutboxEntry>> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT id, service, operation, attempts FROM outbox ORDER BY id LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit], |row| {
            Ok(OutboxEntry {
                id: row.get(0)?,
                service: ServiceId(row.get(1)?),
                operation: row.get(2)?,
                attempts: row.get(3)?,
            })
        })?;
        rows.collect::<rusqlite::Result<_>>().map_err(Into::into)
    }

    /// Removes a write that reached its service.
    pub fn acknowledge(&self, id: i64) -> Result<()> {
        self.conn
            .execute("DELETE FROM outbox WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Counts a failed attempt of a queued write.
    pub fn record_failure(&self, id: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE outbox SET attempts = attempts + 1 WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }
}

type RawEntry = (String, String, Option<String>, u32, Option<u32>, bool, i64);

fn row_to_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<RawEntry> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
    ))
}

fn finish_entry(
    (service, id, item, position, duration, watched, updated_at): RawEntry,
) -> Result<HistoryEntry> {
    Ok(HistoryEntry {
        r#ref: ItemRef::new(ServiceId(service), id),
        item: item.map(|j| serde_json::from_str(&j)).transpose()?,
        progress: Progress {
            position,
            duration,
            updated_at,
        },
        watched,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yt(id: &str) -> ItemRef {
        ItemRef::new(ServiceId::new("youtube"), id)
    }

    fn at(position: u32, updated_at: i64) -> Progress {
        Progress {
            position,
            duration: Some(600),
            updated_at,
        }
    }

    #[test]
    fn migrations_are_idempotent() {
        let mut store = Store::in_memory().unwrap();
        store.migrate().unwrap();
        let version: i64 = store
            .conn
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .unwrap();
        assert_eq!(version, i64::try_from(MIGRATIONS.len()).unwrap());
    }

    #[test]
    fn settings_round_trip() {
        let store = Store::in_memory().unwrap();
        assert_eq!(store.setting::<u32>("max_height").unwrap(), None);
        store.set_setting("max_height", &1080u32).unwrap();
        store.set_setting("max_height", &720u32).unwrap();
        assert_eq!(store.setting::<u32>("max_height").unwrap(), Some(720));
    }

    #[test]
    fn older_progress_never_overwrites_newer() {
        let store = Store::in_memory().unwrap();
        store
            .record_progress(&yt("a"), None, at(300, 2000), false)
            .unwrap();
        store
            .record_progress(&yt("a"), None, at(10, 1000), false)
            .unwrap();
        assert_eq!(
            store.progress(&yt("a")).unwrap().unwrap().progress.position,
            300
        );
    }

    #[test]
    fn watched_sticks_once_set() {
        let store = Store::in_memory().unwrap();
        store
            .record_progress(&yt("a"), None, at(590, 1000), true)
            .unwrap();
        store
            .record_progress(&yt("a"), None, at(20, 2000), false)
            .unwrap();
        let entry = store.progress(&yt("a")).unwrap().unwrap();
        assert!(entry.watched);
        assert_eq!(entry.progress.position, 20);
    }

    #[test]
    fn history_is_most_recent_first() {
        let store = Store::in_memory().unwrap();
        store
            .record_progress(&yt("old"), None, at(1, 1000), false)
            .unwrap();
        store
            .record_progress(&yt("new"), None, at(1, 3000), false)
            .unwrap();
        store
            .record_progress(&yt("mid"), None, at(1, 2000), false)
            .unwrap();
        let ids: Vec<_> = store
            .history(10, None)
            .unwrap()
            .into_iter()
            .map(|e| e.r#ref.id)
            .collect();
        assert_eq!(ids, ["new", "mid", "old"]);
        assert_eq!(store.history(10, Some(2000)).unwrap()[0].r#ref.id, "old");
    }

    #[test]
    fn outbox_keeps_order_until_acknowledged() {
        let store = Store::in_memory().unwrap();
        let svc = ServiceId::new("youtube");
        let first = store.enqueue(&svc, r#"{"like":"a"}"#, 1).unwrap();
        store.enqueue(&svc, r#"{"like":"b"}"#, 2).unwrap();
        store.record_failure(first).unwrap();
        let queued = store.outbox(10).unwrap();
        assert_eq!(queued.len(), 2);
        assert_eq!(queued[0].attempts, 1);
        store.acknowledge(first).unwrap();
        assert_eq!(store.outbox(10).unwrap().len(), 1);
    }
}
