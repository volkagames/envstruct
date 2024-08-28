use crate::*;
use paste::paste;

pub trait EnvParseNested {
    fn new() -> Result<Self, EnvStructError>
    where
        Self: Sized,
    {
        Self::parse_from_env_var("", None)
    }

    fn with_prefix(prefix: impl AsRef<str>) -> Result<Self, EnvStructError>
    where
        Self: Sized,
    {
        Self::parse_from_env_var(prefix, None)
    }

    fn parse_from_env_var(
        var_name: impl AsRef<str>,
        default: Option<&str>,
    ) -> Result<Self, EnvStructError>
    where
        Self: Sized;

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
