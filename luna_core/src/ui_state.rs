//! Transient UI state: where a page was when you left it.
//!
//! Scroll position, unsaved draft text, which layer was selected, which panels were
//! expanded. Restored when a tool's page reopens, so navigating away and back does not
//! feel like starting over.
//!
//! ## Kept well away from user data
//!
//! Stored as one small file per tool under `<data_root>/ui-state`, not in the
//! database. The two have opposite requirements: user data must never be lost, and UI
//! state must never be a reason anything fails. Keeping them in separate files means a
//! corrupt snapshot is deleted and forgotten, and can never take a note or a reminder
//! with it.
//!
//! Every failure path here is silent by design. A snapshot that will not parse, has
//! expired, was written by a newer version, or is simply missing all produce the same
//! result: the page opens fresh.
//!
//! ## The payload is opaque
//!
//! Tools serialise whatever they like and hand over a string. This module only adds a
//! timestamp and decides whether the result is still worth restoring, which keeps it
//! from ever needing to know what a tool's state looks like.

use std::path::{Path, PathBuf};

use chrono::{DateTime, TimeZone, Utc};

use crate::atomic;
use crate::config::{ToolConfig, UiStateTtl};
use crate::error::Result;
use crate::paths::validate_tool_id;

pub const VERSION: luna::Version = luna::Version::new(0, 1, 0);

/// The largest snapshot that will be stored, in bytes.
///
/// UI state is meant to be a scroll offset and some draft text. A tool trying to keep
/// a decoded image here is doing something the image editor's filter stack exists to
/// avoid, and silently obliging it would undo the memory work elsewhere.
pub const MAX_SNAPSHOT_BYTES: usize = 256 * 1024;

/// The on-disk wrapper around a tool's snapshot.
///
/// The time to live is deliberately *not* stored. It is read from the tool's current
/// settings when loading, so shortening it takes effect on what is already saved
/// rather than only on what is written next.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct Envelope {
    /// Unix seconds, UTC.
    written_at: i64,
    /// Whatever the tool handed over.
    state: String,
}

/// Why a snapshot was not restored. Useful in tests and logs, never shown to a user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Discarded {
    /// Nothing was saved.
    Missing,
    /// The tool has turned the feature off.
    Disabled,
    /// The snapshot is older than the tool's time to live.
    Expired,
    /// The file could not be read or parsed.
    Unreadable,
}

/// Per-tool UI snapshots on disk.
pub struct UiStateStore {
    dir: PathBuf,
}

impl UiStateStore {
    /// Opens the store under a data root. The directory is created on first write.
    pub fn new(data_root: &Path) -> Self {
        return Self {
            dir: data_root.join("ui-state"),
        };
    }

    /// The directory snapshots live in.
    pub fn dir(&self) -> &Path {
        return &self.dir;
    }

    fn path_for(&self, tool_id: &str) -> Result<PathBuf> {
        validate_tool_id(tool_id)?;
        return Ok(self.dir.join(format!("{tool_id}.json")));
    }

    /// Saves a tool's snapshot, if its settings allow it.
    ///
    /// Does nothing when the tool has `remember_ui_state` off or a time to live of
    /// `never`, so callers do not have to check first.
    ///
    /// ## Errors
    /// Only genuine IO failures. An oversized snapshot is refused with
    /// [`crate::CoreError::SendRefused`]-style reporting rather than truncated, since
    /// half a snapshot restores worse than none.
    pub fn save(
        &self,
        tool_id: &str,
        config: &ToolConfig,
        state: &str,
        now: DateTime<Utc>,
    ) -> Result<bool> {
        if !Self::wants_saving(config) {
            // Turning the setting off should also take away what was already kept.
            self.clear(tool_id)?;
            return Ok(false);
        }

        if state.len() > MAX_SNAPSHOT_BYTES {
            eprintln!(
                "ui state for {tool_id} is {} bytes, over the {MAX_SNAPSHOT_BYTES} byte limit, \
                 so it was not saved",
                state.len()
            );
            return Ok(false);
        }

        let envelope = Envelope {
            written_at: now.timestamp(),
            state: state.to_string(),
        };

        // Cannot fail: the envelope is two owned fields.
        let text = serde_json::to_string(&envelope).unwrap_or_default();

        atomic::write_string(&self.path_for(tool_id)?, &text)?;

        return Ok(true);
    }

