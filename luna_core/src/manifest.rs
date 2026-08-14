//! What a tool declares about itself.
//!
//! Every tool ships a manifest. It is read by the build script when it discovers
//! tools, and by the [registry](crate::registry) at runtime to decide what appears in
//! the sidebar, what runs in the background, and which tools can hand data to which.
//!
//! Because all tools are first-party and compiled in, a manifest is a description of
//! intent rather than a security boundary. Unknown fields are still rejected: a
//! misspelled key in a first-party file is a typo worth catching, not a forward
//! compatibility problem, since manifests ship with the binary that reads them.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{CoreError, Result};
use crate::paths::validate_tool_id;

pub const VERSION: luna::Version = luna::Version::new(0, 1, 0);

/// Everything a tool declares about itself.
///
/// ## Example
///
/// ```
/// # use luna_core::manifest::ToolManifest;
/// let manifest: ToolManifest = toml::from_str(r#"
///     id       = "luna.img_manipulator"
///     name     = "Image Manipulator"
///     version  = "0.3.0"
///     category = "media"
///
///     background = false
///
///     offers  = ["luna/image"]
///     accepts = ["luna/image", "luna/file-path"]
/// "#).unwrap();
///
/// assert_eq!(manifest.name, "Image Manipulator");
/// assert_eq!(manifest.version.minor, 3);
/// assert!(manifest.accepts(&"luna/image".parse().unwrap()));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolManifest {
    /// Stable, namespaced identifier. Never changes once shipped, because it keys the
    /// tool's config file, its data directory and its scheduled jobs.
    pub id: String,

    /// Human-readable name, shown in the sidebar and the tool manager.
    pub name: String,

    /// The tool's own version, independent of Luna's.
    #[serde(with = "version_string")]
    pub version: luna::Version,

    /// One-line explanation, shown under the title on the tool's page.
    #[serde(default)]
    pub description: String,

    /// Path to an icon, relative to the tool's own directory.
    #[serde(default)]
    pub icon: Option<String>,

    /// Grouping for the sidebar and the tool manager.
    #[serde(default = "default_category")]
    pub category: String,

    /// Whether the tool needs a [`crate::registry::ToolService`] running while the app
    /// is open, even with its page closed.
    ///
    /// A base converter does not. A reminders tool does. Tools that do not need one
    /// should not declare it: an idle service is memory spent for nothing, and idle
    /// footprint is the whole point of the background model.
    #[serde(default)]
    pub background: bool,

    /// Data this tool can hand to others.
    #[serde(default)]
    pub offers: Vec<PortType>,

    /// Data this tool will accept from others.
    #[serde(default)]
    pub accepts: Vec<PortType>,

    /// Version of the tool's own persisted state, for migrations.
    #[serde(default = "default_state_schema")]
    pub state_schema: u32,
}

fn default_category() -> String {
    return "general".to_string();
}

fn default_state_schema() -> u32 {
    return 1;
}

impl ToolManifest {
    /// Parses a manifest from TOML and validates it.
    pub fn from_toml(text: &str) -> Result<Self> {
        let manifest: Self = toml::from_str(text).map_err(|e| CoreError::InvalidManifest {
            id: "<unparsed>".to_string(),
            reason: e.to_string(),
        })?;

        manifest.validate()?;
        return Ok(manifest);
    }

    /// Checks the parts serde cannot: usable id, non-empty name, no duplicate or
    /// contradictory ports.
    pub fn validate(&self) -> Result<()> {
        validate_tool_id(&self.id)?;

        let invalid = |reason: String| CoreError::InvalidManifest {
            id: self.id.clone(),
            reason,
        };

        if self.name.trim().is_empty() {
            return Err(invalid("name must not be empty".to_string()));
        }

        for (label, ports) in [("offers", &self.offers), ("accepts", &self.accepts)] {
            let mut seen = std::collections::BTreeSet::new();
            for port in ports.iter() {
                if !seen.insert(port) {
                    return Err(invalid(format!("{label} lists {port} more than once")));
                }
            }
        }

        return Ok(());
    }

