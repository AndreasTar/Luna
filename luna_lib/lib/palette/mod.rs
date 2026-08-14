//! Semantic colour palettes, with per-consumer overrides and contrast checking.
//!
//! A palette maps a fixed set of **roles** to colours. Consumers ask for
//! [`Role::Warning`], never for "orange", which is what lets one part of an
//! application restyle a single role without knowing anything about the palette in
//! use.
//!
//! Colours resolve through three layers, most specific first:
//!
//! ```text
//! per-role override  ->  consumer palette  ->  application palette
//! ```
//!
//! So an application can run one palette while a single page runs another with its
//! `warning` changed to yellow, and nothing else is affected.
//!
//! This module is pure: no filesystem, no serialisation, no clock. Loading palettes
//! from disk belongs to whatever is embedding it.
//!
//! # Examples
//!
//! ```
//! # use luna::palette::{Color, Palette, Role, Overrides};
//! let mut app = Palette::new("dark", "Dark");
//! app.set(Role::Warning, Color::from_hex("#ff8000").unwrap());
//! app.set(Role::Background, Color::from_hex("#191919").unwrap());
//!
//! // A page that wants a different warning colour, and nothing else changed.
//! let mut overrides = Overrides::new();
//! overrides.set(Role::Warning, Color::from_hex("#ffdd00").unwrap());
//!
//! let resolved = overrides.resolve(&app, None);
//!
//! assert_eq!(resolved.get(Role::Warning).to_hex(), "#ffdd00");
//! assert_eq!(resolved.get(Role::Background).to_hex(), "#191919");
//! ```

use std::collections::BTreeMap;

pub const VERSION: crate::Version = crate::Version::new(1, 0, 0);

/// Declares the role set once, and derives the enum, the name table and the lookups
/// from it, so those cannot drift apart as roles are added.
macro_rules! roles {
    ($($variant:ident => $name:literal),+ $(,)?) => {
        /// A semantic slot in a palette.
        ///
        /// Consumers reference roles rather than colours, which is what makes
        /// overrides and alternative palettes possible without touching them.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum Role {
            $(
                #[doc = concat!("The `", $name, "` role.")]
                $variant
            ),+
        }

        impl Role {
            /// Every role, in declaration order.
            pub const ALL: &'static [Role] = &[$(Role::$variant),+];

            /// How many roles a palette must define.
            pub const COUNT: usize = [$(Role::$variant),+].len();

            /// The role's name as it appears in a palette file.
            pub fn as_str(self) -> &'static str {
                return match self {
                    $(Role::$variant => $name),+
                };
            }
        }

        impl std::str::FromStr for Role {
            type Err = UnknownRole;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                return match s {
                    $($name => Ok(Role::$variant),)+
                    other => Err(UnknownRole(other.to_string())),
                };
            }
        }
    };
}

roles! {
    Primary               => "primary",
    Secondary             => "secondary",
    Tertiary              => "tertiary",
    Quaternary            => "quaternary",

    Text                  => "text",
    TextSecondary         => "text_secondary",
    TextTertiary          => "text_tertiary",
    TextQuaternary        => "text_quaternary",

    Background            => "background",
    BackgroundSecondary   => "background_secondary",
    BackgroundTertiary    => "background_tertiary",
    BackgroundQuaternary  => "background_quaternary",

    Border                => "border",
    BorderSecondary       => "border_secondary",
    BorderTertiary        => "border_tertiary",
    BorderQuaternary      => "border_quaternary",

    Success               => "success",
    Warning               => "warning",
    Error                 => "error",
    Info                  => "info",
    Danger                => "danger",

    Inactive              => "inactive",
    Disabled              => "disabled",

    Highlight             => "highlight",
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}", self.as_str());
    }
}

/// A role name that is not part of the role set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownRole(pub String);

impl std::fmt::Display for UnknownRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{:?} is not a known colour role", self.0);
    }
}

impl std::error::Error for UnknownRole {}

