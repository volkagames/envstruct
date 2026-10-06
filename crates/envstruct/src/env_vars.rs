use std::{
    collections::BTreeMap,
    env::VarError,
    ffi::{OsStr, OsString},
};

/// The environment variables a configuration is parsed from: either the
/// process environment or a fixed set of variables.
///
/// A fixed set lets tests parse a configuration without `std::env::set_var`,
/// which is `unsafe` since Rust 2024. Variables outside the set are not read,
/// even when the process has them.
///
/// ```
/// use envstruct::prelude::*;
///
/// #[derive(EnvStruct)]
/// struct Config {
///     port: u16,
/// }
///
/// let config = Config::with_prefix_from_vars("APP", [("APP_PORT", "8080")]).unwrap();
/// assert_eq!(config.port, 8080);
/// ```
#[derive(Debug, Clone)]
pub struct EnvVars(Source);

#[derive(Debug, Clone)]
enum Source {
    Process,
    Fixed(BTreeMap<OsString, OsString>),
}

impl EnvVars {
    /// The environment of the current process.
    pub fn process() -> Self {
        Self(Source::Process)
    }

    /// Returns `true` if the variables are read from the process environment.
    pub fn is_process(&self) -> bool {
        matches!(self.0, Source::Process)
    }

    /// Fetches the variable `name`, like `std::env::var`.
    ///
    /// # Errors
    ///
    /// Returns `VarError::NotPresent` if the variable is not set and
    /// `VarError::NotUnicode` if its value is not valid unicode.
    pub fn var(&self, name: impl AsRef<OsStr>) -> Result<String, VarError> {
        match &self.0 {
            Source::Process => std::env::var(name),
            Source::Fixed(vars) => match vars.get(name.as_ref()) {
                Some(value) => value.clone().into_string().map_err(VarError::NotUnicode),
                None => Err(VarError::NotPresent),
            },
        }
    }

    /// Fetches the variable `name`, like `std::env::var_os`.
    pub fn var_os(&self, name: impl AsRef<OsStr>) -> Option<OsString> {
        match &self.0 {
            Source::Process => std::env::var_os(name),
            Source::Fixed(vars) => vars.get(name.as_ref()).cloned(),
        }
    }

    /// Iterates over all variables, like `std::env::vars_os`.
    ///
    /// A fixed set is iterated in the order of its names.
    pub fn vars_os(&self) -> Box<dyn Iterator<Item = (OsString, OsString)> + '_> {
        match &self.0 {
            Source::Process => Box::new(std::env::vars_os()),
            Source::Fixed(vars) => Box::new(vars.iter().map(|(k, v)| (k.clone(), v.clone()))),
        }
    }
}

impl<K: Into<OsString>, V: Into<OsString>> FromIterator<(K, V)> for EnvVars {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(iter: I) -> Self {
        Self(Source::Fixed(
            iter.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        ))
    }
}

impl<K: Into<OsString>, V: Into<OsString>, const N: usize> From<[(K, V); N]> for EnvVars {
    fn from(vars: [(K, V); N]) -> Self {
        vars.into_iter().collect()
    }
}
