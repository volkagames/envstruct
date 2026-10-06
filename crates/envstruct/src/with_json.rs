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
        Self::parse_from_env_vars(&EnvVars::process(), var_name, default)
    }

    /// Parses a JSON string from a variable of `vars` into an instance of `T`.
    ///
    /// # Arguments
    ///
    /// * `vars` - The variables to read from.
    /// * `var_name` - The name of the environment variable.
    /// * `default` - An optional default value to use if the environment variable is not set.
    ///
    /// # Returns
    ///
    /// * `Ok(T)` if parsing is successful.
    /// * `Err(EnvStructError)` if parsing fails or the environment variable is not set.
    pub fn parse_from_env_vars(
        vars: &EnvVars,
        var_name: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<T, EnvStructError>
    where
        Self: Sized,
    {
        let var_name = var_name.as_ref();
        parse_var(var_name, vars.var(var_name), default, Self::parse)
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
        Ok(vec![EnvEntry {
            name: prefix.as_ref().to_string(),
            typ: std::any::type_name::<Self>().to_string(),
            default: default.map(|v| v.to_string()),
        }])
    }
}