    /// Restores a tool's snapshot, or explains why there is nothing to restore.
    ///
    /// A snapshot that cannot be used is deleted on the way out, so a file that will
    /// never parse is not re-read on every page open for the rest of time.
    pub fn load(
        &self,
        tool_id: &str,
        config: &ToolConfig,
        now: DateTime<Utc>,
    ) -> std::result::Result<String, Discarded> {
        if !config.remember_ui_state || config.ui_state_ttl == UiStateTtl::Never {
            return Err(Discarded::Disabled);
        }

        let Ok(path) = self.path_for(tool_id) else {
            return Err(Discarded::Unreadable);
        };

        let Ok(text) = std::fs::read_to_string(&path) else {
            return Err(Discarded::Missing);
        };

        let Ok(envelope) = serde_json::from_str::<Envelope>(&text) else {
            let _ = std::fs::remove_file(&path);
            return Err(Discarded::Unreadable);
        };

        let Some(written_at) = Utc.timestamp_opt(envelope.written_at, 0).single() else {
            let _ = std::fs::remove_file(&path);
            return Err(Discarded::Unreadable);
        };

        if Self::has_expired(config.ui_state_ttl, written_at, now) {
            let _ = std::fs::remove_file(&path);
            return Err(Discarded::Expired);
        }

        return Ok(envelope.state);
    }

    /// Deletes a tool's snapshot.
    pub fn clear(&self, tool_id: &str) -> Result<()> {
        let path = self.path_for(tool_id)?;

        // Absent is the desired end state, so a missing file is success.
        let _ = std::fs::remove_file(path);

        return Ok(());
    }

