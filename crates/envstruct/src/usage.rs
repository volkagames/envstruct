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
}

/// Human-facing value format for usage output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UsageType {
    String,
    /// Rust integer type name (`u32`, `NonZeroUsize`) and optional parser bounds.
    Integer(String, Option<IntLimit>),
    /// Rust float type name (`f32`, `f64`).
    Float(String),
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
            Self::Integer(name, _) => name.clone(),
            Self::Float(name) => name.clone(),
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
            Self::Integer(_, limit) => limit.as_ref(),
            _ => None,
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
    /// Field doc-comment, preserved as Markdown for detailed output.
    pub description: Option<String>,
    pub typ: UsageType,
    pub required: bool,
    pub default: Option<String>,
    pub values: Option<Vec<String>>,
    /// Usage-only mark that this value is a secret (a k8s Secret, not a ConfigMap).
    pub secret: bool,
    /// Runtime-computed default shown in the DEFAULT column in parentheses.
    pub default_note: Option<String>,
}

/// A named group of fields and nested groups.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UsageGroup {
    pub title: String,
    /// Doc-comment on the field containing this group.
    pub description: Option<String>,
    /// Preserve an inlined field's description without adding a section to the table.
    pub inline: bool,
    pub optional: bool,
    pub used_if: Option<UsageUsedIf>,
    pub items: Vec<UsageItem>,
}

/// Condition under which a group applies, written as `env_name=value`.
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
                description: None,
                typ,
                required,
                default,
                values,
                secret: false,
                default_note: None,
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
    pub secret: bool,
    pub default_note: Option<String>,
    pub description: Option<String>,
}

/// Wraps a field's usage tree as parent-group items.
pub fn attach_field_usage(mut tree: UsageTree, meta: FieldUsageMeta) -> Vec<UsageItem> {
    apply_usage_flags(&mut tree.items, meta.secret, meta.default_note.as_deref());
    match tree.kind {
        UsageTreeKind::Leaf => {
            for item in &mut tree.items {
                if let UsageItem::Field(field) = item {
                    if meta.description.is_some() {
                        field.description = meta.description.clone();
                    }
                }
            }
            tree.items
        }
        UsageTreeKind::Struct | UsageTreeKind::OptionalStruct => {
            // An inlined group renders as the fields of its parent, and is kept so that a
            // collision can still name the struct a variable was declared in.
            let inline = should_inline(&tree, &meta);
            vec![UsageItem::Group(UsageGroup {
                title: choose_title(&tree, &meta),
                description: meta.description,
                inline,
                optional: tree.kind == UsageTreeKind::OptionalStruct,
                used_if: meta.used_if,
                items: tree.items,
            })]
        }
    }
}

fn apply_usage_flags(items: &mut [UsageItem], secret: bool, default_note: Option<&str>) {
    if !secret && default_note.is_none() {
        return;
    }
    for item in items {
        match item {
            UsageItem::Field(field) => {
                field.secret |= secret;
                if field.default_note.is_none() {
                    field.default_note = default_note.map(str::to_string);
                }
            }
            UsageItem::Group(group) => {
                apply_usage_flags(&mut group.items, secret, default_note);
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

const NO_DEFAULT: &str = "<required>";
const WRAP_WIDTH: usize = 40;
/// Rows of a conditional group are indented under its marker line.
const INDENT: &str = "  ";
const COL_SEP: &str = " | ";
const HEADER_RULE_SEP: &str = "-+-";

enum UsageBlock {
    Fields(Vec<UsageField>),
    Section {
        marker: String,
        fields: Vec<UsageField>,
    },
}

fn render_usage(tree: &UsageTree) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "Environment variables");

    let blocks = collect_blocks(tree);
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
            UsageItem::Group(group) if group.inline => {
                let (nested_fields, nested_groups) = split_items(&group.items);
                fields.extend(nested_fields);
                groups.extend(nested_groups);
            }
            UsageItem::Group(group) => groups.push(group.clone()),
        }
    }
    fields.sort_by(|a, b| a.name.cmp(&b.name));
    (fields, groups)
}

fn collect_blocks(tree: &UsageTree) -> Vec<UsageBlock> {
    let mut blocks = Vec::new();
    emit_items(&tree.items, "", &mut blocks);
    blocks
}

