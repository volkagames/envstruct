use crate::*;
use prettytable::{format, Cell, Row, Table};

pub struct EnvEntry {
    pub name: String,
    pub typ: String,
    pub default: Option<String>,
}

pub trait EnvStructUsage: EnvParseNested {
    fn usage(prefix: impl AsRef<str>) -> Result<String, EnvStructError>
    where
        Self: Sized,
    {
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
                Cell::new(&entry.default.unwrap_or_default()),
            ]));
        }

        table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

        Ok(table.to_string())
    }
}

impl<T: EnvParseNested> EnvStructUsage for T {}

// https://users.rust-lang.org/t/how-can-i-convert-a-struct-name-to-a-string/66724/7
// thanks Quine Dot
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
