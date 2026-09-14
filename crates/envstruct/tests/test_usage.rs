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

fn assert_group_marker(usage: &str, marker: &str) {
    assert!(
        usage.lines().any(|line| line == marker),
        "missing bare group marker {marker}:\n{usage}"
    );
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
fn required_field_has_no_default_and_missing_fails_parse() {
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
fn default_field_is_optional_and_shown_value_is_applied() {
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
        mode.values.as_deref(),
        Some(["gcs".to_string(), "local".to_string(), "mock".to_string()].as_slice())
    );

    let width = find_field(&tree.items, "TEST_WIDTH").unwrap();
    assert!(!width.required);
    assert_eq!(width.default.as_deref(), Some("150"));

    let usage = Config::usage_with_prefix("TEST").unwrap();
    assert!(usage.contains(r#""gcs""#));
    assert!(usage.contains(r#""150""#));
    assert!(usage.contains("enum: gcs, local, mock"));

    clean_env();
    let config = Config::with_prefix("TEST").unwrap();
    assert_eq!(config.mode, StoreMode::gcs);
    assert_eq!(config.width, 150);
}

#[test]
#[serial]
fn optional_without_default_stays_unset() {
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
    assert!(usage.contains(r#"| """#));
    assert!(usage.contains("none"));
    assert!(!field_wrap_lines(&usage, "TEST_EMPTY")[0].contains("required"));
    assert!(!field_wrap_lines(&usage, "TEST_MISSING")[0].contains("required"));

    clean_env();
    let config = Config::with_prefix("TEST").unwrap();
    assert_eq!(config.empty, "");
    assert_eq!(config.missing, None);
}

#[test]
#[serial]
fn quoted_defaults_show_literals() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(default = "false")]
        pub debug: bool,
        #[env(default = "20")]
        pub port: u16,
    }

    let usage = Config::usage_with_prefix("TEST").unwrap();
    assert!(usage.contains(r#""false""#));
    assert!(usage.contains(r#""20""#));
    assert!(usage.contains("u16 (0..=65535)"));
    assert!(!usage.contains(r#""0..=65535""#));
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
    assert_group_marker(&usage, "[used when AVATARDB_MODE=gcs (default)]");

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
    assert_group_marker(&usage, "[Db]");

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
    assert_group_marker(&usage, "[Mock]");

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
    assert_group_marker(&usage, "[Doom → Remote]");
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
    assert_group_marker(&usage, "[used when EMAILER_MOCK=false (default)]");

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
    assert!(
        !usage.contains("[used when AVATARDB_MODE=mock]"),
        "groups without variables must not leave empty sections"
    );
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
    let compact: String = usage
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '|')
        .collect();
    assert!(
        compact.contains("https://very-long.example.com/path/that/should/remain/complete/in/usage")
    );
    assert!(!usage.contains('…'));
    let lines = field_wrap_lines(&usage, "TEST_URL");
    assert!(lines.len() > 1, "long defaults must still wrap: {usage}");
    for line in lines {
        assert!(line.split(" | ").nth(2).unwrap().chars().count() <= 40);
    }
    assert!(
        usage.lines().all(|line| line == line.trim_end()),
        "usage output must not have trailing whitespace"
    );
}

fn column_pipe_positions(line: &str) -> Vec<usize> {
    let chars: Vec<char> = line.chars().collect();
    let mut pos = Vec::new();
    let mut i = 0;
    while i + 1 < chars.len() {
        let spaced = i + 2 < chars.len() && chars[i + 2] == ' ';
        let at_end = i + 2 == chars.len();
        if chars[i] == ' ' && chars[i + 1] == '|' && (spaced || at_end) {
            pos.push(i + 1);
            i += 2;
        } else {
            i += 1;
        }
    }
    pos
}

fn field_wrap_lines<'a>(usage: &'a str, name: &str) -> Vec<&'a str> {
    let mut lines = usage.lines().skip_while(|line| !line.starts_with(name));
    let mut out = Vec::new();
    if let Some(first) = lines.next() {
        out.push(first);
    }
    for line in lines {
        if line.starts_with(' ') && line.contains('|') {
            out.push(line);
        } else {
            break;
        }
    }
    out
}

fn type_cell(line: &str) -> &str {
    line.split(" |").nth(1).map(str::trim).unwrap_or(line)
}

#[test]
#[serial]
fn long_enums_wrap_inside_the_type_column() {
    #[allow(non_camel_case_types)]
    #[derive(EnvStruct, Debug, Clone, PartialEq, Eq, strum::Display, strum::EnumString)]
    enum CustomField {
        nick,
        lang,
        email,
        platform_type,
        public_id,
        player_id,
        support_id,
        refferer,
        crash_id,
        revenue,
        game_info,
        extra_id,
    }

    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub custom_fields: CustomField,
    }

    let tree = Config::get_usage_tree("TEST", None).unwrap();
    let field = find_field(&tree.items, "TEST_CUSTOM_FIELDS").unwrap();
    assert_eq!(field.values.as_ref().map(Vec::len), Some(12));

    let usage = Config::usage_with_prefix("TEST").unwrap();
    let header = usage
        .lines()
        .find(|line| line.starts_with("VARIABLE"))
        .expect("header");
    assert!(
        !header.contains("VALUES"),
        "VALUES must not be a table column:\n{usage}"
    );
    let header_pipes = column_pipe_positions(header);
    let lines = field_wrap_lines(&usage, "TEST_CUSTOM_FIELDS");
    assert!(
        lines.len() > 1,
        "12 enum values should wrap in the TYPE column:\n{usage}"
    );
    for line in &lines {
        assert_eq!(
            column_pipe_positions(line),
            header_pipes,
            "column separators must stay aligned:\n{line}\n{header}"
        );
        let typ = type_cell(line);
        assert!(
            typ.chars().count() <= 40,
            "TYPE fragment longer than wrap width ({typ:?}):\n{line}"
        );
        assert!(
            !typ.contains('|'),
            "values must not look like column separators: {typ:?}"
        );
    }
    assert!(usage.contains("enum: nick, lang"));
}

#[test]
#[serial]
fn map_keys_are_shown_in_the_type_column() {
    #[derive(Debug, PartialEq)]
    struct CustomFields(std::collections::HashMap<String, u64>);

    impl EnvParsePrimitive for CustomFields {
        fn parse(val: &str) -> Result<Self, BoxError> {
            Ok(Self(std::collections::HashMap::<String, u64>::parse(val)?))
        }

        fn usage_type() -> UsageType {
            UsageType::Map(
                Box::new(UsageType::String),
                Box::new(UsageType::Integer("u64".to_string(), None)),
            )
        }

        fn usage_values() -> Option<Vec<String>> {
            Some(
                [
                    "nick",
                    "lang",
                    "email",
                    "platform_type",
                    "public_id",
                    "player_id",
                    "support_id",
                    "refferer",
                    "crash_id",
                    "revenue",
                    "game_info",
                ]
                .into_iter()
                .map(str::to_string)
                .collect(),
            )
        }
    }

    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub custom_fields: CustomFields,
    }

    let usage = Config::usage_with_prefix("ZENDESK").unwrap();
    let header = usage
        .lines()
        .find(|line| line.starts_with("VARIABLE"))
        .expect("header");
    let lines = field_wrap_lines(&usage, "ZENDESK_CUSTOM_FIELDS");
    assert!(lines.len() > 1, "11 map keys should wrap:\n{usage}");
    for line in &lines {
        assert_eq!(column_pipe_positions(line), column_pipe_positions(header));
        assert_eq!(line.matches('|').count(), 2);
        assert!(type_cell(line).chars().count() <= 40);
    }
    let typ = lines
        .iter()
        .map(|line| type_cell(line))
        .collect::<Vec<_>>()
        .join(", ");
    assert_eq!(
        typ,
        "map<string,u64> (keys: nick, lang, email, platform_type, public_id, player_id, support_id, refferer, crash_id, revenue, game_info)"
    );
}

#[test]
#[serial]
fn list_items_are_distinct_from_scalar_choices() {
    #[derive(Debug)]
    struct Items(Vec<String>);

    impl EnvParsePrimitive for Items {
        fn parse(val: &str) -> Result<Self, BoxError> {
            Ok(Self(Vec::<String>::parse(val)?))
        }

        fn usage_type() -> UsageType {
            UsageType::List(Box::new(UsageType::String))
        }

        fn usage_values() -> Option<Vec<String>> {
            Some(vec!["nick".to_string(), "lang".to_string()])
        }
    }

    #[derive(EnvStruct, Debug)]
    struct Config {
        items: Items,
        mode: StoreMode,
    }

    let usage = Config::usage().unwrap();
    assert!(usage.contains("list<string> (items: nick, lang)"));
    assert!(usage.contains("enum: gcs, local, mock"));
}

#[test]
#[serial]
fn usage_snapshot_groups_and_conditions() {
    let usage = AvatarConfig::usage_with_prefix("AVATARDB").unwrap();
    insta_like_eq(
        &usage,
        r#"Environment variables

VARIABLE                                | TYPE                   | DEFAULT
----------------------------------------+------------------------+----------------
AVATARDB_IMAGE_SIZE_LIMIT               | bytesize               | "4MB"
AVATARDB_IMAGE_WIDTH                    | u32                    | "150"
AVATARDB_NSFW_SCORE_MAX                 | f64                    | "0.9"
AVATARDB_MODE                           | enum: gcs, local, mock | "gcs"
[used when AVATARDB_MODE=gcs (default)]
  AVATARDB_BUCKET_NAME                  | string                 | <required>
  AVATARDB_DIGEST_SALT                  | string                 | "squibblefluff"
[used when AVATARDB_MODE=local]
  AVATARDB_LOCAL_DATA_DIR               | string                 | <required>
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
    assert_eq!(field.typ.display(), "u16");
    assert!(usage.contains("u16 (0..=65535)"));
    assert!(!usage.contains("integer"));

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
    assert_eq!(field.typ.display(), "u8");
    assert_eq!(
        field.typ.int_limit().map(|limit| limit.display()),
        Some("0..=255".to_string())
    );
}

#[test]
#[serial]
fn nested_struct_rows_sort_with_sibling_leaves() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(default = "false")]
        pub skip_worker: bool,
        pub public_csrf: Csrf,
        pub account_session: Session,
        pub account_session_refresh_ttl: String,
    }

    #[derive(EnvStruct, Debug)]
    pub struct Csrf {
        #[env(default = "_csrf")]
        pub cookie_name: String,
    }

    #[derive(EnvStruct, Debug)]
    pub struct Session {
        pub cookie_name: String,
        pub gcm_secret: String,
    }

    let usage = Config::usage_with_prefix("APP").unwrap();
    let names: Vec<&str> = usage
        .lines()
        .filter_map(|line| {
            line.split(" | ")
                .next()
                .map(str::trim)
                .filter(|name| name.starts_with("APP_"))
        })
        .collect();
    let pos = |name: &str| {
        names
            .iter()
            .position(|n| *n == name)
            .unwrap_or_else(|| panic!("{name} missing in {names:?}"))
    };
    let cookie = pos("APP_ACCOUNT_SESSION_COOKIE_NAME");
    let secret = pos("APP_ACCOUNT_SESSION_GCM_SECRET");
    let refresh = pos("APP_ACCOUNT_SESSION_REFRESH_TTL");
    let csrf = pos("APP_PUBLIC_CSRF_COOKIE_NAME");
    let skip = pos("APP_SKIP_WORKER");
    assert!(
        cookie < secret && secret < refresh && refresh + 1 == csrf && csrf + 1 == skip,
        "nested structs should sort with sibling leaves by env name, got {names:?}"
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
        find_field(&tree.items, "TEST_CACHE_SIZE")
            .unwrap()
            .typ
            .display(),
        "usize"
    );
    assert_eq!(
        find_field(&tree.items, "TEST_OFFSET")
            .unwrap()
            .typ
            .display(),
        "i64"
    );

    let usage = Config::usage_with_prefix("TEST").unwrap();
    assert!(usage.contains("usize"));
    assert!(usage.contains("i64"));
    assert!(!usage.contains("integer"));
    assert!(!usage.contains("..="));
}

#[test]
#[serial]
fn numeric_usage_prints_the_rust_type() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub pool_size: u32,
        pub score: f64,
        pub ratio: f32,
        pub data_dir: std::path::PathBuf,
        pub timeout_secs: std::time::Duration,
        #[env(default = "30s")]
        pub idle_timeout: envstruct::Duration,
    }

    let tree = Config::get_usage_tree("TEST", None).unwrap();
    assert_eq!(
        find_field(&tree.items, "TEST_POOL_SIZE")
            .unwrap()
            .typ
            .display(),
        "u32"
    );
    assert_eq!(
        find_field(&tree.items, "TEST_SCORE").unwrap().typ.display(),
        "f64"
    );
    assert_eq!(
        find_field(&tree.items, "TEST_RATIO").unwrap().typ.display(),
        "f32"
    );
    assert_eq!(
        find_field(&tree.items, "TEST_DATA_DIR")
            .unwrap()
            .typ
            .display(),
        "path"
    );
    assert_eq!(
        find_field(&tree.items, "TEST_TIMEOUT_SECS")
            .unwrap()
            .typ
            .display(),
        "seconds"
    );
    assert_eq!(
        find_field(&tree.items, "TEST_IDLE_TIMEOUT")
            .unwrap()
            .typ
            .display(),
        "duration"
    );

    let usage = Config::usage_with_prefix("TEST").unwrap();
    assert!(!usage.contains("integer"));
    assert!(!usage.contains("float"));
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
    assert_eq!(workers.typ.display(), "NonZeroUsize");
    assert_eq!(
        workers.typ.int_limit().map(|limit| limit.display()),
        Some("not 0".to_string())
    );
    let offset = find_field(&tree.items, "TEST_OFFSET").unwrap();
    assert_eq!(offset.typ.display(), "NonZeroI8");
    assert_eq!(
        offset.typ.int_limit().map(|limit| limit.display()),
        Some("-128..=127, not 0".to_string())
    );

    let usage = Config::usage_with_prefix("TEST").unwrap();
    assert!(usage.contains("NonZeroUsize"));
    assert!(usage.contains("NonZeroI8 (-128..=127)"));
    assert!(!usage.contains("not 0"));

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

#[test]
#[serial]
fn secret_is_usage_metadata_and_does_not_change_parse() {
    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct Config {
        #[env(secret)]
        pub dsn: String,
        pub port: u16,
    }

    let tree = Config::get_usage_tree("APP", None).unwrap();
    let dsn = find_field(&tree.items, "APP_DSN").unwrap();
    let port = find_field(&tree.items, "APP_PORT").unwrap();
    assert!(dsn.secret);
    assert!(!port.secret);

    let usage = Config::usage_with_prefix("APP").unwrap();
    assert!(usage.contains("APP_DSN (secret)"));
    assert!(!usage.contains("APP_PORT (secret)"));

    clean_env();
    let err = Config::with_prefix("APP").unwrap_err();
    assert!(matches!(err, EnvStructError::MissingEnvVar(name) if name == "APP_DSN"));

    env::set_var("APP_DSN", "postgres://localhost");
    env::set_var("APP_PORT", "5432");
    assert_eq!(
        Config::with_prefix("APP").unwrap(),
        Config {
            dsn: "postgres://localhost".to_string(),
            port: 5432,
        }
    );
}

#[test]
#[serial]
fn secret_on_nested_struct_marks_descendant_fields() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(secret)]
        pub session: Session,
        pub name: String,
    }

    #[derive(EnvStruct, Debug)]
    pub struct Session {
        pub cookie: String,
        pub gcm_secret: String,
    }

    let tree = Config::get_usage_tree("APP", None).unwrap();
    assert!(
        find_field(&tree.items, "APP_SESSION_COOKIE")
            .unwrap()
            .secret
    );
    assert!(
        find_field(&tree.items, "APP_SESSION_GCM_SECRET")
            .unwrap()
            .secret
    );
    assert!(!find_field(&tree.items, "APP_NAME").unwrap().secret);
}

