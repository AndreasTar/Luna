//! The SQLite database.
//!
//! One file at `<data_root>/luna.db`, opened through `rusqlite` with the bundled
//! engine so there is no external process and no system dependency. It holds the
//! structured data that outlives a session: calendar events, reminder rules, the
//! event log, samples and entries.
//!
//! Config stays in TOML (see [`crate::config`]) and large binary artefacts stay as
//! files under `data/tools/<id>/`. Neither belongs in here.
//!
//! Schema changes are applied by [`MIGRATIONS`], in order, each inside a transaction
//! and recorded in `_luna_schema`. Migrations are append-only: once a version has
//! shipped, its SQL is never edited, because a user's database may already have
//! applied it.

use std::path::Path;

use rusqlite::Connection;

use crate::error::{CoreError, Result};

pub const VERSION: luna::Version = luna::Version::new(0, 1, 0);

/// A single forward schema change.
struct Migration {
    version: i64,
    name: &'static str,
    sql: &'static str,
}

/// Every schema change, in order. Append only; never edit a shipped entry.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "initial",
        sql: "
            CREATE TABLE _luna_meta (
                key   TEXT NOT NULL PRIMARY KEY,
                value TEXT NOT NULL
            ) STRICT;
        ",
    },
    Migration {
        version: 2,
        name: "rule_events",
        sql: "
            -- What has happened to scheduled rules. Guards are predicates over this
            -- table, and window tasks anchor on the latest completion in it, so it is
            -- read far more often than it is written.
            CREATE TABLE rule_events (
                id      INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                rule_id TEXT    NOT NULL,
                kind    TEXT    NOT NULL,
                -- Unix seconds UTC. Absolute, never a duration: a deadline stored as
                -- 'in N seconds' does not survive the app being closed.
                at      INTEGER NOT NULL,
                -- Which tool owns the rule, so removing a tool can take its history
                -- with it. Null for rules the host itself owns.
                tool_id TEXT,
                note    TEXT
            ) STRICT;

            -- Guards ask 'how many of this kind, for this rule, in this span' on every
            -- candidate instant, so that exact shape is indexed.
            CREATE INDEX idx_rule_events_lookup ON rule_events (rule_id, kind, at);

            -- Pruning and per-tool cleanup scan by time and by owner.
            CREATE INDEX idx_rule_events_at ON rule_events (at);
            CREATE INDEX idx_rule_events_tool ON rule_events (tool_id);
        ",
    },
    Migration {
        version: 3,
        name: "scheduled_jobs",
        sql: "
            -- Registered scheduled work. Stores the recurrence rule, never a computed
            -- next-fire timestamp: a stored instant goes wrong the moment a clock, a
            -- timezone or a daylight saving rule moves.
            CREATE TABLE scheduled_jobs (
                rule_id  TEXT    NOT NULL PRIMARY KEY,
                tool_id  TEXT,
                schedule TEXT    NOT NULL,
                guard    TEXT    NOT NULL,
                catch_up TEXT    NOT NULL,
                enabled  INTEGER NOT NULL
            ) STRICT;

            CREATE INDEX idx_scheduled_jobs_tool ON scheduled_jobs (tool_id);
        ",
    },
    Migration {
        version: 4,
        name: "job_lead_time",
        sql: "
            -- How far before each occurrence the job fires. Non-zero separates an alert
            -- from the thing it is about, such as a reminder a week before a birthday.
            --
            -- Added as its own migration rather than by editing version 3, which had
            -- already been applied. Changing a shipped migration leaves every existing
            -- database without the change, because the runner only applies versions it
            -- has not seen.
            ALTER TABLE scheduled_jobs
                ADD COLUMN lead_seconds INTEGER NOT NULL DEFAULT 0;
        ",
    },
    Migration {
        version: 5,
        name: "calendar",
        sql: "
            -- Calendar entries. Times are unix seconds UTC, like every other instant in
            -- here, including for all-day entries: those store the local day's midnight
            -- to the next midnight, converted on the way in. Storing a wall-clock string
            -- instead would make 'what is on this day' a string comparison that breaks
            -- the moment the machine moves timezone.
            CREATE TABLE calendar_event (
                id        INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                title     TEXT    NOT NULL,
                starts_at INTEGER NOT NULL,
                ends_at   INTEGER NOT NULL,
                all_day   INTEGER NOT NULL,
                -- A serialised luna::rules::Schedule for a repeating entry, null for a
                -- one-off. Serialised rather than given columns of its own because the
                -- recurrence engine already owns that shape, and two descriptions of one
                -- rule is one more than can be kept in agreement.
                recurrence TEXT,
                -- How long before the entry its reminder fires. Zero means the reminder
                -- is the entry itself; non-zero separates the two, which is what a
                -- birthday with 'buy a gift' a week earlier needs.
                reminder_lead_seconds INTEGER NOT NULL DEFAULT 0,
                notes     TEXT
            ) STRICT;

            -- Every read is 'what falls in this span', for a day, a month or the list of
            -- what is coming.
            CREATE INDEX idx_calendar_event_span ON calendar_event (starts_at, ends_at);

            -- One free-text note per day. Keyed by the local date as YYYY-MM-DD rather
            -- than by an instant: a note belongs to the day as written on the wall, not
            -- to a moment in it.
            CREATE TABLE calendar_note (
                day        TEXT    NOT NULL PRIMARY KEY,
                body       TEXT    NOT NULL,
                updated_at INTEGER NOT NULL
            ) STRICT;
        ",
    },
];

