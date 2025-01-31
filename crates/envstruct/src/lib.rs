#![allow(internal_features)]

mod env_json;
mod env_map;
mod error;
mod parse_nested;
mod parse_primitive;
mod usage;
mod with_json;

pub use error::*;
pub use parse_nested::*;
pub use parse_primitive::*;
pub use usage::*;

pub use envstruct_derive::*;

/// The `prelude` module re-exports common items for easy inclusion.
pub mod prelude {
    pub use super::{
        env_json::*, env_map::*, error::*, parse_nested::*, parse_primitive::*, usage::*,
        with_json::*,
    };
    pub use envstruct_derive::*;
}

// re-export

/// Re-export of the `bytesize` crate if the `bytesize` feature is enabled.
#[cfg(feature = "bytesize")]
pub use bytesize::{self, ByteSize};

/// Re-export of the `chrono` crate if the `chrono` feature is enabled.
#[cfg(feature = "chrono")]
pub use chrono::{self, DateTime, TimeZone, Utc};

/// Re-export of the `humantime` crate if the `humantime` feature is enabled.
#[cfg(feature = "humantime")]
pub use humantime::{self, Duration};

/// Re-export of the `url` crate if the `url` feature is enabled.
#[cfg(feature = "url")]
pub use url::{self, Url};

/// Re-export of the `regex` crate if the `regex` feature is enabled.
#[cfg(feature = "regex")]
pub use regex::{self, Regex};

/// Re-export of the `serde_json` crate if the `serde_json` feature is enabled.
#[cfg(feature = "serde_json")]
pub use serde_json::{self, Value};
