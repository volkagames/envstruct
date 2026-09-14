#![allow(dead_code)]

use envstruct::prelude::*;
use serial_test::*;
use std::env;

#[allow(non_camel_case_types)]
#[derive(EnvStruct, Debug, Clone, PartialEq, Eq, strum::Display, strum::EnumString)]
pub enum StoreMode {
    gcs,
    local,
    mock,
}

fn clean_env() {
    std::env::vars().for_each(|(name, _)| {
        std::env::remove_var(name);
    });
}

fn find_field<'a>(items: &'a [UsageItem], name: &str) -> Option<&'a UsageField> {
    for item in items {
        match item {
            UsageItem::Field(field) if field.name == name => return Some(field),
            UsageItem::Group(group) => {
                if let Some(field) = find_field(&group.items, name) {
                    return Some(field);
                }
            }
            _ => {}
        }
    }
    None
}

/// REQUIRED cell of a group marker line, as printed in the table.
fn group_required<'a>(usage: &'a str, marker: &str) -> &'a str {
    usage
        .lines()
        .find(|line| line.starts_with(marker))
        .unwrap_or_else(|| panic!("no group line {marker}"))
        .split_whitespace()
        .last()
        .unwrap()
}

fn find_group<'a>(items: &'a [UsageItem], title: &str) -> Option<&'a UsageGroup> {
    for item in items {
        match item {
            UsageItem::Group(group) if group.title == title => return Some(group),
            UsageItem::Group(group) => {
                if let Some(found) = find_group(&group.items, title) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

#[derive(EnvStruct, Debug, PartialEq)]
#[env(title = "Avatars")]
pub struct AvatarConfig {
    #[env(default = "gcs")]
    pub mode: StoreMode,

    #[env(flatten, title = "GCS", used_if = "mode=gcs")]
    pub gcs: Option<GcsConfig>,

    #[env(title = "Local", used_if = "mode=local")]
    pub local: Option<LocalConfig>,

    #[env(title = "Mock", used_if = "mode=mock")]
    pub mock: Option<MockConfig>,

    #[env(flatten)]
    pub image: ImageConfig,
}

#[derive(EnvStruct, Debug, PartialEq)]
pub struct GcsConfig {
    pub bucket_name: String,
    #[env(default = "squibblefluff")]
    pub digest_salt: String,
}

#[derive(EnvStruct, Debug, PartialEq)]
pub struct LocalConfig {
    pub data_dir: String,
}

#[derive(EnvStruct, Debug, PartialEq)]
pub struct MockConfig {}

#[derive(EnvStruct, Debug, PartialEq)]
pub struct ImageConfig {
    #[env(default = "4MB")]
    pub image_size_limit: envstruct::ByteSize,
    #[env(default = "150")]
    pub image_width: u32,
    #[env(default = "0.9")]
    pub nsfw_score_max: f64,
}

#[test]
#[serial]
fn required_field_is_yes_and_missing_fails_parse() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub name: String,
    }

    let tree = Config::get_usage_tree("TEST", None).unwrap();
    let field = find_field(&tree.items, "TEST_NAME").unwrap();
    assert!(field.required);
    assert_eq!(field.default, None);
    assert_eq!(field.typ, UsageType::String);

    clean_env();
    let err = Config::with_prefix("TEST").unwrap_err();
    assert!(matches!(err, EnvStructError::MissingEnvVar(name) if name == "TEST_NAME"));
}

#[test]
#[serial]
fn default_field_is_no_and_shown_value_is_applied() {
    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct Config {
        #[env(default = "gcs")]
        pub mode: StoreMode,
        #[env(default = "150")]
        pub width: u32,
    }

    let tree = Config::get_usage_tree("TEST", None).unwrap();
    let mode = find_field(&tree.items, "TEST_MODE").unwrap();
    assert!(!mode.required);
    assert_eq!(mode.default.as_deref(), Some("gcs"));
    assert_eq!(mode.typ, UsageType::Enum);
    assert_eq!(
        mode.values.as_ref().map(|v| v.as_slice()),
        Some(["gcs".to_string(), "local".to_string(), "mock".to_string()].as_slice())
    );

    let width = find_field(&tree.items, "TEST_WIDTH").unwrap();
    assert!(!width.required);
    assert_eq!(width.default.as_deref(), Some("150"));

    clean_env();
    let config = Config::with_prefix("TEST").unwrap();
    assert_eq!(config.mode, StoreMode::gcs);
    assert_eq!(config.width, 150);
}

#[test]
#[serial]
fn optional_without_default_is_no_and_stays_unset() {
    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct Config {
        pub name: Option<String>,
    }

    let tree = Config::get_usage_tree("TEST", None).unwrap();
    let field = find_field(&tree.items, "TEST_NAME").unwrap();
    assert!(!field.required);
    assert_eq!(field.default, None);
    assert_eq!(field.typ, UsageType::String);

    clean_env();
    let config = Config::with_prefix("TEST").unwrap();
    assert_eq!(config.name, None);
}

#[test]
#[serial]
fn empty_default_differs_from_missing_default() {
    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct Config {
        #[env(default = "")]
        pub empty: String,
        pub missing: Option<String>,
    }

    let tree = Config::get_usage_tree("TEST", None).unwrap();
    let empty = find_field(&tree.items, "TEST_EMPTY").unwrap();
    let missing = find_field(&tree.items, "TEST_MISSING").unwrap();
    assert_eq!(empty.default.as_deref(), Some(""));
    assert_eq!(missing.default, None);

    let usage = Config::usage_with_prefix("TEST").unwrap();
    assert!(usage.contains(r#""""#));
    assert!(usage.contains('—'));

    clean_env();
    let config = Config::with_prefix("TEST").unwrap();
    assert_eq!(config.empty, "");
    assert_eq!(config.missing, None);
}

#[test]
#[serial]
fn default_mode_optional_group_is_documented_but_not_parser_required() {
    let tree = AvatarConfig::get_usage_tree("AVATARDB", None).unwrap();
    let gcs = find_group(&tree.items, "GCS").unwrap();
    assert!(gcs.optional);
    let used_if = gcs.used_if.as_ref().unwrap();
    assert_eq!(used_if.env_name, "AVATARDB_MODE");
    assert_eq!(used_if.value, "gcs");
    assert_eq!(used_if.switch_default.as_deref(), Some("gcs"));

    let bucket = find_field(&gcs.items, "AVATARDB_BUCKET_NAME").unwrap();
    assert!(bucket.required);
    assert_eq!(bucket.default, None);

    let usage = AvatarConfig::usage_with_prefix("AVATARDB").unwrap();
    assert_eq!(
        group_required(&usage, "[used when AVATARDB_MODE=gcs (default)]"),
        "no"
    );
    assert!(usage.contains("the parser does not enforce it"));

    clean_env();
    let config = AvatarConfig::with_prefix("AVATARDB").unwrap();
    assert_eq!(config.mode, StoreMode::gcs);
    assert!(config.gcs.is_none());
}

#[test]
#[serial]
fn other_enum_branch_does_not_require_unselected_group() {
    clean_env();
    env::set_var("AVATARDB_MODE", "local");
    env::set_var("AVATARDB_LOCAL_DATA_DIR", "/tmp/avatars");
    let config = AvatarConfig::with_prefix("AVATARDB").unwrap();
    assert_eq!(config.mode, StoreMode::local);
    assert!(config.gcs.is_none());
    assert_eq!(
        config.local.as_ref().map(|local| local.data_dir.as_str()),
        Some("/tmp/avatars")
    );
}

#[test]
#[serial]
fn optional_group_omit_and_partial_match_activation() {
    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct Config {
        pub db: Option<Db>,
    }

    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct Db {
        pub dsn: String,
        #[env(default = "5")]
        pub pool: u32,
    }

    let tree = Config::get_usage_tree("TEST", None).unwrap();
    let db = find_group(&tree.items, "Db").unwrap();
    assert!(db.optional);
    assert!(find_field(&db.items, "TEST_DB_DSN").unwrap().required);
    assert!(!find_field(&db.items, "TEST_DB_POOL").unwrap().required);

    let usage = Config::usage_with_prefix("TEST").unwrap();
    assert_eq!(group_required(&usage, "[Db]"), "no");

    clean_env();
    let omitted = Config::with_prefix("TEST").unwrap();
    assert!(omitted.db.is_none());

    clean_env();
    env::set_var("TEST_DB_POOL", "9");
    let err = Config::with_prefix("TEST").unwrap_err();
    assert!(matches!(err, EnvStructError::MissingEnvVar(name) if name == "TEST_DB_DSN"));

    clean_env();
    env::set_var("TEST_DB_DSN", "postgres://localhost");
    let config = Config::with_prefix("TEST").unwrap();
    assert_eq!(
        config.db,
        Some(Db {
            dsn: "postgres://localhost".to_string(),
            pool: 5,
        })
    );
}

#[test]
#[serial]
fn optional_group_defaults_do_not_create_the_group() {
    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct Config {
        pub mock: Option<MockMap>,
    }

    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct MockMap {
        #[env(default = "")]
        pub country_by_ip: String,
    }

    let tree = Config::get_usage_tree("TEST", None).unwrap();
    let mock = find_group(&tree.items, "Mock").unwrap();
    assert!(mock.optional);
    let field = find_field(&mock.items, "TEST_MOCK_COUNTRY_BY_IP").unwrap();
    assert!(!field.required);
    assert_eq!(field.default.as_deref(), Some(""));

    let usage = Config::usage_with_prefix("TEST").unwrap();
    assert_eq!(group_required(&usage, "[Mock]"), "no");
    assert!(usage.contains("defaults alone do not"));

    clean_env();
    let config = Config::with_prefix("TEST").unwrap();
    assert!(config.mock.is_none());
}

#[test]
#[serial]
fn optional_group_without_condition_keeps_path() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub doom: Doom,
    }

    #[derive(EnvStruct, Debug)]
    #[env(title = "Doom")]
    pub struct Doom {
        pub remote: Option<Remote>,
    }

    #[derive(EnvStruct, Debug)]
    pub struct Remote {
        pub dsn: String,
    }

    let usage = Config::usage_with_prefix("APP").unwrap();
    assert_eq!(group_required(&usage, "[Doom → Remote]"), "no");
    assert!(usage.contains("APP_DOOM_REMOTE_DSN"));
    assert!(!usage.contains("[used when"));
}

#[test]
#[serial]
fn used_if_does_not_weaken_parser_requirements() {
    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct Config {
        #[env(default = "false")]
        pub mock: bool,
        #[env(used_if = "mock=false")]
        pub mailgun: Mailgun,
    }

    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct Mailgun {
        pub api_key: String,
        #[env(default = "example.com")]
        pub domain: String,
    }

    let tree = Config::get_usage_tree("EMAILER", None).unwrap();
    let mailgun = find_group(&tree.items, "Mailgun").unwrap();
    assert!(!mailgun.optional);
    assert_eq!(
        mailgun.used_if.as_ref().map(|u| u.env_name.as_str()),
        Some("EMAILER_MOCK")
    );
    let api_key = find_field(&mailgun.items, "EMAILER_MAILGUN_API_KEY").unwrap();
    assert!(api_key.required);

    let usage = Config::usage_with_prefix("EMAILER").unwrap();
    assert_eq!(
        group_required(&usage, "[used when EMAILER_MOCK=false (default)]"),
        "yes"
    );
    assert!(usage.contains("the parser does not enforce it"));

    clean_env();
    env::set_var("EMAILER_MOCK", "true");
    let err = Config::with_prefix("EMAILER").unwrap_err();
    assert!(
        matches!(err, EnvStructError::MissingEnvVar(name) if name == "EMAILER_MAILGUN_API_KEY")
    );
}

#[test]
#[serial]
fn nested_flatten_keeps_group_and_env_names() {
    let tree = AvatarConfig::get_usage_tree("AVATARDB", None).unwrap();
    assert!(find_field(&tree.items, "AVATARDB_MODE").is_some());
    assert!(find_field(&tree.items, "AVATARDB_IMAGE_WIDTH").is_some());
    assert!(find_field(&tree.items, "AVATARDB_IMAGE_SIZE_LIMIT").is_some());

    let gcs = find_group(&tree.items, "GCS").unwrap();
    assert!(find_field(&gcs.items, "AVATARDB_BUCKET_NAME").is_some());
    assert!(find_field(&gcs.items, "AVATARDB_DIGEST_SALT").is_some());

    let local = find_group(&tree.items, "Local").unwrap();
    assert!(find_field(&local.items, "AVATARDB_LOCAL_DATA_DIR").is_some());

    let mock = find_group(&tree.items, "Mock").unwrap();
    assert!(mock.items.is_empty());
}

#[test]
#[serial]
fn prefix_and_rename_appear_in_used_if_env_names() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(name = "STORE_MODE", default = "gcs")]
        pub mode: StoreMode,
        #[env(title = "GCS", used_if = "mode=gcs")]
        pub gcs: Option<GcsConfig>,
    }

    let tree = Config::get_usage_tree("APP", None).unwrap();
    let gcs = find_group(&tree.items, "GCS").unwrap();
    assert_eq!(
        gcs.used_if.as_ref().map(|u| u.env_name.as_str()),
        Some("APP_STORE_MODE")
    );
    assert!(find_field(&tree.items, "APP_STORE_MODE").is_some());
    assert!(find_field(&gcs.items, "APP_GCS_BUCKET_NAME").is_some());
}

