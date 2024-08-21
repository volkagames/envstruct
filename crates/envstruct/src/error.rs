use derivative::*;
use thiserror::Error;

pub type StdError = Box<dyn std::error::Error + Send + Sync + 'static>;

const CAPTION: &str = "Configuration from environment variables failed";

#[derive(Derivative, Error)]
#[derivative(Debug, PartialEq)]
pub enum EnvStructError {
    #[error("{CAPTION}. `{var_name}` unable to parse value `{var_value}`, {source}")]
    ParseEnvError {
        var_name: String,
        var_value: String,
        #[derivative(PartialEq = "ignore")]
        #[source]
        source: StdError,
    },

    #[error("{CAPTION}. `{var_name}` unable to parse default value `{var_value}`, {source}")]
    ParseDefaultError {
        var_name: String,
        var_value: String,
        #[derivative(PartialEq = "ignore")]
        #[source]
        source: StdError,
    },

    #[error("{CAPTION}. Environment variable `{0}` is not present")]
    MissingEnvVar(String),

    #[error("{CAPTION}. Invalid key format `{0}`")]
    InvalidKeyFormat(String),

    #[error("{CAPTION}. Invalid environment value format `{0}`")]
    InvalidVarFormat(String),
}
