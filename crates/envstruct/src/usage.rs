use crate::*;
use std::fmt::Write as _;

/// Represents an environment variable entry with its name, type, and optional default value.
pub struct EnvEntry {
    pub name: String,
    pub typ: String,
    pub default: Option<String>,
}

/// What the parser accepts for a numeric type, when it is worth showing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IntLimit {
    /// Inclusive bounds, narrow enough that a real value can reach them.
    Range {
        min: String,
        max: String,
        excludes_zero: bool,
    },
    /// Bounds too wide to carry information, but zero is rejected.
    NonZero,
}

impl IntLimit {
    pub fn display(&self) -> String {
        match self {
            Self::Range {
                min,
                max,
                excludes_zero,
            } => {
                let bounds = format!("{min}..={max}");
                if *excludes_zero {
                    format!("{bounds}, not 0")
                } else {
                    bounds
                }
            }
            Self::NonZero => "not 0".to_string(),
        }
    }

    fn excludes_zero(&self) -> bool {
        match self {
            Self::Range { excludes_zero, .. } => *excludes_zero,
            Self::NonZero => true,
        }
    }

    fn is_range(&self) -> bool {
        matches!(self, Self::Range { .. })
    }
}

/// Human-facing value format for usage output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UsageType {
    String,
    Integer(Option<IntLimit>),
    Float,
    Bool,
    Duration,
    ByteSize,
    Url,
    Enum,
    List(Box<UsageType>),
    Map(Box<UsageType>, Box<UsageType>),
    Other(String),
}

impl UsageType {
    pub fn display(&self) -> String {
        match self {
            Self::String => "string".to_string(),
            Self::Integer(_) => "integer".to_string(),
            Self::Float => "float".to_string(),
            Self::Bool => "bool".to_string(),
            Self::Duration => "duration".to_string(),
            Self::ByteSize => "bytesize".to_string(),
            Self::Url => "url".to_string(),
            Self::Enum => "enum".to_string(),
            Self::List(inner) => format!("list<{}>", inner.display()),
            Self::Map(key, value) => format!("map<{},{}>", key.display(), value.display()),
            Self::Other(name) => name.clone(),
        }
    }

    /// Parser limits of this type, when it is a numeric type worth constraining.
    pub fn int_limit(&self) -> Option<&IntLimit> {
        match self {
            Self::Integer(limit) => limit.as_ref(),
            _ => None,
        }
    }

    fn uses_list(&self) -> bool {
        match self {
            Self::List(_) => true,
            Self::Map(key, value) => key.uses_list() || value.uses_list(),
            _ => false,
        }
    }

    fn uses_map(&self) -> bool {
        match self {
            Self::Map(_, _) => true,
            Self::List(inner) => inner.uses_map(),
            _ => false,
        }
    }

    fn uses_duration(&self) -> bool {
        match self {
            Self::Duration => true,
            Self::List(inner) => inner.uses_duration(),
            Self::Map(key, value) => key.uses_duration() || value.uses_duration(),
            _ => false,
        }
    }

    fn uses_bytesize(&self) -> bool {
        match self {
            Self::ByteSize => true,
            Self::List(inner) => inner.uses_bytesize(),
            Self::Map(key, value) => key.uses_bytesize() || value.uses_bytesize(),
            _ => false,
        }
    }

    fn uses_set_other(&self) -> bool {
        match self {
            Self::Other(name) => name.starts_with("set<"),
            Self::List(inner) => inner.uses_set_other(),
            Self::Map(key, value) => key.uses_set_other() || value.uses_set_other(),
            _ => false,
        }
    }
}

/// Kind of a usage tree node produced by a type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UsageTreeKind {
    Leaf,
    Struct,
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
    pub typ: UsageType,
    pub required: bool,
    pub default: Option<String>,
    pub values: Option<Vec<String>>,
}

/// A named group of fields and nested groups.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageGroup {
    pub title: String,
    pub optional: bool,
    pub used_if: Option<UsageUsedIf>,
    pub items: Vec<UsageItem>,
}

/// Application-usage condition. The parser does not enforce it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageUsedIf {
    pub env_name: String,
    pub value: String,
    pub switch_default: Option<String>,
}

/// Usage metadata for a config type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageTree {
    pub title: Option<String>,
    pub kind: UsageTreeKind,
    pub items: Vec<UsageItem>,
}

