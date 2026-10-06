use crate::*;
use pastey::paste;

/// Trait for parsing nested environment variables.
pub trait EnvParseNested {
    /// Creates a new instance by parsing environment variables.
    ///
    /// # Errors
    ///
    /// Returns an `EnvStructError` if parsing fails.
    fn new() -> Result<Self, EnvStructError>
    where
        Self: Sized,
    {
        Self::parse_from_env_var("", None)
    }

    /// Creates a new instance with a specified prefix by parsing environment variables.
    ///
    /// # Arguments
    ///
    /// * `prefix` - A prefix for the environment variables.
    ///
    /// # Errors
    ///
    /// Returns an `EnvStructError` if parsing fails.
    fn with_prefix(prefix: impl AsRef<str>) -> Result<Self, EnvStructError>
    where
        Self: Sized,
    {
        Self::parse_from_env_var(prefix, None)
    }

    /// Creates a new instance by parsing a fixed set of variables instead of
    /// the process environment.
    ///
    /// # Arguments
    ///
    /// * `vars` - The variables to parse, as `(name, value)` pairs.
    ///
    /// # Errors
    ///
    /// Returns an `EnvStructError` if parsing fails.
    fn from_vars<K, V>(vars: impl IntoIterator<Item = (K, V)>) -> Result<Self, EnvStructError>
    where
        Self: Sized,
        K: Into<std::ffi::OsString>,
        V: Into<std::ffi::OsString>,
    {
        Self::parse_from_env_vars(&vars.into_iter().collect(), "", None)
    }

    /// Creates a new instance with a specified prefix by parsing a fixed set
    /// of variables instead of the process environment.
    ///
    /// # Arguments
    ///
    /// * `prefix` - A prefix for the environment variables.
    /// * `vars` - The variables to parse, as `(name, value)` pairs.
    ///
    /// # Errors
    ///
    /// Returns an `EnvStructError` if parsing fails.
    fn with_prefix_from_vars<K, V>(
        prefix: impl AsRef<str>,
        vars: impl IntoIterator<Item = (K, V)>,
    ) -> Result<Self, EnvStructError>
    where
        Self: Sized,
        K: Into<std::ffi::OsString>,
        V: Into<std::ffi::OsString>,
    {
        Self::parse_from_env_vars(&vars.into_iter().collect(), prefix, None)
    }

    /// Parses the environment variable with an optional default value.
    ///
    /// # Arguments
    ///
    /// * `var_name` - The name of the environment variable.
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
        Self: Sized;

    /// Parses the variables of `vars` with an optional default value.
    ///
    /// The derive macro implements this method. The default implementation is
    /// for types implemented by hand before `EnvVars` existed: it ignores
    /// `vars` and reads the process environment.
    ///
    /// # Arguments
    ///
    /// * `vars` - The variables to read from.
    /// * `var_name` - The name of the environment variable.
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
        let _ = vars;
        Self::parse_from_env_var(var_name, default)
    }

    /// Retrieves the environment entries with a specified prefix and optional default value.
    ///
    /// # Arguments
    ///
    /// * `prefix` - A prefix for the environment variables.
    /// * `default` - An optional default value.
    ///
    /// # Errors
    ///
    /// Returns an `EnvStructError` if retrieval fails.
    fn get_env_entries(
        prefix: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<Vec<EnvEntry>, EnvStructError>;
}

impl<T: EnvParseNested> EnvParseNested for Option<T> {
    fn parse_from_env_var(
        var_name: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<Self, EnvStructError>
    where
        Self: Sized,
    {
        Self::parse_from_env_vars(&EnvVars::process(), var_name, default)
    }

    fn parse_from_env_vars(
        vars: &EnvVars,
        var_name: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<Self, EnvStructError>
    where
        Self: Sized,
    {
        let var_name = var_name.as_ref();

        // Defining any environment variable of optional type makes the field required
        // otherwise it is None.
        if !T::get_env_entries(var_name, default)?
            .iter()
            .any(|entry| vars.var_os(&entry.name).is_some())
        {
            return Ok(None);
        }

        Ok(Some(T::parse_from_env_vars(vars, var_name, default)?))
    }

    fn get_env_entries(
        prefix: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<Vec<EnvEntry>, EnvStructError> {
        T::get_env_entries(prefix, default)
    }
}

/// Concatenates two environment variable names with an underscore.
///
/// # Arguments
///
/// * `lhs` - The left-hand side of the environment variable name.
/// * `rhs` - The right-hand side of the environment variable name.
///
/// # Returns
///
/// A concatenated string of the two environment variable names.
pub fn concat_env_name(lhs: impl AsRef<str>, rhs: impl AsRef<str>) -> String {
    let (lhs, rhs) = (lhs.as_ref().to_uppercase(), rhs.as_ref().to_uppercase());
    #[cfg(feature = "env_uppercase")]
    let (lhs, rhs) = (lhs.to_uppercase(), rhs.to_uppercase());
    match (lhs.is_empty(), rhs.is_empty()) {
        (false, true) => lhs.to_string(),
        (true, false) => rhs.to_string(),
        _ => format!("{lhs}_{rhs}"),
    }
}

macro_rules! implement_nested_t {
    ($x:ty) => {
        paste! {
            impl<T: EnvParseNested> EnvParseNested for $x::<T> {
                fn parse_from_env_var(var_name: impl AsRef<str>, default: Option<&str>) -> Result<Self, EnvStructError> {
                    Ok(T::parse_from_env_var(var_name, default)?.into())
                }

                fn parse_from_env_vars(vars: &EnvVars, var_name: impl AsRef<str>, default: Option<&str>) -> Result<Self, EnvStructError> {
                    Ok(T::parse_from_env_vars(vars, var_name, default)?.into())
                }

                fn get_env_entries(
                    prefix: impl AsRef<str>,
                    default: Option<&str>,
                ) -> Result<Vec<EnvEntry>, EnvStructError> {
                    T::get_env_entries(prefix, default)
                }
            }
        }
    };
}

implement_nested_t!(std::cell::Cell);
implement_nested_t!(std::cell::RefCell);
implement_nested_t!(std::rc::Rc);
implement_nested_t!(std::sync::Arc);
