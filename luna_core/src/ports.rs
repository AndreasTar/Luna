//! Handing data from one tool to another.
//!
//! Tools declare typed `offers` and `accepts` in their manifest. The sender never
//! names a receiver: it asks the host which tools accept a type, and the host answers
//! from the registry. A tool that is not installed, or is disabled, simply is not in
//! the list, so nothing breaks and neither tool references the other.
//!
//! ## Large payloads cost a refcount
//!
//! [`PayloadData::Bytes`] holds an `Arc<[u8]>`, so sending a 50 megapixel image
//! between tools copies a pointer. The architecture called for a content-addressed
//! blob store; an `Arc` achieves the same goal without one. Content addressing would
//! add deduplication across sends, which nothing needs yet and which would mean
//! hashing every payload on the way in.
//!
//! ## Delivery is a mailbox, not a callback
//!
//! A send leaves the payload in the target's inbox. The target collects it when it
//! next runs, which may be immediately if its service is up, or when its page opens if
//! it has no service. Calling into the receiver directly would mean a tool's code
//! running at an arbitrary point inside another tool's call stack.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};

use crate::error::{CoreError, Result};
use crate::manifest::PortType;

pub const VERSION: luna::Version = luna::Version::new(0, 1, 0);

/// The most deliveries one tool's inbox will hold.
///
/// A tool that never collects its mail must not grow without bound. Oldest are dropped
/// first, on the grounds that a stale handoff is worth less than a fresh one.
pub const MAX_INBOX: usize = 32;

/// The content of a handoff.
#[derive(Debug, Clone, PartialEq)]
pub enum PayloadData {
    Text(Arc<str>),
    Number(f64),
    /// A colour, as written in a palette file.
    Color(String),
    Path(PathBuf),
    /// Raw bytes, such as an encoded image. Shared rather than copied.
    Bytes(Arc<[u8]>),
}

impl PayloadData {
    /// A short description for logs and for the receiving UI.
    pub fn describe(&self) -> String {
        return match self {
            PayloadData::Text(t) => format!("{} characters of text", t.chars().count()),
            PayloadData::Number(n) => format!("the number {n}"),
            PayloadData::Color(c) => format!("the colour {c}"),
            PayloadData::Path(p) => format!("the file {}", p.display()),
            PayloadData::Bytes(b) => format!("{} bytes", b.len()),
        };
    }
}

/// Something one tool is handing to another.
#[derive(Debug, Clone, PartialEq)]
pub struct Payload {
    /// The declared type, which is what receivers are matched on.
    pub port: PortType,
    pub data: PayloadData,
}

impl Payload {
    pub fn new(port: PortType, data: PayloadData) -> Self {
        return Self { port, data };
    }

    /// Plain text.
    pub fn text(text: impl Into<Arc<str>>) -> Self {
        return Self::new(
            PortType::TEXT.parse().expect("built-in port types are valid"),
            PayloadData::Text(text.into()),
        );
    }

    /// An encoded image.
    pub fn image(bytes: impl Into<Arc<[u8]>>) -> Self {
        return Self::new(
            PortType::IMAGE.parse().expect("built-in port types are valid"),
            PayloadData::Bytes(bytes.into()),
        );
    }

    /// A path to a file on disk.
    pub fn file_path(path: impl Into<PathBuf>) -> Self {
        return Self::new(
            PortType::FILE_PATH.parse().expect("built-in port types are valid"),
            PayloadData::Path(path.into()),
        );
    }
}

/// A payload waiting in a tool's inbox.
#[derive(Debug, Clone, PartialEq)]
pub struct Delivery {
    /// The tool that sent it.
    pub from: String,
    pub payload: Payload,
    pub at: DateTime<Utc>,
}

/// Pending handoffs, keyed by receiving tool.
///
/// Holds no reference to the registry: whether a target exists and accepts a type is
/// checked by [`crate::Host::send_to`] before anything reaches here, which keeps this
/// a plain mailbox.
#[derive(Default)]
pub struct PortBus {
    inboxes: BTreeMap<String, Vec<Delivery>>,
    dropped: usize,
}

impl PortBus {
    pub fn new() -> Self {
        return Self::default();
    }

