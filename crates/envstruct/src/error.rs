use thiserror::Error;

/// A boxed error type that is `Send`, `Sync`, and `'static`.
pub type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

const CAPTION: &str = "Configuration from environment variables failed";

/// Represents errors that can occur while processing environment variables.
#[derive(Debug, Error)]
pub enum EnvStructError {
    /// Error that occurs when an environment variable cannot be parsed.
    ///
    /// `var_name` is the name of the environment variable.
    /// `var_value` is the value of the environment variable.
    /// `source` is the underlying error that caused this error.
    #[error("{CAPTION}. `{var_name}` unable to parse value `{var_value}`, {source}")]
    ParseEnvError {
        var_name: String,
        var_value: String,
        #[source]
        source: BoxError,
    },

    /// Error that occurs when a default value cannot be parsed.
    ///
    /// `var_name` is the name of the environment variable.
    /// `var_value` is the default value of the environment variable.
    /// `source` is the underlying error that caused this error.
    #[error("{CAPTION}. `{var_name}` unable to parse default value `{var_value}`, {source}")]
    ParseDefaultError {
        var_name: String,
        var_value: String,
        #[source]
        source: BoxError,
    },

    /// Error that occurs when an expected environment variable is missing.
    ///
    /// The string is the name of the missing environment variable.
    #[error("{CAPTION}. Environment variable `{0}` is not present")]
    MissingEnvVar(String),

    /// Error that occurs when an environment variable key has an invalid format.
    ///
    /// The string is the invalid key.
    #[error("{CAPTION}. Invalid key format `{0}`")]
    InvalidKeyFormat(String),

    /// Error that occurs when an environment variable value has an invalid format.
    ///
    /// The string is the invalid value.
    #[error("{CAPTION}. Invalid environment value format `{0}`")]
    InvalidVarFormat(String),
}
