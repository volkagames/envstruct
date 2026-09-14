#![allow(dead_code)]

use envstruct::prelude::*;
use serial_test::*;

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

#[derive(EnvStruct, Debug, PartialEq)]
pub struct Avatars {
    pub bucket: String,
    #[env(default = "10")]
    pub max_size: u32,
}

#[derive(EnvStruct, Debug, PartialEq)]
pub struct Config {
    pub port: u16,
    pub avatars: Option<Avatars>,
}

#[test]
#[serial]
fn nested_struct_keeps_its_node_in_the_tree() {
    clean_env();

    let tree = Config::get_usage_tree("APP", None).unwrap();
    assert_eq!(tree.kind, UsageTreeKind::Struct);

    let group = tree
        .items
        .iter()
        .find_map(|item| match item {
            UsageItem::Group(group) => Some(group),
            UsageItem::Field(_) => None,
        })
        .expect("the nested struct is a group of its own");
    assert!(group.optional, "Option<Avatars> is an optional group");
    assert!(find_field(&group.items, "APP_AVATARS_BUCKET").is_some());
}

#[test]
#[serial]
fn optional_group_variables_are_marked_in_the_table() {
    clean_env();

    let usage = Config::usage_with_prefix("APP").unwrap();
    println!("usage: \n{usage}");

    let row = |name: &str| {
        usage
            .lines()
            .find(|line| line.contains(name))
            .unwrap_or_else(|| panic!("no row for {name}:\n{usage}"))
            .to_string()
    };

    // A required variable of the config itself is still required.
    assert!(!row("APP_PORT").contains("<optional group>"));
    // A variable of an optional struct is only read when the struct is configured at all.
    assert!(row("APP_AVATARS_BUCKET").contains("<optional group>"));
    // A default of its own is shown as before.
    assert!(row("APP_AVATARS_MAX_SIZE").contains(r#""10""#));
}

#[test]
#[serial]
fn env_entries_flatten_the_tree() {
    clean_env();

    let entries = Config::get_env_entries("APP", None).unwrap();
    let names: Vec<&str> = entries.iter().map(|entry| entry.name.as_str()).collect();
    assert_eq!(
        names,
        ["APP_PORT", "APP_AVATARS_BUCKET", "APP_AVATARS_MAX_SIZE"]
    );

    let max_size = entries.last().unwrap();
    assert_eq!(max_size.typ, "u32");
    assert_eq!(max_size.default.as_deref(), Some("10"));
}

#[test]
#[serial]
fn usage_builds_without_configured_environment() {
    clean_env();

    assert!(Config::usage_with_prefix("APP").is_ok());
    assert!(Config::with_prefix("APP").is_err(), "APP_PORT is required");
}

#[test]
#[serial]
fn optional_group_stays_unset_until_one_of_its_variables_is_set() {
    clean_env();
    std::env::set_var("APP_PORT", "8080");

    assert_eq!(Config::with_prefix("APP").unwrap().avatars, None);

    std::env::set_var("APP_AVATARS_BUCKET", "avatars");
    assert_eq!(
        Config::with_prefix("APP").unwrap().avatars,
        Some(Avatars {
            bucket: "avatars".to_string(),
            max_size: 10,
        })
    );
}
