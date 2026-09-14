use crate::*;
use prettytable::{format, Cell, Row, Table};

/// Represents an environment variable entry with its name, type, and optional default value.
pub struct EnvEntry {
    pub name: String,
    pub typ: String,
    pub default: Option<String>,
}

/// Kind of a usage tree node produced by a type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsageTreeKind {
    Leaf,
    Struct,
    /// A nested struct behind an `Option`, which the parser reads only when one of its
    /// variables is set.
    OptionalStruct,
}

/// A field or nested group in a usage tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UsageItem {
    Field(UsageField),
    Group(UsageGroup),
}

/// One environment variable as shown in a usage table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageField {
    pub name: String,
    pub typ: String,
    pub default: Option<String>,
}

/// A group of fields and nested groups produced by one nested struct.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageGroup {
    pub optional: bool,
    pub items: Vec<UsageItem>,
}

/// Usage metadata for a config type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageTree {
    pub kind: UsageTreeKind,
    pub items: Vec<UsageItem>,
}

impl UsageTree {
    /// Tree of a type that parses from a single variable.
    pub fn leaf_field(
        name: impl Into<String>,
        typ: impl Into<String>,
        default: Option<String>,
    ) -> Self {
        Self {
            kind: UsageTreeKind::Leaf,
            items: vec![UsageItem::Field(UsageField {
                name: name.into(),
                typ: typ.into(),
                default,
            })],
        }
    }

    /// Every variable of the tree, in declaration order, without its grouping.
    pub fn flatten_entries(&self) -> Vec<EnvEntry> {
        let mut entries = Vec::new();
        flatten_items(&self.items, &mut entries);
        entries
    }
}

fn flatten_items(items: &[UsageItem], entries: &mut Vec<EnvEntry>) {
    for item in items {
        match item {
            UsageItem::Field(field) => entries.push(EnvEntry {
                name: field.name.clone(),
                typ: field.typ.clone(),
                default: field.default.clone(),
            }),
            UsageItem::Group(group) => flatten_items(&group.items, entries),
        }
    }
}

/// Wraps a field's usage tree as parent-group items. A nested struct keeps its own node, so
/// that the table can tell the variables of an optional struct from the ones always read.
pub fn attach_field_usage(tree: UsageTree) -> Vec<UsageItem> {
    match tree.kind {
        UsageTreeKind::Leaf => tree.items,
        UsageTreeKind::Struct | UsageTreeKind::OptionalStruct => {
            vec![UsageItem::Group(UsageGroup {
                optional: tree.kind == UsageTreeKind::OptionalStruct,
                items: tree.items,
            })]
        }
    }
}

/// Shown instead of a default for a variable of an optional struct, which is only read when
/// the struct is configured at all.
const OPTIONAL_GROUP: &str = "<optional group>";

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

        let tree = Self::get_usage_tree(prefix, None)?;
        for row in table_rows(&tree.items, false) {
            table.add_row(row);
        }

        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

        // prettytable's FORMAT_NO_BORDER_LINE_SEPARATOR still writes right padding
        // (rp == 1) after skip_r_fill, so every line ends with trailing space(s).
        Ok(table
            .to_string()
            .lines()
            .map(str::trim_end)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n")
    }
}

impl<T: EnvParseNested> EnvStructUsage for T {}

/// One table row per variable, carrying down whether the group it belongs to is optional.
fn table_rows(items: &[UsageItem], optional: bool) -> Vec<Row> {
    let mut rows = Vec::new();
    for item in items {
        match item {
            UsageItem::Field(field) => rows.push(Row::new(vec![
                Cell::new(&field.name),
                Cell::new(&strip_namespace(&field.typ)),
                Cell::new(&default_cell(field, optional)),
            ])),
            UsageItem::Group(group) => {
                rows.extend(table_rows(&group.items, optional || group.optional))
            }
        }
    }
    rows
}

fn default_cell(field: &UsageField, optional: bool) -> String {
    match &field.default {
        // quote all default values to distinct it from empty strings
        Some(value) => format!(r#""{value}""#),
        None if optional => OPTIONAL_GROUP.to_string(),
        None => String::new(),
    }
}

/// Strips the namespace from a type name, leaving only the base type.
///
/// # Arguments
///
/// * `name` - A string slice that holds the fully qualified type name.
///
/// # Returns
///
/// A `String` containing the base type name without the namespace.
pub fn strip_namespace(name: &str) -> String {
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