impl UsageTree {
    pub fn leaf_field(
        name: impl Into<String>,
        typ: UsageType,
        required: bool,
        default: Option<String>,
        values: Option<Vec<String>>,
    ) -> Self {
        Self {
            title: None,
            kind: UsageTreeKind::Leaf,
            items: vec![UsageItem::Field(UsageField {
                name: name.into(),
                typ,
                required,
                default,
                values,
            })],
        }
    }

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
                typ: field.typ.display(),
                default: field.default.clone(),
            }),
            UsageItem::Group(group) => flatten_items(&group.items, entries),
        }
    }
}

/// Field-level metadata supplied by the derive macro when attaching a nested type.
pub struct FieldUsageMeta {
    pub field_name: &'static str,
    pub title: Option<String>,
    pub flatten: bool,
    pub inline: bool,
    pub used_if: Option<UsageUsedIf>,
}

/// Wraps a field's usage tree as parent-group items.
pub fn attach_field_usage(tree: UsageTree, meta: FieldUsageMeta) -> Vec<UsageItem> {
    match tree.kind {
        UsageTreeKind::Leaf => tree.items,
        UsageTreeKind::Struct | UsageTreeKind::OptionalStruct => {
            if should_inline(&tree, &meta) {
                tree.items
            } else {
                vec![UsageItem::Group(UsageGroup {
                    title: choose_title(&tree, &meta),
                    optional: tree.kind == UsageTreeKind::OptionalStruct,
                    used_if: meta.used_if,
                    items: tree.items,
                })]
            }
        }
    }
}

fn should_inline(tree: &UsageTree, meta: &FieldUsageMeta) -> bool {
    if meta.inline {
        return true;
    }
    meta.flatten
        && tree.kind == UsageTreeKind::Struct
        && meta.title.is_none()
        && meta.used_if.is_none()
}

fn choose_title(tree: &UsageTree, meta: &FieldUsageMeta) -> String {
    if let Some(title) = &meta.title {
        return title.clone();
    }
    if let Some(title) = &tree.title {
        return title.clone();
    }
    let fallback = fallback_title(meta.field_name);
    if fallback.is_empty() {
        "Group".to_string()
    } else {
        fallback
    }
}

fn fallback_title(name: &str) -> String {
    name.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A trait for generating usage information for environment variables.
pub trait EnvStructUsage: EnvParseNested {
    /// Generates usage text for environment variables without any prefix.
    fn usage() -> Result<String, EnvStructError> {
        Self::usage_with_prefix("")
    }

    /// Generates usage text for environment variables with the given prefix.
    fn usage_with_prefix(prefix: impl AsRef<str>) -> Result<String, EnvStructError> {
        Ok(render_usage(&Self::get_usage_tree(prefix, None)?))
    }
}

impl<T: EnvParseNested> EnvStructUsage for T {}

const NO_DEFAULT: &str = "—";
const WRAP_WIDTH: usize = 40;
/// Rows of a conditional group are indented under its marker line.
const INDENT: &str = "  ";

enum UsageBlock {
    Fields(Vec<UsageField>),
    Section {
        marker: String,
        /// Whether the group itself must be present, shown in the REQUIRED column.
        required: bool,
        fields: Vec<UsageField>,
    },
}

fn render_usage(tree: &UsageTree) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Environment variables");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "REQUIRED=yes means an explicit value is needed within the group's parsing scope."
    );
    let _ = writeln!(
        out,
        "DEFAULT is used when omitted; {NO_DEFAULT} = no default; \"\" = empty string."
    );
    let _ = writeln!(
        out,
        "All groups are shown, regardless of the current environment."
    );

    let (blocks, has_optional, has_used_if) = collect_blocks(tree);
    if has_used_if {
        let _ = writeln!(
            out,
            "[used when NAME=value] is application usage; the parser does not enforce it."
        );
    }
    if has_optional {
        let _ = writeln!(
            out,
            "REQUIRED on a group line is about the group: no = it may be omitted entirely,"
        );
        let _ = writeln!(
            out,
            "but any explicit member activates parsing; defaults alone do not."
        );
    }

    let syntax = syntax_notes(tree);
    if !syntax.is_empty() {
        let _ = writeln!(out);
        for line in syntax {
            let _ = writeln!(out, "{line}");
        }
    }

    if !blocks.is_empty() {
        let _ = writeln!(out);
        render_blocks(&mut out, &blocks);
    }

    normalize_output(&out)
}