/// An open connection to Luna's database, migrated to the current schema.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Opens (or creates) the database at `path` and brings it up to date.
    ///
    /// ## Errors
    /// [`CoreError::Database`] if the file cannot be opened, or
    /// [`CoreError::Migration`] if a schema change fails, naming the one that broke.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)
                    .map_err(|source| CoreError::Io { path: parent.to_path_buf(), source })?;
            }
        }

        let conn = Connection::open(path)?;
        return Self::from_connection(conn);
    }

    /// Opens a private in-memory database, migrated the same way. For tests.
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        return Self::from_connection(conn);
    }

    fn from_connection(conn: Connection) -> Result<Self> {
        let mut db = Self { conn };
        db.apply_pragmas()?;
        db.migrate()?;
        return Ok(db);
    }

    /// Configures durability and concurrency behaviour for the connection.
    fn apply_pragmas(&self) -> Result<()> {
        // WAL lets a reader and a writer work at once and survives a hard kill. It is
        // a no-op for in-memory databases, which report "memory" instead, so the
        // result is queried and discarded rather than checked.
        let _: String = self
            .conn
            .query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;

        // NORMAL with WAL is the usual durable-enough pairing: a crash cannot corrupt
        // the file, at the cost of possibly losing the very last commit on power loss.
        self.conn.execute_batch(
            "
            PRAGMA synchronous = NORMAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            ",
        )?;

        return Ok(());
    }

    /// Applies every migration newer than the recorded schema version.
    fn migrate(&mut self) -> Result<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS _luna_schema (
                version    INTEGER NOT NULL PRIMARY KEY,
                name       TEXT    NOT NULL,
                applied_at INTEGER NOT NULL
            ) STRICT;
            ",
        )?;

        let current = self.schema_version()?;

        for migration in MIGRATIONS.iter().filter(|m| m.version > current) {
            let tx = self.conn.transaction()?;

            tx.execute_batch(migration.sql)
                .map_err(|source| CoreError::Migration {
                    version: migration.version,
                    name: migration.name,
                    source,
                })?;

            tx.execute(
                "INSERT INTO _luna_schema (version, name, applied_at) VALUES (?1, ?2, ?3)",
                rusqlite::params![migration.version, migration.name, crate::epoch_seconds()],
            )
            .map_err(|source| CoreError::Migration {
                version: migration.version,
                name: migration.name,
                source,
            })?;

            tx.commit().map_err(|source| CoreError::Migration {
                version: migration.version,
                name: migration.name,
                source,
            })?;
        }

        return Ok(());
    }

    /// The highest migration version applied, or `0` on a fresh database.
    pub fn schema_version(&self) -> Result<i64> {
        let version = self.conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM _luna_schema",
            [],
            |row| row.get(0),
        )?;
        return Ok(version);
    }

    /// The schema version this build of Luna expects.
    pub fn expected_schema_version() -> i64 {
        return MIGRATIONS.last().map(|m| m.version).unwrap_or(0);
    }

    /// Borrows the underlying connection, for tools running their own statements.
    pub fn conn(&self) -> &Connection {
        return &self.conn;
    }

    /// Borrows the connection mutably, for transactions.
    pub fn conn_mut(&mut self) -> &mut Connection {
        return &mut self.conn;
    }

    /// Unwraps to the underlying connection.
    ///
    /// For components that want to own a connection outright rather than borrow one,
    /// such as [`crate::events::EventLog`].
    pub fn into_connection(self) -> Connection {
        return self.conn;
    }

    /// Reads a value from the key/value metadata table.
    pub fn meta_get(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self.conn.prepare("SELECT value FROM _luna_meta WHERE key = ?1")?;
        let mut rows = stmt.query([key])?;

        return match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        };
    }

    /// Writes a value to the key/value metadata table, replacing any existing one.
    pub fn meta_set(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO _luna_meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [key, value],
        )?;
        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_database_is_migrated_to_the_current_version() {
        let db = Database::open_in_memory().unwrap();

        assert_eq!(db.schema_version().unwrap(), Database::expected_schema_version());
        assert!(Database::expected_schema_version() >= 1);
    }

    #[test]
    fn migrations_are_recorded_with_their_names() {
        let db = Database::open_in_memory().unwrap();

        let name: String = db
            .conn()
            .query_row("SELECT name FROM _luna_schema WHERE version = 1", [], |r| r.get(0))
            .unwrap();

        assert_eq!(name, "initial");
    }

    #[test]
    fn reopening_does_not_reapply_migrations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("luna.db");

        let applied_first = {
            let db = Database::open(&path).unwrap();
            db.meta_set("greeting", "hello").unwrap();
            count_applied(&db)
        };

        let db = Database::open(&path).unwrap();

        assert_eq!(count_applied(&db), applied_first, "migrations ran twice");
        assert_eq!(db.meta_get("greeting").unwrap(), Some("hello".to_string()));
    }

    #[test]
    fn creates_missing_parent_directories() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("deeper").join("luna.db");

        let db = Database::open(&path).unwrap();

        assert!(path.exists());
        assert_eq!(db.schema_version().unwrap(), Database::expected_schema_version());
    }

    #[test]
    fn meta_round_trips_and_overwrites() {
        let db = Database::open_in_memory().unwrap();

        assert_eq!(db.meta_get("absent").unwrap(), None);

        db.meta_set("key", "first").unwrap();
        assert_eq!(db.meta_get("key").unwrap(), Some("first".to_string()));

        db.meta_set("key", "second").unwrap();
        assert_eq!(db.meta_get("key").unwrap(), Some("second".to_string()));
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let db = Database::open_in_memory().unwrap();

        let enabled: i64 = db
            .conn()
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();

        assert_eq!(enabled, 1);
    }

    #[test]
    fn a_database_stopped_at_an_older_version_catches_all_the_way_up() {
        // The failure this guards against is subtle and was hit for real: editing an
        // already-shipped migration leaves every existing database without the change,
        // because the runner only applies versions it has not seen. Stepping a database
        // up one version at a time is the only way to notice.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("luna.db");

        let latest = Database::expected_schema_version();

        for stop_at in 1..latest {
            let _ = std::fs::remove_file(&path);

            // Bring a database up to an intermediate version, as an older release
            // would have left it.
            {
                let conn = Connection::open(&path).unwrap();
                conn.execute_batch(
                    "CREATE TABLE IF NOT EXISTS _luna_schema (
                        version    INTEGER NOT NULL PRIMARY KEY,
                        name       TEXT    NOT NULL,
                        applied_at INTEGER NOT NULL
                    ) STRICT;",
                )
                .unwrap();

                for migration in MIGRATIONS.iter().filter(|m| m.version <= stop_at) {
                    conn.execute_batch(migration.sql).unwrap();
                    conn.execute(
                        "INSERT INTO _luna_schema (version, name, applied_at) VALUES (?1, ?2, 0)",
                        rusqlite::params![migration.version, migration.name],
                    )
                    .unwrap();
                }
            }

            // Then open it the way the app would, and check it arrives complete.
            let db = Database::open(&path).unwrap();
            assert_eq!(
                db.schema_version().unwrap(),
                latest,
                "a database at version {stop_at} did not catch up"
            );

            // Every table the app relies on must be usable, not merely present.
            db.conn()
                .execute_batch(
                    "SELECT rule_id, tool_id, schedule, guard, catch_up, enabled, lead_seconds
                     FROM scheduled_jobs;
                     SELECT rule_id, kind, at, tool_id, note FROM rule_events;
                     SELECT key, value FROM _luna_meta;",
                )
                .unwrap_or_else(|e| panic!("schema unusable after upgrading from {stop_at}: {e}"));
        }
    }

    #[test]
    fn migration_versions_are_unique_and_ascending() {
        let versions: Vec<i64> = MIGRATIONS.iter().map(|m| m.version).collect();

        let mut sorted = versions.clone();
        sorted.sort_unstable();
        sorted.dedup();

        assert_eq!(versions, sorted, "migrations must be unique and in ascending order");
    }

    fn count_applied(db: &Database) -> i64 {
        return db
            .conn()
            .query_row("SELECT COUNT(*) FROM _luna_schema", [], |row| row.get(0))
            .unwrap();
    }
}
