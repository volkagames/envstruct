use envstruct::prelude::*;

#[derive(EnvStruct, Debug)]
pub struct Config {
    pub db: DB,

    #[env(default = "https://example.com")]
    pub url: url::Url,

    #[env(default = "/var/log/app.log")]
    pub file_path: std::path::PathBuf,

    #[env(default = "1h")]
    pub duration: envstruct::Duration,

    #[env(default = "10Mib")]
    pub bytesize: envstruct::ByteSize,
}

#[derive(EnvStruct, Debug)]
pub struct DB {
    #[env(default = "localhost")]
    pub host: String,

    #[env(default = 8080)]
    pub port: u16,

    #[env(default = false)]
    pub debug: bool,
}

fn main() -> Result<(), envstruct::EnvStructError> {
    let config = Config::with_prefix("MY_APP")?;
    println!("{:#?}", config);
    println!("{}", Config::usage("MY_APP")?);
    Ok(())
}