#[test]
#[serial]
fn usage_builds_without_configured_environment() {
    clean_env();
    let usage = AvatarConfig::usage_with_prefix("AVATARDB").unwrap();
    assert!(usage.contains("AVATARDB_MODE"));
    assert!(usage.contains("[used when AVATARDB_MODE=gcs (default)]"));
    assert!(usage.contains("[used when AVATARDB_MODE=local]"));
    assert!(usage.contains("[used when AVATARDB_MODE=mock]"));
}

#[test]
#[serial]
fn usage_does_not_change_or_leak_env_values() {
    let before = AvatarConfig::usage_with_prefix("AVATARDB").unwrap();
    env::set_var("AVATARDB_BUCKET_NAME", "secret-bucket");
    env::set_var("AVATARDB_DIGEST_SALT", "super-secret");
    env::set_var("AVATARDB_MODE", "local");
    let after = AvatarConfig::usage_with_prefix("AVATARDB").unwrap();
    assert_eq!(before, after);
    assert!(!after.contains("secret-bucket"));
    assert!(!after.contains("super-secret"));
}

#[test]
#[serial]
fn long_values_are_not_truncated() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(default = "https://very-long.example.com/path/that/should/remain/complete/in/usage")]
        pub url: String,
    }

    let usage = Config::usage_with_prefix("TEST").unwrap();
    let compact: String = usage.chars().filter(|c| !c.is_whitespace()).collect();
    assert!(
        compact.contains("https://very-long.example.com/path/that/should/remain/complete/in/usage")
    );
    assert!(!usage.contains('…'));
    assert!(
        usage.lines().all(|line| line == line.trim_end()),
        "usage output must not have trailing whitespace"
    );
}

