use envstruct::prelude::*;

#[derive(EnvStruct, Debug)]
pub struct Config {
    db: DB,

    #[env(default = "https://example.com")]
    url: url::Url,

    #[env(default = "/var/log/app.log")]
    file_path: std::path::PathBuf,

    #[env(default = "1h")]
    duration: envstruct::Duration,

    #[env(default = "10Mib")]
    bytesize: envstruct::ByteSize,
}

#[derive(EnvStruct, Debug)]
pub struct DB {
    #[env(default = "localhost")]
    host: String,

    #[env(default = 8080)]
    port: u16,

    #[env(default = false)]
    debug: bool,
}

fn main() -> Result<(), envstruct::EnvStructError> {
    let config = Config::with_prefix("MY_APP")?;
    println!("{:?}", config);
    Ok(())
}
