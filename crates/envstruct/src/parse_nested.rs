use crate::*;
use paste::paste;

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
        let var_name = var_name.as_ref();

        // Defining any environment variable of optional type makes the field required
        // otherwise it is None.
        if !T::get_env_entries(var_name, default)?
            .iter()
            .any(|entry| std::env::var_os(&entry.name).is_some())
        {
            return Ok(None);
        }

        Ok(Some(T::parse_from_env_var(var_name, default)?))
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
