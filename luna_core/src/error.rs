//! Error types shared across the host services.

use std::path::PathBuf;

pub const VERSION: luna::Version = luna::Version::new(0, 1, 0);

/// Any failure raised by the host services in this crate.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    /// The directory holding the executable could not be determined.
    ///
    /// Luna is a portable app and anchors everything to that directory, so there is
    /// no sensible fallback if it cannot be found.
    #[error("could not determine the install directory: {0}")]
    InstallDirUnknown(String),

    /// The install directory exists but cannot be written to.
    ///
    /// This is the `Program Files` case: an unelevated process cannot create the
    /// `config/` anchor. Luna deliberately does not fall back to a system location,
    /// so this is fatal and the user has to move the install or elevate.
    #[error("the install directory is not writable: {path}\n\
             Luna keeps all of its files next to the executable. Move the install to a \
             location you can write to, such as your user folder.")]
    InstallDirNotWritable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A user-configured `data_root` could not be created or written to.
    #[error("the configured data directory is not usable: {path}")]
    DataRootNotWritable {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// A tool id could not be turned into a safe path component.
    #[error("invalid tool id {id:?}: {reason}")]
    InvalidToolId { id: String, reason: &'static str },

    /// An IO operation failed, with the path it was operating on.
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Serialising a config value to TOML failed.
    #[error("could not serialise config: {0}")]
    ConfigSerialise(#[from] toml::ser::Error),

    /// A database operation failed.
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    /// A migration failed to apply, identified by version and name.
    #[error("migration {version} ({name}) failed: {source}")]
    Migration {
        version: i64,
        name: &'static str,
        #[source]
        source: rusqlite::Error,
    },
}

/// Convenience alias for results produced by this crate.
pub type Result<T> = std::result::Result<T, CoreError>;

/// Attaches a path to an [`std::io::Error`], so failures say *what* they failed on.
pub(crate) trait IoResultExt<T> {
    fn at_path(self, path: impl Into<PathBuf>) -> Result<T>;
}

impl<T> IoResultExt<T> for std::result::Result<T, std::io::Error> {
    fn at_path(self, path: impl Into<PathBuf>) -> Result<T> {
        return self.map_err(|source| CoreError::Io {
            path: path.into(),
            source,
        });
    }
}