    /// Whether this tool accepts the given payload type.
    pub fn accepts(&self, port: &PortType) -> bool {
        return self.accepts.iter().any(|p| p == port);
    }

    /// Whether this tool can produce the given payload type.
    pub fn offers(&self, port: &PortType) -> bool {
        return self.offers.iter().any(|p| p == port);
    }
}

/// A payload type that can travel between tools, such as `luna/image`.
///
/// Ports are how tools connect without knowing about each other: a sender asks the
/// host which tools accept a type, rather than naming a receiver. The transport
/// itself is not built yet; manifests declare their ports first so the vocabulary is
/// settled before anything depends on it.
///
/// The format is `namespace/name`. The `luna/` namespace is reserved for the built-in
/// vocabulary below.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PortType(String);

impl PortType {
    /// A raster image.
    pub const IMAGE: &'static str = "luna/image";
    /// Plain text.
    pub const TEXT: &'static str = "luna/text";
    /// A single colour.
    pub const COLOR: &'static str = "luna/color";
    /// A path to a file on disk.
    pub const FILE_PATH: &'static str = "luna/file-path";
    /// A numeric value.
    pub const NUMBER: &'static str = "luna/number";

    /// Every port type Luna defines itself.
    pub fn well_known() -> [PortType; 5] {
        return [
            PortType(Self::IMAGE.to_string()),
            PortType(Self::TEXT.to_string()),
            PortType(Self::COLOR.to_string()),
            PortType(Self::FILE_PATH.to_string()),
            PortType(Self::NUMBER.to_string()),
        ];
    }

    /// The wire form, such as `"luna/image"`.
    pub fn as_str(&self) -> &str {
        return &self.0;
    }

    /// The namespace half, such as `"luna"`.
    pub fn namespace(&self) -> &str {
        return self.0.split('/').next().unwrap_or("");
    }
}

impl std::fmt::Display for PortType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}", self.0);
    }
}

impl std::str::FromStr for PortType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        let reject = |why: &str| Err(format!("invalid port type {s:?}: {why}"));

        let mut parts = s.split('/');
        let (namespace, name, extra) = (parts.next(), parts.next(), parts.next());

        if extra.is_some() {
            return reject("expected exactly one '/'");
        }

        let (namespace, name) = match (namespace, name) {
            (Some(ns), Some(n)) => (ns, n),
            _ => return reject("expected the form namespace/name"),
        };

        if namespace.is_empty() || name.is_empty() {
            return reject("neither half may be empty");
        }

        let ok = |part: &str| {
            part.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        };

        if !ok(namespace) || !ok(name) {
            return reject("may only contain lowercase letters, digits, hyphen and underscore");
        }

        return Ok(PortType(s.to_string()));
    }
}

impl Serialize for PortType {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        return serializer.serialize_str(&self.0);
    }
}

impl<'de> Deserialize<'de> for PortType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        return raw.parse().map_err(serde::de::Error::custom);
    }
}

