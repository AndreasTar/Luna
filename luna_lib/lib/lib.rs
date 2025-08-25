//! # Luna Library
//! `Luna` is a collection of small, useful modules for various tasks, conversions and generally useful computations.
//! Each module is guarded behing a feature flag, so you can choose which ones to include in your project.
//! 
//! Developed in parallel with the [Luna CLI](https://github.com/AndreasTar/Luna) project. The link
//! houses the source code for both the CLI and this library.
//! 
//! For more information, please read the [README](https://github.com/AndreasTar/Luna/blob/main/luna_lib/README.md), 
//! but here is a quick rundown of some details:
//! 
//! ## Versioning
//! Each module has its own version, following semantic versioning. The version can be found in the documentation of each module,
//! or by accessing the `VERSION` constant in each module.
//! 
//! The library itself also uses semantic versioning, but the version is not tied to the versions of the individual modules, and follows
//! a different system.
//! Instead, 
//! - the **major** version increases when the library is in a state that contains the modules that make it 'feel' like a complete version.
//! - the **minor** version defines the amount of modules that are complete and functional. 
//! This does not mean that they will never receive updates, if any, but their current state is sufficient for their purposes.
//! - the **patch** version is split among all modules and is incremented every time there is an update, however small or large.
//! 
//! ## Copyright and License
//! This license of this library can be found in the [LICENSE](https://github.com/AndreasTar/Luna/blob/main/LICENSE) file in the 
//! root directory of the repository, as a `PolyForm Noncommercial License 1.0.0`. For a commercial license, please contact the author.
//! 
//! Copyright (c) Andreas Tarasidis. All rights reserved.
//! 
//! For contacting the author, please refer to the links found in the [GitHub profile](https://github.com/AndreasTar).



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