/// Mix sibling leaves and nested structs by env name so prefixes stay together.
/// A used_if group stays glued under the switch that selects it.
fn emit_items(items: &[UsageItem], parent_path: &str, blocks: &mut Vec<UsageBlock>) {
    let (fields, groups) = split_items(items);

    let mut used_if_by_switch: std::collections::HashMap<String, Vec<UsageGroup>> =
        std::collections::HashMap::new();
    let mut free_groups = Vec::new();
    for group in groups {
        match group
            .used_if
            .as_ref()
            .map(|used_if| used_if.env_name.clone())
        {
            Some(name) => used_if_by_switch.entry(name).or_default().push(group),
            None => free_groups.push(group),
        }
    }

    let mut plain = Vec::new();
    let mut switches = Vec::new();
    for field in fields {
        if used_if_by_switch.contains_key(&field.name) {
            switches.push(field);
        } else {
            plain.push(field);
        }
    }

    enum Piece {
        Field(UsageField),
        Group(UsageGroup),
    }
    let mut pieces: Vec<(String, Piece)> = Vec::new();
    for field in plain {
        pieces.push((field.name.clone(), Piece::Field(field)));
    }
    for group in free_groups {
        let key = min_field_name(&group).unwrap_or_else(|| group.title.clone());
        pieces.push((key, Piece::Group(group)));
    }
    pieces.sort_by(|a, b| a.0.cmp(&b.0));

    let mut pending = Vec::new();
    for (_, piece) in pieces {
        match piece {
            Piece::Field(field) => pending.push(field),
            Piece::Group(group) => {
                if !pending.is_empty() {
                    blocks.push(UsageBlock::Fields(std::mem::take(&mut pending)));
                }
                emit_group(group, parent_path, blocks);
            }
        }
    }
    if !pending.is_empty() {
        blocks.push(UsageBlock::Fields(pending));
    }

    for switch in switches {
        let groups = used_if_by_switch.remove(&switch.name).unwrap_or_default();
        blocks.push(UsageBlock::Fields(vec![switch]));
        for group in groups {
            emit_group(group, parent_path, blocks);
        }
    }
    for groups in used_if_by_switch.into_values() {
        for group in groups {
            emit_group(group, parent_path, blocks);
        }
    }
}

fn emit_group(group: UsageGroup, parent_path: &str, blocks: &mut Vec<UsageBlock>) {
    let path = group_path(parent_path, &group.title);
    if group.optional || group.used_if.is_some() {
        let (fields, children) = split_items(&group.items);
        if !fields.is_empty() {
            blocks.push(UsageBlock::Section {
                marker: section_marker(&group, &path),
                fields,
            });
        }
        for child in children {
            emit_group(child, &path, blocks);
        }
    } else {
        emit_items(&group.items, &path, blocks);
    }
}

fn min_field_name(group: &UsageGroup) -> Option<String> {
    fn walk(items: &[UsageItem], min: &mut Option<String>) {
        for item in items {
            match item {
                UsageItem::Field(field) => {
                    if min.as_ref().is_none_or(|current| field.name < *current) {
                        *min = Some(field.name.clone());
                    }
                }
                UsageItem::Group(child) => walk(&child.items, min),
            }
        }
    }
    let mut min = None;
    walk(&group.items, &mut min);
    min
}

fn group_path(parent_path: &str, title: &str) -> String {
    if parent_path.is_empty() {
        title.to_string()
    } else {
        format!("{parent_path} → {title}")
    }
}

/// Returns the bracketed line above a section.
fn section_marker(group: &UsageGroup, path: &str) -> String {
    let Some(used_if) = &group.used_if else {
        return format!("[{path}]");
    };
    let mut cond = format!("used when {}={}", used_if.env_name, used_if.value);
    if used_if.switch_default.as_deref() == Some(used_if.value.as_str()) {
        cond.push_str(" (default)");
    }
    format!("[{cond}]")
}