/// An 8-bit RGBA colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        return Self { r, g, b, a };
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        return Self { r, g, b, a: 255 };
    }

    pub const BLACK: Self = Self::rgb(0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);

    /// Parses `#rgb`, `#rrggbb` or `#rrggbbaa`, with or without the leading `#`.
    ///
    /// ## Examples
    /// ```
    /// # use luna::palette::Color;
    /// assert_eq!(Color::from_hex("#ff8000"), Ok(Color::rgb(255, 128, 0)));
    /// assert_eq!(Color::from_hex("f80"),     Ok(Color::rgb(255, 136, 0)));
    /// assert_eq!(Color::from_hex("#ff800080"), Ok(Color::new(255, 128, 0, 128)));
    /// assert!(Color::from_hex("#xyzxyz").is_err());
    /// ```
    pub fn from_hex(text: &str) -> Result<Self, BadColor> {
        let hex = text.trim().strip_prefix('#').unwrap_or(text.trim());

        let bad = || BadColor(text.to_string());

        let pair = |i: usize| -> Result<u8, BadColor> {
            return u8::from_str_radix(&hex[i..i + 2], 16).map_err(|_| bad());
        };

        // The short form doubles each digit, so #f80 and #ff8800 are the same colour.
        let single = |i: usize| -> Result<u8, BadColor> {
            let v = u8::from_str_radix(&hex[i..i + 1], 16).map_err(|_| bad())?;
            return Ok(v * 17);
        };

        return match hex.len() {
            3 => Ok(Self::rgb(single(0)?, single(1)?, single(2)?)),
            6 => Ok(Self::rgb(pair(0)?, pair(2)?, pair(4)?)),
            8 => Ok(Self::new(pair(0)?, pair(2)?, pair(4)?, pair(6)?)),
            _ => Err(bad()),
        };
    }

    /// Renders as `#rrggbb`, or `#rrggbbaa` when not fully opaque.
    pub fn to_hex(self) -> String {
        if self.a == 255 {
            return format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b);
        }

        return format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a);
    }

    /// Relative luminance as defined by WCAG 2.1, in `0.0..=1.0`.
    ///
    /// Alpha is ignored: luminance is a property of the colour itself, and a
    /// translucent colour's apparent luminance depends on what is behind it.
    pub fn relative_luminance(self) -> f32 {
        let channel = |v: u8| -> f32 {
            let v = v as f32 / 255.0;
            return if v <= 0.04045 {
                v / 12.92
            } else {
                ((v + 0.055) / 1.055).powf(2.4)
            };
        };

        return 0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b);
    }

    /// WCAG contrast ratio against another colour, from `1.0` to `21.0`.
    ///
    /// Symmetric: the order of the two colours does not matter.
    ///
    /// ## Examples
    /// ```
    /// # use luna::palette::Color;
    /// let ratio = Color::WHITE.contrast_ratio(Color::BLACK);
    /// assert!((ratio - 21.0).abs() < 0.01);
    ///
    /// assert_eq!(Color::WHITE.contrast_ratio(Color::WHITE), 1.0);
    /// ```
    pub fn contrast_ratio(self, other: Self) -> f32 {
        let a = self.relative_luminance();
        let b = other.relative_luminance();

        let (lighter, darker) = if a >= b { (a, b) } else { (b, a) };

        return (lighter + 0.05) / (darker + 0.05);
    }

    /// How readable this colour is on `background`.
    pub fn readability_on(self, background: Self) -> Readability {
        return Readability::for_ratio(self.contrast_ratio(background));
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}", self.to_hex());
    }
}

/// A colour string that could not be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BadColor(pub String);

impl std::fmt::Display for BadColor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(
            f,
            "{:?} is not a colour. Expected #rgb, #rrggbb or #rrggbbaa.",
            self.0
        );
    }
}

impl std::error::Error for BadColor {}

/// WCAG readability bands for a contrast ratio.
///
/// Used to warn when a colour override makes text hard to read, rather than to forbid
/// it: the user is entitled to a low-contrast theme if they want one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Readability {
    /// Below 3:1. Hard to read at any size.
    Fail,
    /// At least 3:1. Acceptable for large or bold text only.
    LargeTextOnly,
    /// At least 4.5:1. The usual target for body text.
    Good,
    /// At least 7:1.
    Excellent,
}

