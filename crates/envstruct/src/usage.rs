use crate::*;
use prettytable::{format, Cell, Row, Table};

/// Represents an environment variable entry with its name, type, and optional default value.
pub struct EnvEntry {
    pub name: String,
    pub typ: String,
    pub default: Option<String>,
}

/// A trait for generating usage information for environment variables.
pub trait EnvStructUsage: EnvParseNested {
    /// Generates a usage table for environment variables without any prefix.
    fn usage() -> Result<String, EnvStructError> {
        Self::usage_with_prefix("")
    }

    /// Generates a usage table for environment variables with the given prefix.
    ///
    /// # Arguments
    ///
    /// * `prefix` - A string slice that holds the prefix to be used for the environment variables.
    fn usage_with_prefix(prefix: impl AsRef<str>) -> Result<String, EnvStructError> {
        let mut table = Table::new();
        table.set_titles(Row::new(vec![
            Cell::new("NAME"),
            Cell::new("TYPE"),
            Cell::new("DEFAULT"),
        ]));

        for entry in Self::get_env_entries(prefix, None)? {
            table.add_row(Row::new(vec![
                Cell::new(&entry.name),
                Cell::new(&strip_namespace(&entry.typ)),
                Cell::new(
                    // quote all default values to distinct it from empty strings
                    &entry
                        .default
                        .map(|v| format!(r#""{v}""#))
                        .unwrap_or_default(),
                ),
            ]));
        }

        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

        Ok(table.to_string())
    }
}

impl<T: EnvParseNested> EnvStructUsage for T {}

/// Strips the namespace from a type name, leaving only the base type.
///
/// # Arguments
///
/// * `name` - A string slice that holds the fully qualified type name.
///
/// # Returns
///
/// A `String` containing the base type name without the namespace.
fn strip_namespace(name: &str) -> String {
    // Off the top of my head
    static SPLITTERS: &[char] = &[
        '(', ')', '[', ']', '<', '>', '{', '}',
        // EDIT: ' ' for `Foo as Bar`, ',' for tuples and
        // `Fn` args, `=` for `dyn` associated types, ...
        ' ', ',', '=',
    ];
    name
        // Split into substrings but preserve the delimiters, and...
        .split_inclusive(SPLITTERS)
        // ...for each substring...
        .flat_map(|component| {
            // ...return the portion after the last "::"
            // (or the entire substring, if there is no "::")...
            component.rsplit("::").next()
        })
        // ...and collect into a `String`
        .collect()
}

#[test]
fn test_strip_namespace() {
    let types = vec![
       ( "String",  "String"),
       ( "i32", "i32"),
       ( "alloc::string::String", "String"),
       ( "primitive_types::Point", "Point"),
       ( "alloc::vec::Vec<alloc::string::String>", "Vec<String>"),
       ( "alloc::vec::Vec<i32>", "Vec<i32>"),
       ( "std::collections::hash::map::HashMap<alloc::string::String, alloc::string::String>", "HashMap<String, String>"),
       ( "alloc::vec::Vec<std::collections::hash::map::HashMap<alloc::string::String, alloc::vec::Vec<i32>>>", "Vec<HashMap<String, Vec<i32>>>"),
       ( "alloc::string::String", "String"),
       ( "std::path::PathBuf", "PathBuf"),
    ];

    for (typ, _expected) in &types {
        println!("{}", strip_namespace(typ));
    }

    for (typ, expected) in types {
        assert_eq!(strip_namespace(typ), expected);
    }
}