#[test]
#[serial]
fn usage_snapshot_groups_and_conditions() {
    let usage = AvatarConfig::usage_with_prefix("AVATARDB").unwrap();
    insta_like_eq(
        &usage,
        r#"Environment variables

REQUIRED=yes means an explicit value is needed within the group's parsing scope.
DEFAULT is used when omitted; — = no default; "" = empty string.
All groups are shown, regardless of the current environment.
[used when NAME=value] is application usage; the parser does not enforce it.
REQUIRED on a group line is about the group: no = it may be omitted entirely,
but any explicit member activates parsing; defaults alone do not.

Byte sizes accept values such as 4MB and 10MiB.

VARIABLE                                 TYPE      REQUIRED  DEFAULT        VALUES
AVATARDB_IMAGE_SIZE_LIMIT                bytesize  no        4MB            —
AVATARDB_IMAGE_WIDTH                     integer   no        150            —
AVATARDB_NSFW_SCORE_MAX                  float     no        0.9            —
AVATARDB_MODE                            enum      no        gcs            gcs | local | mock
[used when AVATARDB_MODE=gcs (default)]            no
  AVATARDB_BUCKET_NAME                   string    yes       —              —
  AVATARDB_DIGEST_SALT                   string    no        squibblefluff  —
[used when AVATARDB_MODE=local]                    no
  AVATARDB_LOCAL_DATA_DIR                string    yes       —              —
[used when AVATARDB_MODE=mock]                     no
"#,
    );
}