fn split_items(items: &[UsageItem]) -> (Vec<UsageField>, Vec<UsageGroup>) {
    let mut fields = Vec::new();
    let mut groups = Vec::new();
    for item in items {
        match item {
            UsageItem::Field(field) => fields.push(field.clone()),
            UsageItem::Group(group) => groups.push(group.clone()),
        }
    }
    fields.sort_by(|a, b| a.name.cmp(&b.name));
    // A mode switch reads better right above the variants it selects.
    let switches: Vec<&str> = groups
        .iter()
        .filter_map(|group| group.used_if.as_ref())
        .map(|used_if| used_if.env_name.as_str())
        .collect();
    let (switch_fields, plain_fields): (Vec<UsageField>, Vec<UsageField>) = fields
        .into_iter()
        .partition(|field| switches.contains(&field.name.as_str()));
    let fields = [plain_fields, switch_fields].concat();
    (fields, groups)
}

fn collect_blocks(tree: &UsageTree) -> (Vec<UsageBlock>, bool, bool) {
    let mut blocks = Vec::new();
    let mut has_optional = false;
    let mut has_used_if = false;
    let (fields, groups) = split_items(&tree.items);
    if !fields.is_empty() {
        blocks.push(UsageBlock::Fields(fields));
    }
    for group in &groups {
        collect_group(group, "", &mut blocks, &mut has_optional, &mut has_used_if);
    }
    (blocks, has_optional, has_used_if)
}

fn collect_group(
    group: &UsageGroup,
    parent_path: &str,
    blocks: &mut Vec<UsageBlock>,
    has_optional: &mut bool,
    has_used_if: &mut bool,
) {
    let path = group_path(parent_path, &group.title);
    let (fields, children) = split_items(&group.items);
    *has_optional |= group.optional;
    *has_used_if |= group.used_if.is_some();
    if group.optional || group.used_if.is_some() {
        blocks.push(UsageBlock::Section {
            marker: section_marker(group, &path),
            required: !group.optional,
            fields,
        });
    } else if !fields.is_empty() {
        blocks.push(UsageBlock::Fields(fields));
    }
    for child in &children {
        collect_group(child, &path, blocks, has_optional, has_used_if);
    }
}

fn group_path(parent_path: &str, title: &str) -> String {
    if parent_path.is_empty() {
        title.to_string()
    } else {
        format!("{parent_path} → {title}")
    }
}

fn section_marker(group: &UsageGroup, path: &str) -> String {
    let mut parts = Vec::new();
    if let Some(used_if) = &group.used_if {
        let mut cond = format!("used when {}={}", used_if.env_name, used_if.value);
        if used_if.switch_default.as_deref() == Some(used_if.value.as_str()) {
            cond.push_str(" (default)");
        }
        parts.push(cond);
    } else {
        parts.push(path.to_string());
    }
    format!("[{}]", parts.join("; "))
}

fn render_blocks(out: &mut String, blocks: &[UsageBlock]) {
    let rows = table_rows(blocks);
    let has_values = rows
        .iter()
        .any(|(field, _)| field.values.is_some() || field.typ.int_limit().is_some());
    let markers: Vec<&str> = blocks
        .iter()
        .filter_map(|block| match block {
            UsageBlock::Section { marker, .. } => Some(marker.as_str()),
            UsageBlock::Fields(_) => None,
        })
        .collect();
    let widths = column_widths(&rows, &markers, has_values);
    if !rows.is_empty() {
        out.push_str(&format_row(&header_cols(has_values), &widths));
        let _ = writeln!(out);
    }

    // The table is printed as one continuous block: markers and name prefixes carry
    // the grouping, blank lines are not used inside it.
    for block in blocks {
        let (fields, indent) = match block {
            UsageBlock::Fields(fields) => (fields, ""),
            UsageBlock::Section {
                marker,
                required,
                fields,
            } => {
                out.push_str(&format_row(
                    &section_columns(marker, *required, has_values),
                    &widths,
                ));
                let _ = writeln!(out);
                (fields, INDENT)
            }
        };
        for field in fields {
            out.push_str(&format_wrapped_row(
                &field_columns(field, has_values, indent),
                &widths,
            ));
        }
    }
}

