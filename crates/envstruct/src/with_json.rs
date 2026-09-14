#![cfg(feature = "serde_json")]

use crate::*;

/// A wrapper struct that provides JSON parsing functionality for a given type `T`.
///
/// `T` must implement the `serde::de::Deserialize` trait.
pub struct WithJson<T: for<'a> serde::de::Deserialize<'a>>(T);

impl<T: for<'a> serde::de::Deserialize<'a>> WithJson<T> {
    /// Parses a JSON string into an instance of `T`.
    ///
    /// # Arguments
    ///
    /// * `val` - A string slice that holds the JSON data.
    ///
    /// # Returns
    ///
    /// * `Ok(T)` if parsing is successful.
    /// * `Err(BoxError)` if parsing fails.
    pub fn parse(val: &str) -> Result<T, BoxError> {
        Ok(serde_json::from_str(val)?)
    }

    /// Parses a JSON string from an environment variable into an instance of `T`.
    ///
    /// # Arguments
    ///
    /// * `var_name` - The name of the environment variable.
    /// * `default` - An optional default value to use if the environment variable is not set.
    ///
    /// # Returns
    ///
    /// * `Ok(T)` if parsing is successful.
    /// * `Err(EnvStructError)` if parsing fails or the environment variable is not set.
    pub fn parse_from_env_var(
        var_name: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<T, EnvStructError>
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

    /// Usage tree for a JSON environment variable.
    pub fn get_usage_tree(
        prefix: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<UsageTree, EnvStructError> {
        Ok(UsageTree::leaf_field(
            prefix.as_ref(),
            UsageType::Other("json".to_string()),
            default.is_none(),
            default.map(str::to_string),
            None,
        ))
    }

    /// Retrieves environment variable entries for documentation purposes.
    ///
    /// # Arguments
    ///
    /// * `prefix` - A prefix to use for the environment variable names.
    /// * `default` - An optional default value to use if the environment variable is not set.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<EnvEntry>)` containing the environment variable entries.
    /// * `Err(EnvStructError)` if an error occurs.
    pub fn get_env_entries(
        prefix: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<Vec<EnvEntry>, EnvStructError> {
        Ok(Self::get_usage_tree(prefix, default)?.flatten_entries())
    }
}