/// Serde support for [`luna::Version`], which lives in the published crate and is kept
/// free of a serde dependency.
mod version_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(
        version: &luna::Version,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        return serializer.serialize_str(&version.to_string());
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<luna::Version, D::Error> {
        let raw = String::deserialize(deserializer)?;
        return parse(&raw).map_err(serde::de::Error::custom);
    }

    /// Parses `major.minor.patch`, rejecting anything else.
    fn parse(text: &str) -> Result<luna::Version, String> {
        let parts: Vec<&str> = text.trim().split('.').collect();

        if parts.len() != 3 {
            return Err(format!("expected major.minor.patch, got {text:?}"));
        }

        let number = |part: &str| -> Result<u16, String> {
            return part
                .parse::<u16>()
                .map_err(|_| format!("{part:?} in {text:?} is not a number between 0 and 65535"));
        };

        return Ok(luna::Version::new(
            number(parts[0])?,
            number(parts[1])?,
            number(parts[2])?,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal() -> &'static str {
        return r#"
            id      = "luna.test"
            name    = "Test"
            version = "1.2.3"
        "#;
    }

    #[test]
    fn parses_a_minimal_manifest_with_sensible_defaults() {
        let m = ToolManifest::from_toml(minimal()).unwrap();

        assert_eq!(m.id, "luna.test");
        assert_eq!(m.version, luna::Version::new(1, 2, 3));
        assert_eq!(m.category, "general");
        assert_eq!(m.state_schema, 1);
        assert_eq!(m.background, false);
        assert!(m.offers.is_empty());
        assert!(m.accepts.is_empty());
    }

    #[test]
    fn round_trips_through_toml() {
        let original = ToolManifest::from_toml(
            r#"
            id          = "luna.img"
            name        = "Images"
            version     = "0.3.0"
            description = "Edits images."
            icon        = "icons/img.svg"
            category    = "media"
            background  = true
            offers      = ["luna/image"]
            accepts     = ["luna/image", "luna/file-path"]
            state_schema = 4
            "#,
        )
        .unwrap();

        let text = toml::to_string_pretty(&original).unwrap();
        let parsed = ToolManifest::from_toml(&text).unwrap();

        assert_eq!(parsed, original);
    }

    #[test]
    fn rejects_unknown_fields() {
        let err = ToolManifest::from_toml(
            r#"
            id      = "luna.test"
            name    = "Test"
            version = "1.0.0"
            backgruond = true
            "#,
        );

        assert!(matches!(err, Err(CoreError::InvalidManifest { .. })), "{err:?}");
    }

    #[test]
    fn rejects_malformed_versions() {
        for bad in ["1.2", "1.2.3.4", "v1.2.3", "1.2.x", "", "1.2.99999999"] {
            let text = format!("id = \"luna.test\"\nname = \"T\"\nversion = \"{bad}\"\n");
            assert!(ToolManifest::from_toml(&text).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn rejects_bad_ids_and_empty_names() {
        let bad_id = "id = \"../escape\"\nname = \"T\"\nversion = \"1.0.0\"\n";
        assert!(matches!(
            ToolManifest::from_toml(bad_id),
            Err(CoreError::InvalidToolId { .. })
        ));

        let blank_name = "id = \"luna.t\"\nname = \"   \"\nversion = \"1.0.0\"\n";
        assert!(matches!(
            ToolManifest::from_toml(blank_name),
            Err(CoreError::InvalidManifest { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_ports() {
        let text = r#"
            id      = "luna.test"
            name    = "Test"
            version = "1.0.0"
            accepts = ["luna/image", "luna/image"]
        "#;

        let err = ToolManifest::from_toml(text);
        assert!(matches!(err, Err(CoreError::InvalidManifest { .. })), "{err:?}");
    }

    #[test]
    fn answers_port_queries() {
        let m = ToolManifest::from_toml(
            r#"
            id      = "luna.test"
            name    = "Test"
            version = "1.0.0"
            offers  = ["luna/text"]
            accepts = ["luna/image"]
            "#,
        )
        .unwrap();

        let image: PortType = "luna/image".parse().unwrap();
        let text: PortType = "luna/text".parse().unwrap();

        assert!(m.accepts(&image));
        assert!(!m.accepts(&text));
        assert!(m.offers(&text));
        assert!(!m.offers(&image));
    }

    #[test]
    fn accepts_well_formed_port_types() {
        for good in ["luna/image", "luna/file-path", "third_party/thing", "a/b"] {
            assert!(good.parse::<PortType>().is_ok(), "rejected {good:?}");
        }
    }

    #[test]
    fn rejects_malformed_port_types() {
        for bad in ["", "image", "luna/", "/image", "luna/a/b", "Luna/Image", "luna image"] {
            assert!(bad.parse::<PortType>().is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn well_known_port_types_are_valid_and_namespaced() {
        for port in PortType::well_known() {
            assert_eq!(port.namespace(), "luna");
            assert_eq!(port.as_str().parse::<PortType>().unwrap(), port);
        }
    }
}
