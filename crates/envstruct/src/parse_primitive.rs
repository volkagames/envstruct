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

    /// Human-facing type shown in usage output.
    fn usage_type() -> UsageType {
        UsageType::Other(strip_namespace(std::any::type_name::<Self>()))
    }

    /// Closed set of allowed values, if known from the type declaration.
    /// For maps and lists, describes keys and items respectively, rather than
    /// complete variable values. Presence requirements are enforced by `parse`.
    fn usage_values() -> Option<Vec<String>> {
        None
    }

    /// Whether omitting the variable leaves this field unset.
    fn usage_optional() -> bool {
        false
    }

    /// Usage tree for this primitive value.
    fn get_usage_tree(
        prefix: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<UsageTree, EnvStructError> {
        Ok(UsageTree::leaf_field(
            prefix.as_ref(),
            Self::usage_type(),
            !Self::usage_optional() && default.is_none(),
            default.map(str::to_string),
            Self::usage_values(),
        ))
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
        Ok(Self::get_usage_tree(prefix, default)?.flatten_entries())
    }
}

macro_rules! implement_primitive {
    ($x:ty, $usage:expr) => {
        impl EnvParsePrimitive for $x {
            fn parse(val: &str) -> Result<Self, BoxError> {
                Ok(val.trim().parse::<$x>()?)
            }

            fn usage_type() -> UsageType {
                $usage
            }
        }
    };
}

/// Inclusive bounds of a numeric type, for types narrow enough that they matter.
macro_rules! int_bounds {
    ($x:ty, $excludes_zero:expr) => {
        UsageType::Integer(
            strip_namespace(stringify!($x)),
            Some(IntLimit::Range {
                min: <$x>::MIN.to_string(),
                max: <$x>::MAX.to_string(),
                excludes_zero: $excludes_zero,
            }),
        )
    };
}

macro_rules! int_type {
    ($x:ty) => {
        UsageType::Integer(stringify!($x).to_string(), None)
    };
}

implement_primitive!(bool, UsageType::Bool); // "true" | "false"
implement_primitive!(char, UsageType::Other("char".to_string()));

// Only narrow integers report bounds: for the wider ones the limits exist, but no
// realistic configuration value can reach them. The rust type name is still printed.
implement_primitive!(u8, int_bounds!(u8, false));
implement_primitive!(u16, int_bounds!(u16, false));
implement_primitive!(u32, int_type!(u32));
implement_primitive!(u64, int_type!(u64));
implement_primitive!(u128, int_type!(u128));
implement_primitive!(usize, int_type!(usize));

implement_primitive!(i8, int_bounds!(i8, false));
implement_primitive!(i16, int_bounds!(i16, false));
implement_primitive!(i32, int_type!(i32));
implement_primitive!(i64, int_type!(i64));
implement_primitive!(i128, int_type!(i128));

implement_primitive!(f32, UsageType::Float("f32".to_string()));
implement_primitive!(f64, UsageType::Float("f64".to_string()));

implement_primitive!(std::path::PathBuf, UsageType::Other("path".to_string()));

#[cfg(feature = "serde_json")]
implement_primitive!(serde_json::Value, UsageType::Other("json".to_string()));

#[cfg(feature = "humantime")]
implement_primitive!(humantime::Duration, UsageType::Duration); // "60s"

#[cfg(feature = "bytesize")]
implement_primitive!(bytesize::ByteSize, UsageType::ByteSize); // "1.50MB"

#[cfg(feature = "url")]
implement_primitive!(url::Url, UsageType::Url); // "https://user:password@example.com/path?query=arg#hash"

#[cfg(feature = "regex")]
implement_primitive!(regex::Regex, UsageType::Other("regex".to_string()));

#[cfg(feature = "chrono")]
impl EnvParsePrimitive for chrono::DateTime<chrono::Utc> {
    fn parse(val: &str) -> Result<Self, BoxError> {
        Ok(chrono::DateTime::parse_from_rfc3339(val.trim())?.to_utc())
    }

    fn usage_type() -> UsageType {
        UsageType::Other("datetime".to_string())
    }
}

