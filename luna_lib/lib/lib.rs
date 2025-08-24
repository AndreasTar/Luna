

// TODO add //! type doc to this
// TODO fix readme
// TODO fix license on cargo.io
#[cfg(feature = "number_converter")]
pub mod number_converter;

#[cfg(feature = "img_manipulator")]
pub mod img_manipulator;

#[cfg(feature = "color_format_converter")]
pub mod color_format_converter;



/// A simple struct representing a semantic version (major.minor.patch), used in the library by each individual tool.
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
}

impl Version {
    pub const fn new(major: u16, minor: u16, patch: u16) -> Self {
        Version { major, minor, patch }
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "{}.{}.{}", self.major, self.minor, self.patch);
    }
}