    /// Leaves a payload in a tool's inbox.
    ///
    /// Drops the oldest delivery if the inbox is full, and counts it. A tool that is
    /// never opened must not be able to grow the process without bound.
    pub fn deliver(&mut self, to: &str, delivery: Delivery) {
        let inbox = self.inboxes.entry(to.to_string()).or_default();

        inbox.push(delivery);

        while inbox.len() > MAX_INBOX {
            inbox.remove(0);
            self.dropped += 1;
        }
    }

    /// Collects everything waiting for a tool, emptying its inbox.
    pub fn take(&mut self, tool_id: &str) -> Vec<Delivery> {
        return self.inboxes.remove(tool_id).unwrap_or_default();
    }

    /// How many deliveries are waiting for a tool.
    pub fn pending(&self, tool_id: &str) -> usize {
        return self.inboxes.get(tool_id).map(|i| i.len()).unwrap_or(0);
    }

    /// Every tool with mail waiting, for a badge or a startup summary.
    pub fn tools_with_mail(&self) -> Vec<&str> {
        return self
            .inboxes
            .iter()
            .filter(|(_, inbox)| !inbox.is_empty())
            .map(|(id, _)| id.as_str())
            .collect();
    }

    /// Discards anything waiting for a tool, for when it is disabled or removed.
    pub fn clear(&mut self, tool_id: &str) {
        self.inboxes.remove(tool_id);
    }

    /// How many deliveries have been dropped for overflowing an inbox.
    pub fn dropped(&self) -> usize {
        return self.dropped;
    }
}

/// A tool that can receive a given payload type, for building a "Send to" menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortTarget {
    pub tool_id: String,
    pub name: String,
}

/// Why a send did not happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendRefusal {
    /// No tool with that id is compiled in.
    UnknownTool,
    /// The tool exists but is turned off.
    ToolDisabled,
    /// The tool does not declare that type in its `accepts`.
    NotAccepted,
    /// A tool cannot hand something to itself.
    SameTool,
}

impl SendRefusal {
    /// What to tell the user, if anything.
    ///
    /// These are not errors in the usual sense. A "Send to" menu is built from the
    /// live list of targets, so the only way to hit one is a race: the target was
    /// disabled between the menu opening and the click.
    pub fn explanation(self) -> &'static str {
        return match self {
            SendRefusal::UnknownTool => "That tool is not installed.",
            SendRefusal::ToolDisabled => "That tool was turned off.",
            SendRefusal::NotAccepted => "That tool does not accept this kind of data.",
            SendRefusal::SameTool => "A tool cannot send to itself.",
        };
    }
}

impl From<SendRefusal> for CoreError {
    fn from(refusal: SendRefusal) -> Self {
        return CoreError::SendRefused {
            reason: refusal.explanation().to_string(),
        };
    }
}

/// Checks a send against the registry, without performing it.
///
/// Separated so the UI can grey out a target for the same reason the send would fail.
pub fn check_send(
    registry: &crate::Registry,
    from: &str,
    to: &str,
    port: &PortType,
) -> std::result::Result<(), SendRefusal> {
    if from == to {
        return Err(SendRefusal::SameTool);
    }

    let Ok(manifest) = registry.manifest(to) else {
        return Err(SendRefusal::UnknownTool);
    };

    if !registry.is_enabled(to).unwrap_or(false) {
        return Err(SendRefusal::ToolDisabled);
    }

    if !manifest.accepts(port) {
        return Err(SendRefusal::NotAccepted);
    }

    return Ok(());
}

/// Every enabled tool that accepts a payload type, excluding the sender.
///
/// This is the "Send to" menu. Built fresh each time it is asked for, so a tool
/// disabled a moment ago is already gone from it.
pub fn targets_for(registry: &crate::Registry, from: &str, port: &PortType) -> Vec<PortTarget> {
    return registry
        .accepting(port)
        .into_iter()
        .filter(|m| m.id != from)
        .map(|m| PortTarget {
            tool_id: m.id.clone(),
            name: m.name.clone(),
        })
        .collect();
}

/// Helper for tools receiving mail, so the common shape is not written out each time.
pub fn describe_delivery(delivery: &Delivery) -> String {
    return format!(
        "{} from {}",
        delivery.payload.data.describe(),
        delivery.from
    );
}

