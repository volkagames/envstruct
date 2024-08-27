# EnvStruct

Yo, check it. EnvStruct is here to make handling those env variables smooth as butter. It’s packed with tools to pull common types straight from environment variables and drop 'em into nested structures, no hassle. Plus, it's got those dope derive macros that keep things simple. This crate is all about that "battery included" life, bringing built-in support for the usual suspects and types, so you can hit the ground running with ease.

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
- Custom Parsing: Create custom parsers for specialized types.
- Prefix Support: Seamlessly handle environment variables with a common prefix.
- Default Values: Set default values for environment variables when they aren't provided.
- Error Handling: Receive detailed error messages to troubleshoot environment configuration issues effectively.
- Testing: benefit from a well-tested library with numerous test cases for reliability.
- Derive macros: Clean and readable thanks to darling crate

## Complex types

- Dates and Times: Parse chrono::DateTime and chrono::NaiveDateTime types, allowing you to easily work with dates and times directly from your environment variables.
- Durations: Parse duration from environment variables in human-readable formats, such as "1h", "30m", or "15s".
- URLs: Parse url::Url to handle and validate URLs from your environment variables, ensuring they are well-formed and valid.
- Regex Patterns: Parse regex::Regex to validate or match patterns directly from environment variables, allowing for dynamic regular expressions.
- File Paths: Parse std::path::PathBuf to work with file and directory paths, providing flexibility in file system operations.
- Byte Sizes: Parse sizes like "10KB", "5MB", or "1GB" into u64 representing bytes, useful for memory-related configurations.
- JSON Values: Parse serde_json::Value for scenarios where you need to handle arbitrary JSON data from environment variables.
- Collections: Parse std::collections::HashMap, std::collections::BTreeMap, and std::collections::HashSet directly from environment variables, key-value pairs are separated by ";", with each key and value separated by "=".
- Vec parse from environment variables, values can be separated by commas, allowing you to handle lists of items easily.
- EnvMap: Parse variables with a common prefix into a HashMap.

## Macro attributes

- `name` - name of an environment variable which provides a field value.
- `default` - default value of a field if an environment variable doesn't exist. If the environment variable exist but has invalid value an error returns.
- `flatten` - allows to ignore the field name when collecting the full name of an environment variable
- `with` - custom parser for field

## License

This project is licensed under the MPL-2 License. See the LICENSE file for details.