/// Every table row with the indent its block applies to the variable name.
fn table_rows(blocks: &[UsageBlock]) -> Vec<(&UsageField, &'static str)> {
    let mut rows = Vec::new();
    for block in blocks {
        match block {
            UsageBlock::Fields(fields) => rows.extend(fields.iter().map(|field| (field, ""))),
            UsageBlock::Section { fields, .. } => {
                rows.extend(fields.iter().map(|field| (field, INDENT)))
            }
        }
    }
    rows
}

/// A group marker occupies the VARIABLE column; REQUIRED then applies to the group itself.
fn section_columns(marker: &str, required: bool, has_values: bool) -> Vec<String> {
    let mut cols = vec![
        marker.to_string(),
        String::new(),
        if required {
            "yes".to_string()
        } else {
            "no".to_string()
        },
        String::new(),
    ];
    if has_values {
        cols.push(String::new());
    }
    cols
}

fn header_cols(has_values: bool) -> Vec<String> {
    let mut cols = vec![
        "VARIABLE".to_string(),
        "TYPE".to_string(),
        "REQUIRED".to_string(),
        "DEFAULT".to_string(),
    ];
    if has_values {
        cols.push("VALUES".to_string());
    }
    cols
}

fn field_columns(field: &UsageField, has_values: bool, indent: &str) -> Vec<String> {
    let mut cols = vec![
        format!("{indent}{}", field.name),
        field.typ.display(),
        if field.required {
            "yes".to_string()
        } else {
            "no".to_string()
        },
        display_default(&field.default),
    ];
    if has_values {
        cols.push(display_values(field));
    }
    cols
}

fn column_widths(rows: &[(&UsageField, &str)], markers: &[&str], has_values: bool) -> Vec<usize> {
    let headers = header_cols(has_values);
    let mut widths: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for marker in markers {
        widths[0] = widths[0].max(marker.chars().count());
    }
    for (field, indent) in rows {
        for (i, col) in field_columns(field, has_values, indent).iter().enumerate() {
            if i == 0 {
                widths[i] = widths[i].max(col.chars().count());
            } else {
                let first = wrap_text(col, wrap_width_for(i)).into_iter().next();
                let first_len = first.map(|s| s.chars().count()).unwrap_or(0);
                widths[i] = widths[i].max(first_len);
            }
        }
    }
    widths
}

fn wrap_width_for(col: usize) -> usize {
    match col {
        3 | 4 => WRAP_WIDTH,
        _ => usize::MAX,
    }
}

fn format_row(cols: &[String], widths: &[usize]) -> String {
    let mut line = String::new();
    for (i, col) in cols.iter().enumerate() {
        if i > 0 {
            line.push_str("  ");
        }
        let pad = widths[i].saturating_sub(col.chars().count());
        line.push_str(col);
        for _ in 0..pad {
            line.push(' ');
        }
    }
    line.trim_end().to_string()
}

fn format_wrapped_row(cols: &[String], widths: &[usize]) -> String {
    let wrapped: Vec<Vec<String>> = cols
        .iter()
        .enumerate()
        .map(|(i, col)| wrap_text(col, wrap_width_for(i)))
        .collect();
    let lines = wrapped.iter().map(Vec::len).max().unwrap_or(1);
    let mut out = String::new();
    for line_idx in 0..lines {
        let mut parts = Vec::with_capacity(cols.len());
        for cell_lines in &wrapped {
            parts.push(cell_lines.get(line_idx).cloned().unwrap_or_default());
        }
        out.push_str(&format_row(&parts, widths));
        let _ = writeln!(out);
    }
    out
}

