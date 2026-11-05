//! The archive's own files: `(listfile)` and `(attributes)`.

mod attributes;
mod listfile;

pub use attributes::{AttributeFlags, Attributes, AttributesHeader};
pub use listfile::{build_listfile, parse_listfile};

/// Name of the file listing the archive's files.
pub const LISTFILE: &str = "(listfile)";
/// Name of the file holding checksums and times of the archive's files.
pub const ATTRIBUTES: &str = "(attributes)";