impl Readability {
    pub fn for_ratio(ratio: f32) -> Self {
        return if ratio >= 7.0 {
            Readability::Excellent
        } else if ratio >= 4.5 {
            Readability::Good
        } else if ratio >= 3.0 {
            Readability::LargeTextOnly
        } else {
            Readability::Fail
        };
    }

    /// Whether normal body text at this ratio is comfortably readable.
    pub fn is_readable(self) -> bool {
        return self >= Readability::Good;
    }
}

/// A named set of colours, one per [`Role`].
///
/// Built incrementally, so a palette under construction can be checked for
/// completeness with [`Palette::missing_roles`] before use.
#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    /// Stable identifier, referenced by settings.
    pub id: String,
    /// Human-readable name for a picker.
    pub name: String,
    /// Optional one-line description.
    pub description: String,
    /// Optional author credit.
    pub author: String,
    /// Whether this palette reads as light or dark.
    ///
    /// A grouping hint only. Light and dark are separate palettes rather than two
    /// variants of one, so this does not select anything.
    pub appearance: Appearance,

    colors: BTreeMap<Role, Color>,
}

/// Whether a palette reads as light or dark. A grouping hint, not a mode switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    Light,
    Dark,
}

impl Appearance {
    pub fn as_str(self) -> &'static str {
        return match self {
            Appearance::Light => "light",
            Appearance::Dark => "dark",
        };
    }
}

impl std::str::FromStr for Appearance {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        return match s.trim().to_ascii_lowercase().as_str() {
            "light" => Ok(Appearance::Light),
            "dark" => Ok(Appearance::Dark),
            other => Err(format!("expected \"light\" or \"dark\", got {other:?}")),
        };
    }
}

impl std::fmt::Display for Appearance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}", self.as_str());
    }
}

impl Palette {
    /// An empty palette. Every role must be set before it is complete.
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        return Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            author: String::new(),
            appearance: Appearance::Dark,
            colors: BTreeMap::new(),
        };
    }

    pub fn set(&mut self, role: Role, color: Color) -> &mut Self {
        self.colors.insert(role, color);
        return self;
    }

    /// The colour for a role, if this palette defines it.
    pub fn try_get(&self, role: Role) -> Option<Color> {
        return self.colors.get(&role).copied();
    }

    /// The colour for a role, falling back to magenta if it is missing.
    ///
    /// The fallback is deliberately loud. A complete palette never hits it, and an
    /// incomplete one should be obvious on screen rather than subtly wrong.
    pub fn get(&self, role: Role) -> Color {
        return self.try_get(role).unwrap_or(Color::rgb(255, 0, 255));
    }

    /// Roles this palette has not defined, in declaration order.
    pub fn missing_roles(&self) -> Vec<Role> {
        return Role::ALL
            .iter()
            .copied()
            .filter(|role| !self.colors.contains_key(role))
            .collect();
    }

    /// Whether every role is defined.
    pub fn is_complete(&self) -> bool {
        return self.colors.len() == Role::COUNT;
    }

    /// Roles whose colour is hard to read against this palette's own background.
    ///
    /// Advisory. Returned so a picker or editor can warn, not to reject the palette.
    pub fn readability_warnings(&self) -> Vec<(Role, Readability)> {
        let background = match self.try_get(Role::Background) {
            Some(bg) => bg,
            None => return Vec::new(),
        };

        // Only roles that are actually rendered as text or as a signal on the
        // background. Warning that a background contrasts poorly with itself would be
        // noise.
        const CHECKED: &[Role] = &[
            Role::Text,
            Role::TextSecondary,
            Role::TextTertiary,
            Role::TextQuaternary,
            Role::Success,
            Role::Warning,
            Role::Error,
            Role::Info,
            Role::Danger,
            Role::Highlight,
        ];

        return CHECKED
            .iter()
            .filter_map(|&role| {
                let readability = self.get(role).readability_on(background);
                return if readability.is_readable() {
                    None
                } else {
                    Some((role, readability))
                };
            })
            .collect();
    }
}

/// One consumer's deviations from the application palette.
///
/// Holds an optional whole-palette substitution and any individual role overrides on
/// top of it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Overrides {
    colors: BTreeMap<Role, Color>,
}

