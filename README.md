# EnvStruct

EnvStruct simplifies the process of handling environment variables in Rust applications. It provides tools to parse common types from environment variables and map them into nested structures effortlessly. With derive macros, the crate ensures clean and readable code. EnvStruct offers built-in support for various types, making it easy to get started quickly.

## Installation

Add the following to your Cargo.toml:

```toml
[dependencies]
envstruct = "1.0"
```

```rust
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


```

## Features

- Nested Structures: Parse environment variables into nested Rust structures.
- Custom Parsing: Create custom parsers for special types.
- Prefix Support: Handle environment variables with a common prefix.
- Default Values: Set default values for environment variables.
- Error Handling: Get detailed error messages for troubleshooting.
- Testing: Well-tested library with many test cases.
- Derive Macros: Clean and readable code with derive macros.

## Complex Types

- Dates and Times: Parse `chrono::DateTime` and `chrono::NaiveDateTime` types.
- Durations: Parse durations like "1h", "30m", or "15s".
- URLs: Parse `url::Url` to handle and validate URLs.
- Regex Patterns: Parse `regex::Regex` for dynamic regular expressions.
- File Paths: Parse `std::path::PathBuf` for file and directory paths.
- Byte Sizes: Parse sizes like "10KB", "5MB", or "1GB" into bytes.
- JSON Values: Parse `serde_json::Value` for arbitrary JSON data.
- Collections: Parse `HashMap`, `BTreeMap`, and `HashSet` from environment variables.
- Vectors: Parse lists of items separated by commas.
- EnvMap: Parse variables with a common prefix into a `HashMap`.

## Macro Attributes

- `name`: Name of the environment variable for a field.
- `default`: Default value if the environment variable doesn't exist.
- `flatten`: Ignore the field name when collecting the full name of an environment variable.
- `with`: Custom parser for a field.

## License

This project is licensed under the MPL-2 License. See the LICENSE file for details.
