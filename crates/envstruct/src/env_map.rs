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
        Self::parse_from_env_vars(&EnvVars::process(), var_name, default)
    }

    /// Parses the variables of `vars` into an `EnvMap`.
    ///
    /// # Arguments
    ///
    /// * `vars` - The variables to read from.
    /// * `var_name` - The prefix of the environment variables to parse.
    /// * `default` - An optional default value.
    ///
    /// # Errors
    ///
    /// Returns an `EnvStructError` if parsing fails.
    fn parse_from_env_vars(
        vars: &EnvVars,
        var_name: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<Self, EnvStructError>
    where
        Self: Sized,
    {
        let var_name = var_name.as_ref();
        // keys start after a `_` separator, so `TEST_MAPPING` is not a key of `TEST_MAP`;
        // a map at the root (empty prefix) takes every variable
        let prefix = if var_name.is_empty() {
            String::new()
        } else {
            format!("{var_name}_")
        };
        // `vars()` panics on any non-UTF-8 name in the process, even one outside the prefix
        let map = vars
            .vars_os()
            .filter_map(|(k, _)| match k.into_string() {
                Ok(k) => {
                    let key = k.strip_prefix(&prefix)?.trim_start_matches('_').to_string();
                    Some(Ok((k, key)))
                }
                Err(k) => k
                    .as_encoded_bytes()
                    .starts_with(prefix.as_bytes())
                    .then(|| {
                        Err(EnvStructError::InvalidKeyFormat(
                            k.to_string_lossy().into_owned(),
                        ))
                    }),
            })
            .map(|entry| {
                let (k, key) = entry?;
                Ok((
                    K::from_str(&key)
                        .map_err(|_| EnvStructError::InvalidKeyFormat(k.to_string()))?,
                    V::parse_from_env_vars(vars, k, default)?,
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
