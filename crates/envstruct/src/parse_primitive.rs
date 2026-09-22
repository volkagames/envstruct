use crate::*;
use pastey::paste;
use std::collections::{BTreeMap, HashMap, HashSet};

/// A trait for parsing environment variables into primitive types.
pub trait EnvParsePrimitive {
    /// Parses a string value into the implementing type.
    ///
    /// # Arguments
    ///
    /// * `val` - A string slice that holds the value to be parsed.
    ///
    /// # Returns
    ///
    /// * `Result<Self, BoxError>` - The parsed value or an error.
    fn parse(val: &str) -> Result<Self, BoxError>
    where
        Self: Sized;

    /// Parses an environment variable into the implementing type.
    ///
    /// # Arguments
    ///
    /// * `var_name` - The name of the environment variable.
    /// * `default` - An optional default value if the environment variable is not set.
    ///
    /// # Returns
    ///
    /// * `Result<Self, EnvStructError>` - The parsed value or an error.
    fn parse_from_env_var(
        var_name: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<Self, EnvStructError>
    where
        Self: Sized,
    {
        let var_name = var_name.as_ref().to_string();
        match std::env::var(&var_name) {
            Ok(ref value) => Self::parse(value).map_err(|e| EnvStructError::ParseEnvError {
                var_name,
                var_value: value.to_owned(),
                source: e,
            }),
            Err(e) => match default {
                Some(default) => {
                    Self::parse(default).map_err(|e| EnvStructError::ParseDefaultError {
                        var_name,
                        var_value: default.to_owned(),
                        source: e,
                    })
                }
                None => match e {
                    std::env::VarError::NotPresent => Err(EnvStructError::MissingEnvVar(var_name)),
                    std::env::VarError::NotUnicode(_) => {
                        Err(EnvStructError::InvalidVarFormat(var_name))
                    }
                },
            },
        }
    }

    /// Retrieves environment variable entries for documentation purposes.
    ///
    /// # Arguments
    ///
    /// * `prefix` - A prefix for the environment variable names.
    /// * `default` - An optional default value.
    ///
    /// # Returns
    ///
    /// * `Result<Vec<EnvEntry>, EnvStructError>` - A list of environment entries or an error.
    fn get_env_entries(
        prefix: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<Vec<EnvEntry>, EnvStructError> {
        Ok(vec![EnvEntry {
            name: prefix.as_ref().to_string(),
            typ: std::any::type_name::<Self>().to_string(),
            default: default.map(|v| v.to_string()),
        }])
    }
}

macro_rules! implement_primitive {
    ($x:ty) => {
        impl EnvParsePrimitive for $x {
            fn parse(val: &str) -> Result<Self, BoxError> {
                Ok(val.trim().parse::<$x>()?)
            }
        }
    };
}

implement_primitive!(bool); // "true" | "false"
implement_primitive!(usize);
implement_primitive!(char);
implement_primitive!(u8);
implement_primitive!(u16);
implement_primitive!(u32);
implement_primitive!(u64);
implement_primitive!(u128);

implement_primitive!(i8);
implement_primitive!(i16);
implement_primitive!(i32);
implement_primitive!(i64);
implement_primitive!(i128);

implement_primitive!(f32);
implement_primitive!(f64);

implement_primitive!(std::path::PathBuf);

#[cfg(feature = "serde_json")]
implement_primitive!(serde_json::Value);

#[cfg(feature = "humantime")]
implement_primitive!(humantime::Duration); // "60s"

#[cfg(feature = "bytesize")]
implement_primitive!(bytesize::ByteSize); // "1.50MB"

#[cfg(feature = "url")]
implement_primitive!(url::Url); // "https://user:password@example.com/path?query=arg#hash"

#[cfg(feature = "regex")]
implement_primitive!(regex::Regex);

#[cfg(feature = "chrono")]
impl EnvParsePrimitive for chrono::DateTime<chrono::Utc> {
    fn parse(val: &str) -> Result<Self, BoxError> {
        Ok(chrono::DateTime::parse_from_rfc3339(val.trim())?.to_utc())
    }
}

#[cfg(feature = "chrono")]
impl EnvParsePrimitive for chrono::DateTime<chrono::FixedOffset> {
    fn parse(val: &str) -> Result<Self, BoxError> {
        Ok(chrono::DateTime::parse_from_rfc3339(val.trim())?)
    }
}

#[cfg(feature = "chrono")]
impl EnvParsePrimitive for chrono::NaiveDateTime {
    fn parse(val: &str) -> Result<Self, BoxError> {
        Ok(chrono::NaiveDateTime::parse_from_str(
            val.trim(),
            "%Y-%m-%d %H:%M:%S",
        )?)
    }
}

#[cfg(feature = "jiff")]
implement_primitive!(jiff::Timestamp); // "2024-01-01T00:00:00Z"

// "2024-01-01T00:00:00+01:00[Europe/Berlin]" or a bare offset "2024-01-01T00:00:00+01:00"
#[cfg(feature = "jiff")]
impl EnvParsePrimitive for jiff::Zoned {
    fn parse(val: &str) -> Result<Self, BoxError> {
        let val = val.trim();
        let pieces = jiff::fmt::temporal::Pieces::parse(val)?;

        // `Zoned::from_str` rejects anything without a time zone annotation, so
        // fall back to a fixed zone built from the offset the value carries
        let Some(offset) = pieces.offset() else {
            if pieces.time_zone_annotation().is_some() {
                return Ok(val.parse()?);
            }
            return Err("datetime has neither a time zone nor an offset".into());
        };
        if pieces.time_zone_annotation().is_some() {
            return Ok(val.parse()?);
        }

        let offset = match offset {
            jiff::fmt::temporal::PiecesOffset::Zulu => jiff::tz::Offset::UTC,
            other => other.to_numeric_offset(),
        };
        let time = pieces.time().unwrap_or(jiff::civil::Time::midnight());
        Ok(pieces
            .date()
            .to_datetime(time)
            .to_zoned(jiff::tz::TimeZone::fixed(offset))?)
    }
}

#[cfg(feature = "jiff")]
implement_primitive!(jiff::civil::DateTime); // "2024-01-01 00:00:00"

#[cfg(feature = "jiff")]
implement_primitive!(jiff::civil::Date); // "2024-01-01"

#[cfg(feature = "jiff")]
implement_primitive!(jiff::civil::Time); // "13:45:00"

// accepts both the friendly syntax ("1h 30m") and ISO-8601 ("PT1H30M")
#[cfg(feature = "jiff")]
implement_primitive!(jiff::Span);

/// Gregorian averages, matching how `humantime` expands the same units, so that
/// one string means the same in both duration types this crate supports.
#[cfg(feature = "jiff")]
const SECS_PER_MONTH: i64 = 2_630_016; // 30.44 days

#[cfg(feature = "jiff")]
const SECS_PER_YEAR: i64 = 31_557_600; // 365.25 days

#[cfg(feature = "jiff")]
impl EnvParsePrimitive for jiff::SignedDuration {
    fn parse(val: &str) -> Result<Self, BoxError> {
        let val = val.trim();
        if let Ok(duration) = val.parse() {
            return Ok(duration);
        }

        // `SignedDuration` is an exact span of time, so its own parser rejects
        // calendar units; a config value is never relative to a moment, which
        // makes them plain duration units here, as they are in humantime
        let span = val.parse::<jiff::Span>()?;
        let calendar = jiff::SignedDuration::from_secs(
            span.get_years() as i64 * SECS_PER_YEAR
                + span.get_months() as i64 * SECS_PER_MONTH
                + span.get_weeks() as i64 * 7 * 24 * 60 * 60,
        );

        // days and below are exact, so the reference date cannot affect them
        let exact = jiff::Span::new()
            .days(span.get_days())
            .hours(span.get_hours())
            .minutes(span.get_minutes())
            .seconds(span.get_seconds())
            .milliseconds(span.get_milliseconds())
            .microseconds(span.get_microseconds())
            .nanoseconds(span.get_nanoseconds())
            .to_duration(jiff::civil::date(1970, 1, 1))?;

        Ok(calendar + exact)
    }
}

impl EnvParsePrimitive for std::time::Duration {
    fn parse(val: &str) -> Result<Self, BoxError> {
        Ok(std::time::Duration::from_secs_f64(
            val.trim().parse::<f64>()?,
        ))
    }
}

impl EnvParsePrimitive for String {
    fn parse(val: &str) -> Result<Self, BoxError> {
        Ok(val.trim().to_owned())
    }
}

impl<V: EnvParsePrimitive> EnvParsePrimitive for Vec<V> {
    fn parse(val: &str) -> Result<Self, BoxError> {
        val.split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| V::parse(s))
            .collect::<Result<Vec<_>, _>>()
    }
}

impl<K, V> EnvParsePrimitive for HashMap<K, V>
where
    K: EnvParsePrimitive + std::hash::Hash + std::cmp::Eq,
    V: EnvParsePrimitive,
{
    fn parse(val: &str) -> Result<Self, BoxError> {
        let v = val
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| match s.split_once('=') {
                Some((key, value)) => Ok((K::parse(key)?, V::parse(value)?)),
                None => Err(Box::new(EnvStructError::InvalidVarFormat(s.to_owned()))
                    as Box<dyn std::error::Error + Send + Sync + 'static>),
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(v)
    }
}

impl<K, V> EnvParsePrimitive for BTreeMap<K, V>
where
    K: EnvParsePrimitive + std::cmp::Ord,
    V: EnvParsePrimitive,
{
    fn parse(val: &str) -> Result<Self, BoxError> {
        let v = val
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| match s.split_once('=') {
                Some((key, value)) => Ok((K::parse(key)?, V::parse(value)?)),
                None => Err(Box::new(EnvStructError::InvalidVarFormat(s.to_owned()))
                    as Box<dyn std::error::Error + Send + Sync + 'static>),
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        Ok(v)
    }
}

impl<V> EnvParsePrimitive for HashSet<V>
where
    V: EnvParsePrimitive + std::hash::Hash + std::cmp::Eq,
{
    fn parse(val: &str) -> Result<Self, BoxError> {
        let v = val
            .split(';')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|value| V::parse(value))
            .collect::<Result<HashSet<_>, _>>()?;
        Ok(v)
    }
}

impl<T: EnvParsePrimitive> EnvParsePrimitive for Option<T> {
    fn parse(val: &str) -> Result<Self, BoxError> {
        Ok(Some(T::parse(val)?))
    }

    fn parse_from_env_var(
        var_name: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<Self, EnvStructError> {
        match T::parse_from_env_var(var_name, default) {
            Ok(value) => Ok(Some(value)),
            Err(err) => match err {
                EnvStructError::MissingEnvVar(_) => Ok(None),
                _ => Err(err),
            },
        }
    }
}

macro_rules! implement_primitive_t {
    ($x:ty) => {
        paste! {
            impl<T: EnvParsePrimitive> EnvParsePrimitive for $x::<T> {
                fn parse(val: &str) -> Result<Self, BoxError> {
                    Ok(T::parse(val.trim())?.into())
                }
            }
        }
    };
}

implement_primitive_t!(std::cell::Cell);
implement_primitive_t!(std::cell::RefCell);
implement_primitive_t!(std::rc::Rc);
implement_primitive_t!(std::sync::Arc);

macro_rules! implement_non_zero {
    ($x:ty) => {
        impl EnvParsePrimitive for $x {
            fn parse(val: &str) -> Result<Self, BoxError> {
                let value: $x = val
                    .parse()
                    .map_err(|_err| Box::new(EnvStructError::InvalidVarFormat(val.to_owned())))?;
                Ok(value)
            }
        }
    };
}

implement_non_zero!(std::num::NonZeroU8);
implement_non_zero!(std::num::NonZeroU16);
implement_non_zero!(std::num::NonZeroU32);
implement_non_zero!(std::num::NonZeroU64);
implement_non_zero!(std::num::NonZeroU128);
implement_non_zero!(std::num::NonZeroUsize);
implement_non_zero!(std::num::NonZeroI8);
implement_non_zero!(std::num::NonZeroI16);
implement_non_zero!(std::num::NonZeroI32);
implement_non_zero!(std::num::NonZeroI64);
implement_non_zero!(std::num::NonZeroI128);
implement_non_zero!(std::num::NonZeroIsize);
