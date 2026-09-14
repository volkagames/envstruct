use envstruct::prelude::*;

#[allow(non_camel_case_types)]
#[derive(EnvStruct, Debug, Clone, PartialEq, Eq, strum::Display, strum::EnumString)]
pub enum Mode {
    remote,
    local,
}

#[derive(EnvStruct, Debug)]
#[env(title = "Deploy")]
pub struct DeployConfig {
    #[env(default = "remote")]
    pub mode: Mode,

    // Not an Option: the group is always parsed, the condition only documents usage.
    #[env(title = "Remote", used_if = "mode=remote")]
    pub remote: RemoteConfig,

    // Option: the whole group may be omitted.
    #[env(title = "Local", used_if = "mode=local")]
    pub local: Option<LocalConfig>,

    #[env(default = "60s")]
    pub reload_delay: envstruct::Duration,
}

#[derive(EnvStruct, Debug)]
pub struct RemoteConfig {
    pub dsn: String,
    #[env(default = "15s")]
    pub request_timeout: envstruct::Duration,
    #[env(default = "10000")]
    pub cache_size: u32,
}

#[derive(EnvStruct, Debug)]
pub struct LocalConfig {
    pub data_dir: String,
}

fn main() {
    println!("{}", DeployConfig::usage_with_prefix("DEPLOY").unwrap());
}
