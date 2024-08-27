use envstruct::prelude::*;
use std::collections::BTreeMap;

#[derive(EnvStruct, Debug)]
pub struct Config {
    #[env(default = "default value")]
    pub value: String,

    pub file_path: std::path::PathBuf,
    pub duration: humantime::Duration,
    pub bytesize: bytesize::ByteSize,
    pub url: url::Url,
    pub regex: regex::Regex,
    pub datetime: chrono::DateTime<chrono::FixedOffset>,
    pub utc: chrono::DateTime<chrono::Utc>,
    pub naive: chrono::NaiveDateTime,

    pub vec_of_strings: Vec<String>,
    pub vec_of_ints: Vec<i64>,
    pub vec_of_floats: Vec<f64>,
    pub vec_of_durations: Vec<humantime::Duration>,
    pub vec_of_vec: Vec<Vec<String>>,

    pub map_str_str: BTreeMap<String, String>,
    pub map_str_int: BTreeMap<String, i64>,
    pub map_str_vec_of_bool: BTreeMap<String, Vec<bool>>,

    pub env_map: EnvMap<i32, String>,
}

fn main() {
    let usage = Config::usage_with_prefix("TEST").unwrap();
    println!("usage: \n{usage}");
}