#[test]
#[serial]
fn default_note_is_shown_in_parentheses_and_does_not_parse() {
    #[derive(EnvStruct, Debug, PartialEq)]
    pub struct Config {
        #[env(default_note = "physical CPU count")]
        pub worker_count: Option<usize>,
    }

    let tree = Config::get_usage_tree("APP", None).unwrap();
    let field = find_field(&tree.items, "APP_WORKER_COUNT").unwrap();
    assert!(!field.required);
    assert_eq!(field.default, None);
    assert_eq!(field.default_note.as_deref(), Some("physical CPU count"));

    let usage = Config::usage_with_prefix("APP").unwrap();
    assert!(usage.contains("(physical CPU count)"));
    assert!(!usage.contains("\"physical CPU count\""));
    let row = field_wrap_lines(&usage, "APP_WORKER_COUNT")[0];
    assert!(!row.contains("none"));

    clean_env();
    assert_eq!(
        Config::with_prefix("APP").unwrap(),
        Config { worker_count: None }
    );
}

#[test]
#[serial]
fn optional_and_required_defaults_are_readable_without_a_legend() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub dsn: String,
        pub app_name: Option<String>,
    }

    let usage = Config::usage_with_prefix("APP").unwrap();
    assert!(!usage.contains("must be set."));
    assert!(!usage.contains("optional, unset by default."));
    let header = usage
        .lines()
        .find(|line| line.starts_with("VARIABLE"))
        .unwrap();
    assert_eq!(
        header.split(" | ").map(str::trim).collect::<Vec<_>>(),
        ["VARIABLE", "TYPE", "DEFAULT"]
    );
    let dsn = usage
        .lines()
        .find(|line| line.starts_with("APP_DSN"))
        .expect("dsn row");
    let app_name = usage
        .lines()
        .find(|line| line.starts_with("APP_APP_NAME"))
        .expect("app_name row");
    assert_eq!(dsn.split(" | ").nth(2).unwrap().trim(), "<required>");
    assert_eq!(app_name.split(" | ").nth(2).unwrap().trim(), "none");
}

