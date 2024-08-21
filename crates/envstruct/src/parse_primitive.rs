use crate::*;
use paste::paste;
use std::collections::{BTreeMap, HashMap, HashSet};

pub trait EnvParsePrimitive {
    fn parse(val: &str) -> Result<Self, StdError>
    where
        Self: Sized;

    fn from_env_var(
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

    fn inspect_env_entry(
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
            fn parse(val: &str) -> Result<Self, StdError> {
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
    fn parse(val: &str) -> Result<Self, StdError> {
        Ok(chrono::DateTime::parse_from_rfc3339(val.trim())?.to_utc())
    }
}

#[cfg(feature = "chrono")]
impl EnvParsePrimitive for chrono::DateTime<chrono::FixedOffset> {
    fn parse(val: &str) -> Result<Self, StdError> {
        Ok(chrono::DateTime::parse_from_rfc3339(val.trim())?)
    }
}

#[cfg(feature = "chrono")]
impl EnvParsePrimitive for chrono::NaiveDateTime {
    fn parse(val: &str) -> Result<Self, StdError> {
        Ok(chrono::NaiveDateTime::parse_from_str(
            val.trim(),
            "%Y-%m-%d %H:%M:%S",
        )?)
    }
}

impl EnvParsePrimitive for std::time::Duration {
    fn parse(val: &str) -> Result<Self, StdError> {
        Ok(std::time::Duration::from_secs_f64(
            val.trim().parse::<f64>()?,
        ))
    }
}

impl EnvParsePrimitive for String {
    fn parse(val: &str) -> Result<Self, StdError> {
        Ok(val.trim().to_owned())
    }
}

impl<V: EnvParsePrimitive> EnvParsePrimitive for Vec<V> {
    fn parse(val: &str) -> Result<Self, StdError> {
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
    fn parse(val: &str) -> Result<Self, StdError> {
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
    fn parse(val: &str) -> Result<Self, StdError> {
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
    fn parse(val: &str) -> Result<Self, StdError> {
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
    fn parse(val: &str) -> Result<Self, StdError> {
        Ok(Some(T::parse(val)?))
    }

    fn from_env_var(
        var_name: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<Self, EnvStructError> {
        match T::from_env_var(var_name, default) {
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
                fn parse(val: &str) -> Result<Self, StdError> {
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