fn wrap_text(s: &str, width: usize) -> Vec<String> {
    if width == usize::MAX || s.chars().count() <= width {
        return vec![s.to_string()];
    }
    let mut lines = Vec::new();
    let mut rest = s;
    while !rest.is_empty() {
        if rest.chars().count() <= width {
            lines.push(rest.to_string());
            break;
        }
        let split_at = find_split(rest, width);
        let (head, tail) = split_at_char(rest, split_at);
        lines.push(head.trim_end().to_string());
        rest = tail.trim_start();
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

fn find_split(s: &str, width: usize) -> usize {
    let mut last_break = None;
    for (count, (idx, ch)) in s.char_indices().enumerate() {
        if count >= width {
            return last_break.unwrap_or(idx);
        }
        if matches!(ch, ' ' | ',' | ';' | '|') {
            last_break = Some(idx + ch.len_utf8());
        }
    }
    s.len()
}

fn split_at_char(s: &str, idx: usize) -> (&str, &str) {
    if idx >= s.len() {
        (s, "")
    } else {
        s.split_at(idx)
    }
}

fn display_default(default: &Option<String>) -> String {
    match default {
        None => NO_DEFAULT.to_string(),
        Some(value) if value.is_empty() => r#""""#.to_string(),
        Some(value) => value.clone(),
    }
}

fn display_values(field: &UsageField) -> String {
    if let Some(values) = &field.values {
        return values.join(" | ");
    }
    match field.typ.int_limit() {
        Some(limit) => limit.display(),
        None => NO_DEFAULT.to_string(),
    }
}

fn syntax_notes(tree: &UsageTree) -> Vec<String> {
    let mut list = false;
    let mut map = false;
    let mut set = false;
    let mut duration = false;
    let mut bytesize = false;
    let mut int_range = false;
    let mut non_zero = false;
    walk_types(tree, &mut |typ| {
        if let Some(limit) = typ.int_limit() {
            int_range |= limit.is_range();
            non_zero |= limit.excludes_zero();
        }
        list |= typ.uses_list();
        map |= typ.uses_map();
        set |= typ.uses_set_other();
        duration |= typ.uses_duration();
        bytesize |= typ.uses_bytesize();
    });
    let mut notes = Vec::new();
    if int_range {
        notes.push(
            "Integer ranges are inclusive bounds; a value outside them fails to parse.".to_string(),
        );
    }
    if non_zero {
        notes.push("\"not 0\" means the parser rejects zero.".to_string());
    }
    if list {
        notes.push("Lists are comma-separated values, for example a,b,c.".to_string());
    }
    if map {
        notes
            .push("Maps are semicolon-separated key=value pairs, for example a=b;c=d.".to_string());
    }
    if set {
        notes.push("Sets are semicolon-separated values, for example a;b;c.".to_string());
    }
    if duration {
        notes.push("Durations accept values such as 15s, 10m, and 24h.".to_string());
    }
    if bytesize {
        notes.push("Byte sizes accept values such as 4MB and 10MiB.".to_string());
    }
    notes
}

fn walk_types(tree: &UsageTree, visit: &mut impl FnMut(&UsageType)) {
    walk_item_types(&tree.items, visit);
}

fn walk_item_types(items: &[UsageItem], visit: &mut impl FnMut(&UsageType)) {
    for item in items {
        match item {
            UsageItem::Field(field) => visit(&field.typ),
            UsageItem::Group(group) => walk_item_types(&group.items, visit),
        }
    }
}

fn normalize_output(out: &str) -> String {
    let mut lines: Vec<String> = out
        .lines()
        .map(|line| line.trim_end().to_string())
        .collect();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    lines.push(String::new());
    lines.join("\n")
}

/// Strips the namespace from a type name, leaving only the base type.
pub fn strip_namespace(name: &str) -> String {
    static SPLITTERS: &[char] = &['(', ')', '[', ']', '<', '>', '{', '}', ' ', ',', '='];
    name.split_inclusive(SPLITTERS)
        .flat_map(|component| component.rsplit("::").next())
        .collect()
}

#[test]
fn test_fallback_title() {
    assert_eq!(fallback_title("maintenance"), "Maintenance");
    assert_eq!(fallback_title("client_registry"), "Client Registry");
    assert_eq!(fallback_title("google_code_auth"), "Google Code Auth");
    assert_eq!(fallback_title("gcs"), "Gcs");
}

#[test]
fn test_strip_namespace() {
    let types = vec![
        ("String", "String"),
        ("i32", "i32"),
        ("alloc::string::String", "String"),
        ("primitive_types::Point", "Point"),
        ("alloc::vec::Vec<alloc::string::String>", "Vec<String>"),
        ("alloc::vec::Vec<i32>", "Vec<i32>"),
        (
            "std::collections::hash::map::HashMap<alloc::string::String, alloc::string::String>",
            "HashMap<String, String>",
        ),
        (
            "alloc::vec::Vec<std::collections::hash::map::HashMap<alloc::string::String, alloc::vec::Vec<i32>>>",
            "Vec<HashMap<String, Vec<i32>>>",
        ),
        ("alloc::string::String", "String"),
        ("std::path::PathBuf", "PathBuf"),
    ];

    for (typ, expected) in types {
        assert_eq!(strip_namespace(typ), expected);
    }
}
