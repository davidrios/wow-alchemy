pub mod chunks;
pub mod error;
pub mod header;
pub mod md20;
pub mod model;
pub mod phys;
pub mod skin;
pub mod version;

pub use error::{M2Error, Result};
pub use md20::MD20Model;
pub use model::M2Model;
pub use phys::PhysFile;
pub use skin::Skin;
pub use version::MD20Version;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