fn render_blocks(out: &mut String, blocks: &[UsageBlock]) {
    let rows = table_rows(blocks);
    let markers: Vec<&str> = blocks
        .iter()
        .filter_map(|block| match block {
            UsageBlock::Section { marker, .. } => Some(marker.as_str()),
            UsageBlock::Fields(_) => None,
        })
        .collect();
    let widths = column_widths(&rows, &markers);
    if !rows.is_empty() {
        out.push_str(&format_row(&header_cols(), &widths));
        let _ = writeln!(out);
        out.push_str(&format_header_rule(&widths));
        let _ = writeln!(out);
    }

    // The table is printed as one continuous block: markers and name prefixes carry
    // the grouping, blank lines are not used inside it.
    for block in blocks {
        let (fields, indent) = match block {
            UsageBlock::Fields(fields) => (fields, ""),
            UsageBlock::Section { marker, fields } => {
                let _ = writeln!(out, "{marker}");
                (fields, INDENT)
            }
        };
        for field in fields {
            out.push_str(&format_wrapped_row(&field_columns(field, indent), &widths));
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

fn header_cols() -> Vec<String> {
    vec![
        "VARIABLE".to_string(),
        "TYPE".to_string(),
        "DEFAULT".to_string(),
    ]
}

fn field_columns(field: &UsageField, indent: &str) -> Vec<String> {
    vec![
        field_name_cell(field, indent),
        type_cell(field),
        display_default(field),
    ]
}

fn field_name_cell(field: &UsageField, indent: &str) -> String {
    if field.secret {
        format!("{indent}{} (secret)", field.name)
    } else {
        format!("{indent}{}", field.name)
    }
}

fn type_cell(field: &UsageField) -> String {
    let base = field.typ.display();
    if let Some(values) = &field.values {
        let values = values.join(", ");
        return match &field.typ {
            UsageType::Map(_, _) => format!("{base} (keys: {values})"),
            UsageType::List(_) => format!("{base} (items: {values})"),
            _ => format!("{base}: {values}"),
        };
    }
    match field.typ.int_limit() {
        Some(IntLimit::Range { min, max, .. }) => format!("{base} ({min}..={max})"),
        Some(IntLimit::NonZero) | None => base,
    }
}

fn column_widths(rows: &[(&UsageField, &str)], markers: &[&str]) -> Vec<usize> {
    let headers = header_cols();
    let mut widths: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for marker in markers {
        widths[0] = widths[0].max(marker.chars().count());
    }
    for (field, indent) in rows {
        for (i, col) in field_columns(field, indent).iter().enumerate() {
            if i == 0 {
                widths[i] = widths[i].max(col.chars().count());
            } else {
                let max_len = wrap_text(col, wrap_width_for(i))
                    .iter()
                    .map(|fragment| fragment.chars().count())
                    .max()
                    .unwrap_or(0);
                widths[i] = widths[i].max(max_len);
            }
        }
    }
    widths
}

fn wrap_width_for(col: usize) -> usize {
    match col {
        1 | 2 => WRAP_WIDTH,
        _ => usize::MAX,
    }
}

fn format_row(cols: &[String], widths: &[usize]) -> String {
    format_row_n(cols, widths, true)
}

fn format_row_n(cols: &[String], widths: &[usize], trim_empty: bool) -> String {
    let last = if trim_empty {
        cols.iter().rposition(|col| !col.is_empty()).unwrap_or(0)
    } else {
        cols.len().saturating_sub(1)
    };
    let mut line = String::new();
    for (i, col) in cols.iter().take(last + 1).enumerate() {
        if i > 0 {
            line.push_str(COL_SEP);
        }
        let pad = widths[i].saturating_sub(col.chars().count());
        line.push_str(col);
        for _ in 0..pad {
            line.push(' ');
        }
    }
    line.trim_end().to_string()
}

fn format_header_rule(widths: &[usize]) -> String {
    widths
        .iter()
        .map(|width| "-".repeat(*width))
        .collect::<Vec<_>>()
        .join(HEADER_RULE_SEP)
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
        out.push_str(&format_row_n(&parts, widths, false));
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
        lines.push(trim_wrap_edge(head).to_string());
        rest = trim_wrap_edge(tail);
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
        if matches!(ch, ' ' | ',' | ';') {
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

fn trim_wrap_edge(s: &str) -> &str {
    s.trim_matches(|c: char| matches!(c, ' ' | ',' | ';'))
}

/// A table cell must stay on its own line, so control characters are rendered escaped instead of
/// being emitted raw and breaking the column layout.
fn escape_cell(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

fn quote_value(value: &str) -> String {
    format!("\"{}\"", escape_cell(value))
}

fn display_default(field: &UsageField) -> String {
    if let Some(value) = &field.default {
        quote_value(value)
    } else if let Some(note) = &field.default_note {
        if field.required {
            format!("{NO_DEFAULT} ({note})")
        } else {
            format!("({note})")
        }
    } else if field.required {
        NO_DEFAULT.to_string()
    } else {
        "none".to_string()
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
fn wrapping_preserves_literal_pipes_without_preferring_them_as_breaks() {
    assert_eq!(wrap_text("ab|cdefgh", 6), ["ab|cde", "fgh"]);
    assert_eq!(wrap_text("abcde|fgh", 6), ["abcde|", "fgh"]);
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