#[test]
#[serial]
fn required_field_with_default_note_still_marks_the_value_as_required() {
    #[derive(EnvStruct, Debug)]
    struct Config {
        #[env(default_note = "physical CPU count")]
        worker_count: usize,
    }

    let usage = Config::usage_with_prefix("APP").unwrap();
    let row = field_wrap_lines(&usage, "APP_WORKER_COUNT")[0];
    assert_eq!(
        row.split(" | ").nth(2).unwrap().trim(),
        "<required> (physical CPU count)"
    );

    clean_env();
    assert!(
        matches!(Config::with_prefix("APP"), Err(EnvStructError::MissingEnvVar(name)) if name == "APP_WORKER_COUNT")
    );
}

#[test]
#[serial]
fn usage_snapshot_secret_default_note_and_none() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        #[env(secret)]
        pub dsn: String,
        #[env(default_note = "physical CPU count")]
        pub worker_count: Option<usize>,
        pub app_name: Option<String>,
        #[env(default = "8080")]
        pub port: u16,
    }

    let usage = Config::usage_with_prefix("APP").unwrap();
    insta_like_eq(
        &usage,
        r#"Environment variables

VARIABLE         | TYPE            | DEFAULT
-----------------+-----------------+---------------------
APP_APP_NAME     | string          | none
APP_DSN (secret) | string          | <required>
APP_PORT         | u16 (0..=65535) | "8080"
APP_WORKER_COUNT | usize           | (physical CPU count)
"#,
    );
}

#[test]
#[serial]
fn usage_snapshot_escapes_control_chars_in_default() {
    #[derive(EnvStruct, Debug)]
    pub struct Config {
        pub first: String,
        #[env(default = "a\tb")]
        pub second: String,
    }

    let usage = Config::usage_with_prefix("APP").unwrap();
    insta_like_eq(
        &usage,
        r#"Environment variables

VARIABLE   | TYPE   | DEFAULT
-----------+--------+-----------
APP_FIRST  | string | <required>
APP_SECOND | string | "a\tb"
"#,
    );
}