#[test]
#[serial]
fn integer_range_is_shown_and_enforced() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(default = "150")]
        pub image_height: u16,
    }

    let tree = Config::get_usage_tree("TEST", None).unwrap();
    let field = find_field(&tree.items, "TEST_IMAGE_HEIGHT").unwrap();
    assert_eq!(
        field.typ.int_limit().map(|limit| limit.display()),
        Some("0..=65535".to_string())
    );

    let usage = Config::usage_with_prefix("TEST").unwrap();
    assert!(usage.contains("integer"));
    assert!(usage.contains("0..=65535"));

    clean_env();
    env::set_var("TEST_IMAGE_HEIGHT", "65536");
    let err = Config::with_prefix("TEST").unwrap_err();
    assert!(matches!(err, EnvStructError::ParseEnvError { .. }));

    env::set_var("TEST_IMAGE_HEIGHT", "-1");
    let err = Config::with_prefix("TEST").unwrap_err();
    assert!(matches!(err, EnvStructError::ParseEnvError { .. }));

    env::set_var("TEST_IMAGE_HEIGHT", "65535");
    assert_eq!(Config::with_prefix("TEST").unwrap().image_height, 65535);
    clean_env();
}

#[test]
#[serial]
fn optional_integer_keeps_its_range() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub retries: Option<u8>,
    }

    let tree = Config::get_usage_tree("TEST", None).unwrap();
    let field = find_field(&tree.items, "TEST_RETRIES").unwrap();
    assert!(!field.required);
    assert_eq!(
        field.typ.int_limit().map(|limit| limit.display()),
        Some("0..=255".to_string())
    );
}