#[cfg(feature = "chrono")]
impl EnvParsePrimitive for chrono::DateTime<chrono::FixedOffset> {
    fn parse(val: &str) -> Result<Self, BoxError> {
        Ok(chrono::DateTime::parse_from_rfc3339(val.trim())?)
    }

    fn usage_type() -> UsageType {
        UsageType::Other("datetime".to_string())
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

    fn usage_type() -> UsageType {
        UsageType::Other("datetime".to_string())
    }
}

impl EnvParsePrimitive for std::time::Duration {
    fn parse(val: &str) -> Result<Self, BoxError> {
        Ok(std::time::Duration::from_secs_f64(
            val.trim().parse::<f64>()?,
        ))
    }

    fn usage_type() -> UsageType {
        UsageType::Other("seconds".to_string())
    }
}

impl EnvParsePrimitive for String {
    fn parse(val: &str) -> Result<Self, BoxError> {
        Ok(val.trim().to_owned())
    }

    fn usage_type() -> UsageType {
        UsageType::String
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

    fn usage_type() -> UsageType {
        UsageType::List(Box::new(V::usage_type()))
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

    fn usage_type() -> UsageType {
        UsageType::Map(Box::new(K::usage_type()), Box::new(V::usage_type()))
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

    fn usage_type() -> UsageType {
        UsageType::Map(Box::new(K::usage_type()), Box::new(V::usage_type()))
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

    fn usage_type() -> UsageType {
        UsageType::Other(format!("set<{}>", V::usage_type().display()))
    }
}

impl<T: EnvParsePrimitive> EnvParsePrimitive for Option<T> {
    fn parse(val: &str) -> Result<Self, BoxError> {
        Ok(Some(T::parse(val)?))
    }

    fn usage_type() -> UsageType {
        T::usage_type()
    }

    fn usage_values() -> Option<Vec<String>> {
        T::usage_values()
    }

    fn usage_optional() -> bool {
        true
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

                fn usage_type() -> UsageType {
                    T::usage_type()
                }

                fn usage_values() -> Option<Vec<String>> {
                    T::usage_values()
                }

                fn usage_optional() -> bool {
                    T::usage_optional()
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
    ($x:ty, $usage:expr) => {
        impl EnvParsePrimitive for $x {
            fn parse(val: &str) -> Result<Self, BoxError> {
                let value: $x = val
                    .parse()
                    .map_err(|_err| Box::new(EnvStructError::InvalidVarFormat(val.to_owned())))?;
                Ok(value)
            }

            fn usage_type() -> UsageType {
                $usage
            }
        }
    };
}

macro_rules! nonzero_type {
    ($x:ty) => {
        UsageType::Integer(strip_namespace(stringify!($x)), Some(IntLimit::NonZero))
    };
}

// Narrow NonZero types show their bounds; unsigned ones already start at 1, so only
// the signed ones need the explicit zero exclusion. Wider ones show the exclusion alone.
implement_non_zero!(std::num::NonZeroU8, int_bounds!(std::num::NonZeroU8, false));
implement_non_zero!(
    std::num::NonZeroU16,
    int_bounds!(std::num::NonZeroU16, false)
);
implement_non_zero!(std::num::NonZeroI8, int_bounds!(std::num::NonZeroI8, true));
implement_non_zero!(
    std::num::NonZeroI16,
    int_bounds!(std::num::NonZeroI16, true)
);

implement_non_zero!(std::num::NonZeroU32, nonzero_type!(std::num::NonZeroU32));
implement_non_zero!(std::num::NonZeroU64, nonzero_type!(std::num::NonZeroU64));
implement_non_zero!(std::num::NonZeroU128, nonzero_type!(std::num::NonZeroU128));
implement_non_zero!(
    std::num::NonZeroUsize,
    nonzero_type!(std::num::NonZeroUsize)
);
implement_non_zero!(std::num::NonZeroI32, nonzero_type!(std::num::NonZeroI32));
implement_non_zero!(std::num::NonZeroI64, nonzero_type!(std::num::NonZeroI64));
implement_non_zero!(std::num::NonZeroI128, nonzero_type!(std::num::NonZeroI128));
implement_non_zero!(
    std::num::NonZeroIsize,
    nonzero_type!(std::num::NonZeroIsize)
);
