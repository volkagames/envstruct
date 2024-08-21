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

pub mod prelude {
    pub use super::{
        env_json::*, env_map::*, error::*, parse_nested::*, parse_primitive::*, usage::*,
        with_json::*,
    };
    pub use envstruct_derive::*;
}

// re-export

#[cfg(feature = "bytesize")]
pub use bytesize::{self, ByteSize};

#[cfg(feature = "chrono")]
pub use chrono::{self, DateTime, TimeZone, Utc};

#[cfg(feature = "humantime")]
pub use humantime::{self, Duration};

#[cfg(feature = "url")]
pub use url::{self, Url};

#[cfg(feature = "regex")]
pub use regex::{self, Regex};

#[cfg(feature = "serde_json")]
pub use serde_json::{self, Value};
