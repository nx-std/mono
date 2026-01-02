//! Builder patterns for constructing Nintendo Switch executable formats.
//!
//! This module provides builders for creating NRO, NSO, NACP, and RomFS files.
//! All builders follow a consistent pattern:
//!
//! 1. Create a new builder with `Builder::new()`
//! 2. Configure it with chainable setter methods
//! 3. Call `.build()` to generate the final byte buffer
//!
//! # Example
//!
//! ```no_run
//! use nx_object::write::NacpBuilder;
//!
//! let nacp = NacpBuilder::new()
//!     .name("My Homebrew")
//!     .author("Developer")
//!     .version("1.0.0")
//!     .build()
//!     .expect("failed to build NACP");
//! ```

pub mod nacp;
pub mod nro;
pub mod nso;
pub mod romfs;

pub use nacp::NacpBuilder;
pub use nro::NroBuilder;
pub use nso::NsoBuilder;
pub use romfs::RomFsBuilder;
