use crate::*;

const HEADERS: [&str; 3] = ["NAME", "TYPE", "DEFAULT"];

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
        let mut rows = vec![HEADERS.map(String::from)];
        for entry in Self::get_env_entries(prefix, None)? {
            rows.push([
                entry.name,
                strip_namespace(&entry.typ),
                // quote all default values to distinct it from empty strings
                entry
                    .default
                    .map(|v| format!(r#""{v}""#))
                    .unwrap_or_default(),
            ]);
        }

        Ok(render_table(&rows))
    }
}

impl<T: EnvParseNested> EnvStructUsage for T {}

/// Renders rows as a bordless table: cells padded with one space on each side,
/// joined by `|`, with a `-`/`+` separator under the header row. Trailing
/// whitespace is never emitted, so the last column is not right-padded.
fn render_table(rows: &[[String; HEADERS.len()]]) -> String {
    let mut widths = [0usize; HEADERS.len()];
    for row in rows {
        for (width, cell) in widths.iter_mut().zip(row) {
            *width = (*width).max(cell.chars().count());
        }
    }

    let mut out = String::new();
    for (index, row) in rows.iter().enumerate() {
        let line = row
            .iter()
            .zip(widths)
            // padded by char count, not byte length, to keep non-ASCII cells aligned
            .map(|(cell, width)| {
                let padding = " ".repeat(width - cell.chars().count());
                format!(" {cell}{padding} ")
            })
            .collect::<Vec<_>>()
            .join("|");
        out.push_str(line.trim_end());
        out.push('\n');

        if index == 0 {
            let separator = widths
                .iter()
                .map(|width| "-".repeat(width + 2))
                .collect::<Vec<_>>()
                .join("+");
            out.push_str(&separator);
            out.push('\n');
        }
    }
    out
}

#[test]
fn test_render_table() {
    let row = |cells: [&str; 3]| cells.map(String::from);

    // column widths come from the widest cell, header included; the empty
    // trailing cell leaves the line ending right after the separator
    assert_eq!(
        render_table(&[
            row(HEADERS),
            row(["TEST_VALUE1", "String", r#""default value""#]),
            row(["TEST_V2", "BTreeMap<String, i64>", ""]),
        ]),
        concat!(
            " NAME        | TYPE                  | DEFAULT\n",
            "-------------+-----------------------+-----------------\n",
            " TEST_VALUE1 | String                | \"default value\"\n",
            " TEST_V2     | BTreeMap<String, i64> |\n",
        )
    );

    // header-only table still emits the separator sized by the headers
    assert_eq!(
        render_table(&[row(HEADERS)]),
        " NAME | TYPE | DEFAULT\n------+------+---------\n"
    );

    // padding counts characters, not bytes, so multi-byte cells stay aligned
    assert_eq!(
        render_table(&[row(["ключ", "тип", "x"]), row(["a", "b", "y"])]),
        " ключ | тип | x\n------+-----+---\n a    | b   | y\n"
    );
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