/// Result alias matching the rest of the crate.
pub type SendResult = Result<()>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::ToolManifest;
    use crate::paths::AppPaths;
    use crate::Registry;

    fn port(name: &str) -> PortType {
        return name.parse().unwrap();
    }

    fn tool(id: &str, accepts: &str, offers: &str) -> ToolManifest {
        return ToolManifest::from_toml(&format!(
            "id = \"{id}\"\nname = \"{id}\"\nversion = \"1.0.0\"\n\
             accepts = [{accepts}]\noffers = [{offers}]\n"
        ))
        .unwrap();
    }

    fn registry_with(tools: Vec<ToolManifest>) -> (tempfile::TempDir, Registry) {
        let dir = tempfile::tempdir().unwrap();
        let paths = AppPaths::rooted_at(dir.path()).unwrap();

        let mut registry = Registry::new();
        for manifest in tools {
            let id = manifest.id.clone();
            registry.register(manifest, None).unwrap();
            registry.enable(&id, &paths).unwrap();
        }

        return (dir, registry);
    }

    fn delivery(from: &str, payload: Payload) -> Delivery {
        return Delivery {
            from: from.to_string(),
            payload,
            at: Utc::now(),
        };
    }

    #[test]
    fn the_send_to_menu_lists_tools_that_accept_the_type() {
        let (_dir, registry) = registry_with(vec![
            tool("luna.editor", "\"luna/image\"", "\"luna/image\""),
            tool("luna.ascii", "\"luna/image\"", "\"luna/text\""),
            tool("luna.notes", "\"luna/text\"", ""),
        ]);

        let targets = targets_for(&registry, "luna.editor", &port("luna/image"));

        let ids: Vec<&str> = targets.iter().map(|t| t.tool_id.as_str()).collect();

        assert_eq!(ids, vec!["luna.ascii"]);
    }

    #[test]
    fn the_sender_is_never_a_target_for_its_own_send() {
        let (_dir, registry) = registry_with(vec![tool(
            "luna.editor",
            "\"luna/image\"",
            "\"luna/image\"",
        )]);

        // The editor both offers and accepts images, but must not offer to send to
        // itself.
        assert!(targets_for(&registry, "luna.editor", &port("luna/image")).is_empty());
    }

    #[test]
    fn a_disabled_tool_disappears_from_the_menu() {
        let (dir, mut registry) = registry_with(vec![
            tool("luna.editor", "", "\"luna/image\""),
            tool("luna.ascii", "\"luna/image\"", ""),
        ]);

        assert_eq!(targets_for(&registry, "luna.editor", &port("luna/image")).len(), 1);

        registry.disable("luna.ascii").unwrap();

        assert!(
            targets_for(&registry, "luna.editor", &port("luna/image")).is_empty(),
            "a disabled tool must not be offered"
        );

        // And enabling it brings it back, with no other coordination.
        let paths = AppPaths::rooted_at(dir.path()).unwrap();
        registry.enable("luna.ascii", &paths).unwrap();

        assert_eq!(targets_for(&registry, "luna.editor", &port("luna/image")).len(), 1);
    }

    #[test]
    fn an_absent_tool_simply_is_not_offered() {
        // The motivating case: the image editor offers to send somewhere only if
        // something that accepts images is installed.
        let (_dir, registry) = registry_with(vec![tool("luna.editor", "", "\"luna/image\"")]);

        assert!(targets_for(&registry, "luna.editor", &port("luna/image")).is_empty());
    }

    #[test]
    fn checking_a_send_names_the_reason_it_cannot_happen() {
        let (_dir, mut registry) = registry_with(vec![
            tool("luna.editor", "", "\"luna/image\""),
            tool("luna.ascii", "\"luna/image\"", ""),
            tool("luna.notes", "\"luna/text\"", ""),
        ]);

        let image = port("luna/image");

        assert_eq!(check_send(&registry, "luna.editor", "luna.ascii", &image), Ok(()));

        assert_eq!(
            check_send(&registry, "luna.editor", "luna.notes", &image),
            Err(SendRefusal::NotAccepted)
        );

        assert_eq!(
            check_send(&registry, "luna.editor", "luna.nope", &image),
            Err(SendRefusal::UnknownTool)
        );

        assert_eq!(
            check_send(&registry, "luna.editor", "luna.editor", &image),
            Err(SendRefusal::SameTool)
        );

        registry.disable("luna.ascii").unwrap();
        assert_eq!(
            check_send(&registry, "luna.editor", "luna.ascii", &image),
            Err(SendRefusal::ToolDisabled)
        );
    }

    #[test]
    fn every_refusal_explains_itself() {
        for refusal in [
            SendRefusal::UnknownTool,
            SendRefusal::ToolDisabled,
            SendRefusal::NotAccepted,
            SendRefusal::SameTool,
        ] {
            assert!(!refusal.explanation().is_empty());
        }
    }

    #[test]
    fn a_delivery_waits_until_it_is_collected() {
        let mut bus = PortBus::new();

        assert_eq!(bus.pending("luna.ascii"), 0);

        bus.deliver("luna.ascii", delivery("luna.editor", Payload::text("hello")));

        assert_eq!(bus.pending("luna.ascii"), 1);

        let taken = bus.take("luna.ascii");

        assert_eq!(taken.len(), 1);
        assert_eq!(taken[0].from, "luna.editor");
        assert_eq!(bus.pending("luna.ascii"), 0, "collecting empties the inbox");
    }

    #[test]
    fn inboxes_are_per_tool() {
        let mut bus = PortBus::new();

        bus.deliver("luna.a", delivery("luna.x", Payload::text("for a")));
        bus.deliver("luna.b", delivery("luna.x", Payload::text("for b")));

        assert_eq!(bus.pending("luna.a"), 1);
        assert_eq!(bus.pending("luna.b"), 1);

        bus.take("luna.a");

        assert_eq!(bus.pending("luna.b"), 1, "collecting one must not empty another");
    }

    #[test]
    fn a_never_collected_inbox_does_not_grow_without_bound() {
        let mut bus = PortBus::new();

        for i in 0..MAX_INBOX + 10 {
            bus.deliver(
                "luna.hoarder",
                delivery("luna.x", Payload::text(format!("{i}"))),
            );
        }

        assert_eq!(bus.pending("luna.hoarder"), MAX_INBOX);
        assert_eq!(bus.dropped(), 10);

        // The oldest went, so what remains is the most recent.
        let taken = bus.take("luna.hoarder");
        let first = match &taken[0].payload.data {
            PayloadData::Text(t) => t.to_string(),
            other => panic!("unexpected payload: {other:?}"),
        };

        assert_eq!(first, "10", "the oldest deliveries are the ones dropped");
    }

    #[test]
    fn clearing_discards_a_tools_mail() {
        let mut bus = PortBus::new();

        bus.deliver("luna.a", delivery("luna.x", Payload::text("hello")));
        bus.clear("luna.a");

        assert_eq!(bus.pending("luna.a"), 0);
    }

    #[test]
    fn tools_with_mail_are_reportable() {
        let mut bus = PortBus::new();

        bus.deliver("luna.a", delivery("luna.x", Payload::text("hello")));
        bus.deliver("luna.b", delivery("luna.x", Payload::text("hello")));

        let mut waiting = bus.tools_with_mail();
        waiting.sort_unstable();

        assert_eq!(waiting, vec!["luna.a", "luna.b"]);
    }

    #[test]
    fn a_large_payload_is_shared_rather_than_copied() {
        let bytes: Arc<[u8]> = vec![0u8; 4 * 1024 * 1024].into();
        let payload = Payload::image(bytes.clone());

        // Cloning the payload for delivery must not duplicate the buffer.
        let cloned = payload.clone();

        let (PayloadData::Bytes(a), PayloadData::Bytes(b)) = (&payload.data, &cloned.data) else {
            panic!("expected bytes");
        };

        assert!(Arc::ptr_eq(a, b), "the buffer should be shared, not copied");
        assert_eq!(a.len(), 4 * 1024 * 1024);
    }

    #[test]
    fn payloads_describe_themselves() {
        assert!(Payload::text("hello").data.describe().contains('5'));
        assert!(Payload::image(vec![0u8; 10]).data.describe().contains("10"));
        assert!(Payload::file_path("/tmp/x.png").data.describe().contains("x.png"));
    }

    #[test]
    fn the_built_in_payload_constructors_use_valid_port_types() {
        assert_eq!(Payload::text("x").port.as_str(), PortType::TEXT);
        assert_eq!(Payload::image(vec![]).port.as_str(), PortType::IMAGE);
        assert_eq!(Payload::file_path("x").port.as_str(), PortType::FILE_PATH);
    }
}
