use crate::*;
use std::{collections::HashMap, str::FromStr};

/// A wrapper around `HashMap` to provide environment variable parsing functionality.
#[derive(Debug, Clone)]
pub struct EnvMap<K, V>(pub HashMap<K, V>);

impl<K, V> AsRef<HashMap<K, V>> for EnvMap<K, V> {
    /// Returns a reference to the underlying `HashMap`.
    fn as_ref(&self) -> &HashMap<K, V> {
        &self.0
    }
}

impl<K, V> std::ops::Deref for EnvMap<K, V> {
    type Target = HashMap<K, V>;
    /// Dereferences to the underlying `HashMap`.
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<K, V> std::ops::DerefMut for EnvMap<K, V> {
    /// Dereferences to the underlying mutable `HashMap`.
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<K, V> EnvParseNested for EnvMap<K, V>
where
    K: FromStr + std::hash::Hash + std::cmp::Eq,
    V: EnvParsePrimitive,
{
    /// Parses the environment variables into an `EnvMap`.
    ///
    /// # Arguments
    ///
    /// * `var_name` - The prefix of the environment variables to parse.
    /// * `default` - An optional default value.
    ///
    /// # Errors
    ///
    /// Returns an `EnvStructError` if parsing fails.
    fn parse_from_env_var(
        var_name: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<Self, EnvStructError>
    where
        Self: Sized,
    {
        let var_name = var_name.as_ref();
        let map = std::env::vars()
            .filter_map(|(k, _)| {
                let key = k
                    .strip_prefix(var_name)
                    .map(|key| key.trim_start_matches('_'))?
                    .to_string();
                Some((k, key))
            })
            .map(|(k, key)| {
                Ok((
                    K::from_str(&key)
                        .map_err(|_| EnvStructError::InvalidKeyFormat(k.to_string()))?,
                    V::parse_from_env_var(k, default)?,
                ))
            })
            .collect::<Result<HashMap<_, _>, _>>()?;
        Ok(Self(map))
    }

    /// Gets the environment entries for the `EnvMap`.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prefix for the environment entries.
    /// * `default` - An optional default value.
    ///
    /// # Returns
    ///
    /// A vector of `EnvEntry` objects.
    ///
    /// # Errors
    ///
    /// Returns an `EnvStructError` if retrieval fails.
    fn get_env_entries(
        prefix: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<Vec<EnvEntry>, EnvStructError> {
        Ok(vec![EnvEntry {
            name: format!("{}_*", prefix.as_ref()),
            typ: std::any::type_name::<Self>().to_string(),
            default: default.map(|v| v.to_string()),
        }])
    }
}