#[test]
#[serial]
fn mode_switch_is_printed_right_above_its_variants() {
    let usage = AvatarConfig::usage_with_prefix("AVATARDB").unwrap();
    let lines: Vec<&str> = usage.lines().collect();
    let switch = lines
        .iter()
        .position(|line| line.starts_with("AVATARDB_MODE"))
        .unwrap();
    assert!(lines[switch + 1].starts_with("[used when AVATARDB_MODE=gcs"));
    // the switch is hoisted out of the alphabetical order of its own block
    let width = lines
        .iter()
        .position(|line| line.starts_with("AVATARDB_IMAGE_WIDTH"))
        .unwrap();
    assert!(width < switch);
}

#[test]
#[serial]
fn wide_integer_reports_no_bounds() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(default = "10000")]
        pub cache_size: usize,
        #[env(default = "0")]
        pub offset: i64,
    }

    let tree = Config::get_usage_tree("TEST", None).unwrap();
    assert_eq!(
        find_field(&tree.items, "TEST_CACHE_SIZE").unwrap().typ,
        UsageType::Integer(None)
    );
    assert_eq!(
        find_field(&tree.items, "TEST_OFFSET").unwrap().typ,
        UsageType::Integer(None)
    );

    let usage = Config::usage_with_prefix("TEST").unwrap();
    assert!(!usage.contains("..="));
    assert!(!usage.contains("Integer ranges"));
}

#[test]
#[serial]
fn non_zero_reports_the_zero_exclusion() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(default = "4")]
        pub workers: std::num::NonZeroUsize,
        #[env(default = "-1")]
        pub offset: std::num::NonZeroI8,
    }

    let tree = Config::get_usage_tree("TEST", None).unwrap();
    let workers = find_field(&tree.items, "TEST_WORKERS").unwrap();
    assert_eq!(
        workers.typ.int_limit().map(|limit| limit.display()),
        Some("not 0".to_string())
    );
    let offset = find_field(&tree.items, "TEST_OFFSET").unwrap();
    assert_eq!(
        offset.typ.int_limit().map(|limit| limit.display()),
        Some("-128..=127, not 0".to_string())
    );

    let usage = Config::usage_with_prefix("TEST").unwrap();
    assert!(usage.contains(r#""not 0" means the parser rejects zero."#));

    clean_env();
    env::set_var("TEST_WORKERS", "0");
    let err = Config::with_prefix("TEST").unwrap_err();
    assert!(matches!(err, EnvStructError::ParseEnvError { .. }));
    clean_env();
}

fn insta_like_eq(actual: &str, expected: &str) {
    if actual != expected {
        panic!("usage snapshot mismatch\n=== actual ===\n{actual}=== expected ===\n{expected}");
    }
}