    /// Deletes snapshots for tools whose state is only meant to last a session.
    ///
    /// Called on a clean exit. A tool set to `session` that is killed rather than
    /// closed keeps its snapshot until the next clean exit, which is the right way
    /// round: the alternative is losing state every time the app crashes.
    pub fn discard_session_state<'a>(
        &self,
        tools: impl Iterator<Item = (&'a str, &'a ToolConfig)>,
    ) -> Result<usize> {
        let mut cleared = 0;

        for (tool_id, config) in tools {
            if config.ui_state_ttl == UiStateTtl::Session {
                self.clear(tool_id)?;
                cleared += 1;
            }
        }

        return Ok(cleared);
    }

    /// Deletes every snapshot, for a "reset the interface" action.
    pub fn clear_all(&self) -> Result<usize> {
        let Ok(entries) = std::fs::read_dir(&self.dir) else {
            return Ok(0);
        };

        let mut cleared = 0;

        for entry in entries.flatten() {
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("json")
                && std::fs::remove_file(&path).is_ok()
            {
                cleared += 1;
            }
        }

        return Ok(cleared);
    }

    /// Whether a tool's settings mean anything should be written at all.
    fn wants_saving(config: &ToolConfig) -> bool {
        return config.remember_ui_state && config.ui_state_ttl != UiStateTtl::Never;
    }

    /// Whether a snapshot written at a given time is too old to use.
    fn has_expired(ttl: UiStateTtl, written_at: DateTime<Utc>, now: DateTime<Utc>) -> bool {
        return match ttl {
            UiStateTtl::Never => true,
            // Session state is cleared on a clean exit rather than by age, so within a
            // run it never goes stale.
            UiStateTtl::Session => false,
            UiStateTtl::Forever => false,
            UiStateTtl::For(duration) => {
                let Ok(max_age) = chrono::Duration::from_std(duration) else {
                    // A duration too large to represent is effectively forever.
                    return false;
                };

                // A snapshot from the future means the clock moved backwards. Treating
                // it as valid is the kinder reading: the state is probably still what
                // the user left behind.
                now.signed_duration_since(written_at) > max_age
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration as StdDuration;

    fn store() -> (tempfile::TempDir, UiStateStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = UiStateStore::new(dir.path());
        return (dir, store);
    }

    fn now() -> DateTime<Utc> {
        return Utc.with_ymd_and_hms(2026, 8, 14, 12, 0, 0).unwrap();
    }

    fn config(ttl: UiStateTtl) -> ToolConfig {
        let mut config = ToolConfig::default();
        config.remember_ui_state = true;
        config.ui_state_ttl = ttl;
        return config;
    }

    #[test]
    fn a_snapshot_round_trips() {
        let (_dir, store) = store();
        let config = config(UiStateTtl::Forever);

        assert!(store.save("luna.notes", &config, "{\"scroll\":42}", now()).unwrap());

        assert_eq!(
            store.load("luna.notes", &config, now()),
            Ok("{\"scroll\":42}".to_string())
        );
    }

    #[test]
    fn nothing_saved_means_nothing_to_restore() {
        let (_dir, store) = store();

        assert_eq!(
            store.load("luna.notes", &config(UiStateTtl::Forever), now()),
            Err(Discarded::Missing)
        );
    }

    #[test]
    fn snapshots_are_per_tool() {
        let (_dir, store) = store();
        let config = config(UiStateTtl::Forever);

        store.save("luna.a", &config, "state a", now()).unwrap();
        store.save("luna.b", &config, "state b", now()).unwrap();

        assert_eq!(store.load("luna.a", &config, now()), Ok("state a".to_string()));
        assert_eq!(store.load("luna.b", &config, now()), Ok("state b".to_string()));
    }

    #[test]
    fn a_snapshot_expires_once_it_is_older_than_the_ttl() {
        let (_dir, store) = store();
        let config = config(UiStateTtl::For(StdDuration::from_secs(60 * 60)));

        store.save("luna.notes", &config, "state", now()).unwrap();

        // Well within the hour.
        assert!(store
            .load("luna.notes", &config, now() + chrono::Duration::minutes(30))
            .is_ok());

        // And past it.
        assert_eq!(
            store.load("luna.notes", &config, now() + chrono::Duration::hours(2)),
            Err(Discarded::Expired)
        );
    }

    #[test]
    fn an_expired_snapshot_is_deleted_rather_than_re_read_forever() {
        let (_dir, store) = store();
        let config = config(UiStateTtl::For(StdDuration::from_secs(60)));

        store.save("luna.notes", &config, "state", now()).unwrap();

        let later = now() + chrono::Duration::hours(1);
        assert_eq!(store.load("luna.notes", &config, later), Err(Discarded::Expired));

        // Gone, so the next open is a plain miss rather than another expiry check.
        assert_eq!(store.load("luna.notes", &config, later), Err(Discarded::Missing));
    }

    #[test]
    fn shortening_the_ttl_applies_to_what_was_already_saved() {
        let (_dir, store) = store();

        let generous = config(UiStateTtl::For(StdDuration::from_secs(7 * 24 * 60 * 60)));
        store.save("luna.notes", &generous, "state", now()).unwrap();

        // The user changes their mind an hour later. The TTL is read from settings at
        // load time, not from the file, so the existing snapshot is judged by the new
        // rule.
        let strict = config(UiStateTtl::For(StdDuration::from_secs(60)));

        assert_eq!(
            store.load("luna.notes", &strict, now() + chrono::Duration::hours(1)),
            Err(Discarded::Expired)
        );
    }

    #[test]
    fn forever_never_expires() {
        let (_dir, store) = store();
        let config = config(UiStateTtl::Forever);

        store.save("luna.notes", &config, "state", now()).unwrap();

        assert!(store
            .load("luna.notes", &config, now() + chrono::Duration::days(3650))
            .is_ok());
    }

    #[test]
    fn session_state_does_not_expire_within_a_run() {
        let (_dir, store) = store();
        let config = config(UiStateTtl::Session);

        store.save("luna.notes", &config, "state", now()).unwrap();

        assert!(store
            .load("luna.notes", &config, now() + chrono::Duration::days(30))
            .is_ok());
    }

    #[test]
    fn session_state_is_discarded_on_a_clean_exit() {
        let (_dir, store) = store();

        let session = config(UiStateTtl::Session);
        let kept = config(UiStateTtl::Forever);

        store.save("luna.transient", &session, "state", now()).unwrap();
        store.save("luna.durable", &kept, "state", now()).unwrap();

        let cleared = store
            .discard_session_state(
                [("luna.transient", &session), ("luna.durable", &kept)].into_iter(),
            )
            .unwrap();

        assert_eq!(cleared, 1);
        assert_eq!(
            store.load("luna.transient", &session, now()),
            Err(Discarded::Missing)
        );
        assert!(store.load("luna.durable", &kept, now()).is_ok());
    }

    #[test]
    fn turning_the_setting_off_refuses_to_save_and_removes_what_was_kept() {
        let (_dir, store) = store();

        let on = config(UiStateTtl::Forever);
        store.save("luna.notes", &on, "state", now()).unwrap();

        let mut off = on.clone();
        off.remember_ui_state = false;

        assert!(!store.save("luna.notes", &off, "state", now()).unwrap());
        assert_eq!(store.load("luna.notes", &off, now()), Err(Discarded::Disabled));

        // And turning it back on does not resurrect the old snapshot.
        assert_eq!(store.load("luna.notes", &on, now()), Err(Discarded::Missing));
    }

    #[test]
    fn a_ttl_of_never_saves_nothing() {
        let (_dir, store) = store();
        let config = config(UiStateTtl::Never);

        assert!(!store.save("luna.notes", &config, "state", now()).unwrap());
        assert_eq!(store.load("luna.notes", &config, now()), Err(Discarded::Disabled));
    }

    #[test]
    fn a_corrupt_snapshot_is_discarded_silently_and_deleted() {
        let (_dir, store) = store();
        let config = config(UiStateTtl::Forever);

        store.save("luna.notes", &config, "state", now()).unwrap();

        // As a partial write or a hand edit might leave it.
        let path = store.path_for("luna.notes").unwrap();
        std::fs::write(&path, "{ not json").unwrap();

        assert_eq!(
            store.load("luna.notes", &config, now()),
            Err(Discarded::Unreadable)
        );
        assert!(!path.exists(), "an unusable snapshot should not be kept");
    }

    #[test]
    fn an_oversized_snapshot_is_refused_rather_than_truncated() {
        let (_dir, store) = store();
        let config = config(UiStateTtl::Forever);

        let huge = "x".repeat(MAX_SNAPSHOT_BYTES + 1);

        assert!(!store.save("luna.greedy", &config, &huge, now()).unwrap());
        assert_eq!(
            store.load("luna.greedy", &config, now()),
            Err(Discarded::Missing),
            "half a snapshot restores worse than none"
        );
    }

    #[test]
    fn a_snapshot_at_exactly_the_limit_is_kept() {
        let (_dir, store) = store();
        let config = config(UiStateTtl::Forever);

        let limit = "x".repeat(MAX_SNAPSHOT_BYTES);

        assert!(store.save("luna.big", &config, &limit, now()).unwrap());
        assert!(store.load("luna.big", &config, now()).is_ok());
    }

    #[test]
    fn a_clock_that_moved_backwards_does_not_discard_good_state() {
        let (_dir, store) = store();
        let config = config(UiStateTtl::For(StdDuration::from_secs(60)));

        store.save("luna.notes", &config, "state", now()).unwrap();

        // The wall clock is corrected backwards. The snapshot is probably still what
        // the user left behind, so keeping it is the kinder reading.
        assert!(store
            .load("luna.notes", &config, now() - chrono::Duration::hours(5))
            .is_ok());
    }

    #[test]
    fn saving_replaces_the_previous_snapshot() {
        let (_dir, store) = store();
        let config = config(UiStateTtl::Forever);

        store.save("luna.notes", &config, "first", now()).unwrap();
        store.save("luna.notes", &config, "second", now()).unwrap();

        assert_eq!(store.load("luna.notes", &config, now()), Ok("second".to_string()));
    }

    #[test]
    fn clear_all_empties_the_store() {
        let (_dir, store) = store();
        let config = config(UiStateTtl::Forever);

        store.save("luna.a", &config, "a", now()).unwrap();
        store.save("luna.b", &config, "b", now()).unwrap();

        assert_eq!(store.clear_all().unwrap(), 2);
        assert_eq!(store.load("luna.a", &config, now()), Err(Discarded::Missing));
    }

    #[test]
    fn a_tool_id_that_would_escape_the_directory_is_rejected() {
        let (_dir, store) = store();
        let config = config(UiStateTtl::Forever);

        assert!(store.save("../escape", &config, "state", now()).is_err());
        assert_eq!(
            store.load("../escape", &config, now()),
            Err(Discarded::Unreadable)
        );
    }

    #[test]
    fn ui_state_lives_apart_from_user_data() {
        let (dir, store) = store();
        let config = config(UiStateTtl::Forever);

        store.save("luna.notes", &config, "state", now()).unwrap();

        // Its own directory, so it can be wiped wholesale without going near the
        // database or a tool's own files.
        assert_eq!(store.dir(), dir.path().join("ui-state"));
        assert!(store.dir().is_dir());
    }
}