impl Overrides {
    pub fn new() -> Self {
        return Self::default();
    }

    pub fn set(&mut self, role: Role, color: Color) -> &mut Self {
        self.colors.insert(role, color);
        return self;
    }

    pub fn clear(&mut self, role: Role) -> &mut Self {
        self.colors.remove(&role);
        return self;
    }

    pub fn get(&self, role: Role) -> Option<Color> {
        return self.colors.get(&role).copied();
    }

    pub fn is_empty(&self) -> bool {
        return self.colors.is_empty();
    }

    /// Every override, in role order.
    pub fn iter(&self) -> impl Iterator<Item = (Role, Color)> + '_ {
        return self.colors.iter().map(|(&role, &color)| (role, color));
    }

    /// Resolves one role through the three layers.
    ///
    /// Most specific first: this override, then `consumer` if there is one, then
    /// `app`.
    pub fn resolve_role(&self, role: Role, app: &Palette, consumer: Option<&Palette>) -> Color {
        if let Some(color) = self.get(role) {
            return color;
        }

        if let Some(color) = consumer.and_then(|p| p.try_get(role)) {
            return color;
        }

        return app.get(role);
    }

    /// Resolves every role into a flat palette.
    ///
    /// The result carries no notion of where each colour came from, which is the
    /// point: consumers read a finished palette and never learn that overrides exist.
    pub fn resolve(&self, app: &Palette, consumer: Option<&Palette>) -> Resolved {
        let mut colors = BTreeMap::new();

        for &role in Role::ALL {
            colors.insert(role, self.resolve_role(role, app, consumer));
        }

        return Resolved { colors };
    }
}

/// A fully resolved palette: every role has exactly one colour.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    colors: BTreeMap<Role, Color>,
}

impl Resolved {
    pub fn get(&self, role: Role) -> Color {
        // Built by resolving every role, so this cannot miss.
        return self.colors.get(&role).copied().unwrap_or(Color::rgb(255, 0, 255));
    }

    /// Every role and its colour, in role order.
    pub fn iter(&self) -> impl Iterator<Item = (Role, Color)> + '_ {
        return self.colors.iter().map(|(&role, &color)| (role, color));
    }

    /// Roles that are hard to read against the resolved background.
    pub fn readability_warnings(&self) -> Vec<(Role, Readability)> {
        let background = self.get(Role::Background);

        return Role::ALL
            .iter()
            .copied()
            .filter(|&role| {
                matches!(
                    role,
                    Role::Text
                        | Role::TextSecondary
                        | Role::TextTertiary
                        | Role::TextQuaternary
                        | Role::Success
                        | Role::Warning
                        | Role::Error
                        | Role::Info
                        | Role::Danger
                        | Role::Highlight
                )
            })
            .filter_map(|role| {
                let readability = self.get(role).readability_on(background);
                return if readability.is_readable() {
                    None
                } else {
                    Some((role, readability))
                };
            })
            .collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn complete(id: &str, fill: Color) -> Palette {
        let mut p = Palette::new(id, id);
        for &role in Role::ALL {
            p.set(role, fill);
        }
        return p;
    }

    #[test]
    fn role_names_round_trip() {
        for &role in Role::ALL {
            assert_eq!(Role::from_str(role.as_str()), Ok(role), "{role}");
        }
    }

    #[test]
    fn role_names_are_unique() {
        let mut names: Vec<&str> = Role::ALL.iter().map(|r| r.as_str()).collect();
        let count = names.len();
        names.sort_unstable();
        names.dedup();

        assert_eq!(names.len(), count, "two roles share a name");
        assert_eq!(count, Role::COUNT);
    }

    #[test]
    fn unknown_role_names_are_rejected() {
        assert!(Role::from_str("chartreuse").is_err());
        assert!(Role::from_str("").is_err());
        assert!(Role::from_str("Primary").is_err(), "role names are lowercase");
    }

    #[test]
    fn parses_every_hex_form() {
        assert_eq!(Color::from_hex("#ff8000"), Ok(Color::rgb(255, 128, 0)));
        assert_eq!(Color::from_hex("ff8000"), Ok(Color::rgb(255, 128, 0)));
        assert_eq!(Color::from_hex("  #ff8000  "), Ok(Color::rgb(255, 128, 0)));
        assert_eq!(Color::from_hex("#FF8000"), Ok(Color::rgb(255, 128, 0)));
        assert_eq!(Color::from_hex("#f80"), Ok(Color::rgb(255, 136, 0)));
        assert_eq!(Color::from_hex("#ff800080"), Ok(Color::new(255, 128, 0, 128)));
    }

    #[test]
    fn rejects_malformed_colors() {
        for bad in ["", "#", "#ff", "#fffff", "#xyzxyz", "not a colour", "#ff80000"] {
            assert!(Color::from_hex(bad).is_err(), "accepted {bad:?}");
        }
    }

    #[test]
    fn hex_round_trips_including_alpha() {
        let opaque = Color::rgb(18, 52, 86);
        assert_eq!(opaque.to_hex(), "#123456");
        assert_eq!(Color::from_hex(&opaque.to_hex()), Ok(opaque));

        let translucent = Color::new(18, 52, 86, 128);
        assert_eq!(translucent.to_hex(), "#12345680");
        assert_eq!(Color::from_hex(&translucent.to_hex()), Ok(translucent));
    }

    #[test]
    fn contrast_matches_the_wcag_extremes() {
        let ratio = Color::WHITE.contrast_ratio(Color::BLACK);
        assert!((ratio - 21.0).abs() < 0.01, "got {ratio}");

        assert_eq!(Color::WHITE.contrast_ratio(Color::WHITE), 1.0);

        // Symmetric.
        assert_eq!(
            Color::WHITE.contrast_ratio(Color::BLACK),
            Color::BLACK.contrast_ratio(Color::WHITE)
        );
    }

    #[test]
    fn readability_bands_line_up_with_the_thresholds() {
        assert_eq!(Readability::for_ratio(21.0), Readability::Excellent);
        assert_eq!(Readability::for_ratio(7.0), Readability::Excellent);
        assert_eq!(Readability::for_ratio(4.5), Readability::Good);
        assert_eq!(Readability::for_ratio(3.0), Readability::LargeTextOnly);
        assert_eq!(Readability::for_ratio(1.0), Readability::Fail);

        assert!(Readability::Good.is_readable());
        assert!(!Readability::LargeTextOnly.is_readable());
    }

    #[test]
    fn a_palette_knows_which_roles_it_is_missing() {
        let mut p = Palette::new("partial", "Partial");
        p.set(Role::Text, Color::WHITE);

        assert!(!p.is_complete());

        let missing = p.missing_roles();
        assert_eq!(missing.len(), Role::COUNT - 1);
        assert!(!missing.contains(&Role::Text));
        assert!(missing.contains(&Role::Background));
    }

    #[test]
    fn a_filled_palette_is_complete() {
        let p = complete("full", Color::WHITE);

        assert!(p.is_complete());
        assert!(p.missing_roles().is_empty());
    }

    #[test]
    fn a_missing_role_returns_something_obviously_wrong() {
        let p = Palette::new("empty", "Empty");

        // Loud rather than subtle: an incomplete palette should be visible at a glance.
        assert_eq!(p.get(Role::Text), Color::rgb(255, 0, 255));
        assert_eq!(p.try_get(Role::Text), None);
    }

    #[test]
    fn resolution_prefers_the_override_then_the_consumer_then_the_app() {
        let app = complete("app", Color::rgb(1, 1, 1));
        let consumer = complete("consumer", Color::rgb(2, 2, 2));

        let mut overrides = Overrides::new();
        overrides.set(Role::Warning, Color::rgb(3, 3, 3));

        // Override wins over everything.
        assert_eq!(
            overrides.resolve_role(Role::Warning, &app, Some(&consumer)),
            Color::rgb(3, 3, 3)
        );

        // Consumer palette wins where there is no override.
        assert_eq!(
            overrides.resolve_role(Role::Error, &app, Some(&consumer)),
            Color::rgb(2, 2, 2)
        );

        // App palette is the floor.
        assert_eq!(
            overrides.resolve_role(Role::Error, &app, None),
            Color::rgb(1, 1, 1)
        );
    }

    #[test]
    fn resolving_covers_every_role() {
        let app = complete("app", Color::rgb(9, 9, 9));
        let resolved = Overrides::new().resolve(&app, None);

        assert_eq!(resolved.iter().count(), Role::COUNT);

        for &role in Role::ALL {
            assert_eq!(resolved.get(role), Color::rgb(9, 9, 9), "{role}");
        }
    }

    #[test]
    fn an_override_changes_only_its_own_role() {
        let mut app = complete("app", Color::rgb(1, 1, 1));
        app.set(Role::Background, Color::rgb(5, 5, 5));

        let mut overrides = Overrides::new();
        overrides.set(Role::Warning, Color::rgb(7, 7, 7));

        let resolved = overrides.resolve(&app, None);

        assert_eq!(resolved.get(Role::Warning), Color::rgb(7, 7, 7));
        assert_eq!(resolved.get(Role::Background), Color::rgb(5, 5, 5));
        assert_eq!(resolved.get(Role::Error), Color::rgb(1, 1, 1));
    }

    #[test]
    fn a_consumer_palette_falls_back_for_roles_it_does_not_define() {
        let app = complete("app", Color::rgb(1, 1, 1));

        // A partial palette, which a hand-written file could easily be.
        let mut consumer = Palette::new("partial", "Partial");
        consumer.set(Role::Warning, Color::rgb(2, 2, 2));

        let overrides = Overrides::new();

        assert_eq!(
            overrides.resolve_role(Role::Warning, &app, Some(&consumer)),
            Color::rgb(2, 2, 2)
        );
        assert_eq!(
            overrides.resolve_role(Role::Text, &app, Some(&consumer)),
            Color::rgb(1, 1, 1),
            "an undefined role must fall through, not return the loud fallback"
        );
    }

    #[test]
    fn clearing_an_override_restores_the_layer_below() {
        let app = complete("app", Color::rgb(1, 1, 1));

        let mut overrides = Overrides::new();
        overrides.set(Role::Warning, Color::rgb(9, 9, 9));
        assert_eq!(overrides.resolve_role(Role::Warning, &app, None), Color::rgb(9, 9, 9));

        overrides.clear(Role::Warning);
        assert!(overrides.is_empty());
        assert_eq!(overrides.resolve_role(Role::Warning, &app, None), Color::rgb(1, 1, 1));
    }

    #[test]
    fn unreadable_text_is_flagged_against_the_background() {
        let mut p = complete("low", Color::WHITE);
        p.set(Role::Background, Color::WHITE);
        p.set(Role::Text, Color::rgb(250, 250, 250));

        let warnings = p.readability_warnings();

        assert!(
            warnings.iter().any(|(role, _)| *role == Role::Text),
            "near-white text on white should warn: {warnings:?}"
        );
    }

    #[test]
    fn readable_text_is_not_flagged() {
        let mut p = complete("high", Color::BLACK);
        p.set(Role::Background, Color::BLACK);
        p.set(Role::Text, Color::WHITE);

        let flagged: Vec<Role> = p
            .readability_warnings()
            .into_iter()
            .map(|(role, _)| role)
            .collect();

        assert!(!flagged.contains(&Role::Text), "white on black should pass");
    }

    #[test]
    fn backgrounds_are_not_checked_against_themselves() {
        let p = complete("flat", Color::rgb(40, 40, 40));

        let flagged: Vec<Role> = p
            .readability_warnings()
            .into_iter()
            .map(|(role, _)| role)
            .collect();

        assert!(!flagged.contains(&Role::Background));
        assert!(!flagged.contains(&Role::BackgroundSecondary));
        assert!(!flagged.contains(&Role::Border));
    }

    #[test]
    fn appearance_round_trips() {
        for appearance in [Appearance::Light, Appearance::Dark] {
            assert_eq!(Appearance::from_str(appearance.as_str()), Ok(appearance));
        }

        assert_eq!(Appearance::from_str("DARK"), Ok(Appearance::Dark));
        assert!(Appearance::from_str("twilight").is_err());
    }
}
