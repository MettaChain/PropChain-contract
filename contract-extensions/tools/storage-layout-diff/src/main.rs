//! Storage layout diff checker for ink! proxy upgrades (#1471).
//!
//! Parses the `#[ink(storage)]` struct of two versions of a contract (no
//! compilation of either) and compares the fields by name, position and type.
//! A proxy upgrade keeps the old storage, so any change that moves or
//! re-interprets an existing field corrupts state.
//!
//! Classification:
//!
//! * **Breaking**: a field was removed, existing fields were reordered, a field
//!   was inserted anywhere but at the end (it shifts the fields after it), or a
//!   field's type changed in a way that is not a widening.
//! * **Warning**: a field's type was widened within the same integer family
//!   (`u8`..`u128`, `i8`..`i128`). The SCALE encoding still changes size, so
//!   review it, but the value is not reinterpreted as something unrelated.
//! * **Safe**: new fields appended after every existing field.
//!
//! ```text
//! storage-layout-diff --old OLD.rs --new NEW.rs [--format text|json]
//! ```
//! Exit code 1 if any Breaking change exists, 2 on usage or parse errors.

use quote::ToTokens;
use std::fs;
use std::process::ExitCode;
use syn::{Attribute, Fields, Item, ItemStruct};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Field {
    pub name: String,
    pub ty: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Severity {
    Safe,
    Warning,
    Breaking,
}

impl Severity {
    fn as_str(self) -> &'static str {
        match self {
            Severity::Safe => "safe",
            Severity::Warning => "warning",
            Severity::Breaking => "breaking",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChangeKind {
    Removed,
    Reordered,
    Inserted,
    Appended,
    TypeChanged,
    TypeWidened,
}

impl ChangeKind {
    fn as_str(&self) -> &'static str {
        match self {
            ChangeKind::Removed => "removed",
            ChangeKind::Reordered => "reordered",
            ChangeKind::Inserted => "inserted",
            ChangeKind::Appended => "appended",
            ChangeKind::TypeChanged => "type_changed",
            ChangeKind::TypeWidened => "type_widened",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Change {
    pub field: String,
    pub kind: ChangeKind,
    pub severity: Severity,
    pub detail: String,
}

fn is_ink_storage(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("ink")
            && a.meta
                .require_list()
                .map(|l| l.tokens.to_string().split(',').any(|t| t.trim() == "storage"))
                .unwrap_or(false)
    })
}

fn collect_storage_structs<'a>(items: &'a [Item], out: &mut Vec<&'a ItemStruct>) {
    for item in items {
        match item {
            Item::Struct(s) if is_ink_storage(&s.attrs) => out.push(s),
            Item::Mod(m) => {
                if let Some((_, inner)) = &m.content {
                    collect_storage_structs(inner, out);
                }
            }
            _ => {}
        }
    }
}

/// Type text with all whitespace removed, so formatting differences never count as a change.
fn type_text(ty: &syn::Type) -> String {
    ty.to_token_stream().to_string().chars().filter(|c| !c.is_whitespace()).collect()
}

/// The fields of the first `#[ink(storage)]` struct in `source`, in declaration order.
pub fn parse_storage(source: &str) -> Result<Vec<Field>, String> {
    let file = syn::parse_file(source).map_err(|e| format!("parse error: {e}"))?;
    let mut found = Vec::new();
    collect_storage_structs(&file.items, &mut found);
    let storage = found.first().ok_or_else(|| "no #[ink(storage)] struct found".to_string())?;
    match &storage.fields {
        Fields::Named(named) => Ok(named
            .named
            .iter()
            .map(|f| Field {
                name: f.ident.as_ref().map(|i| i.to_string()).unwrap_or_default(),
                ty: type_text(&f.ty),
            })
            .collect()),
        Fields::Unnamed(unnamed) => Ok(unnamed
            .unnamed
            .iter()
            .enumerate()
            .map(|(i, f)| Field { name: i.to_string(), ty: type_text(&f.ty) })
            .collect()),
        Fields::Unit => Ok(Vec::new()),
    }
}

fn int_rank(ty: &str) -> Option<(char, u32)> {
    let (family, bits) = ty.split_at(1);
    if family != "u" && family != "i" {
        return None;
    }
    let bits: u32 = bits.parse().ok()?;
    matches!(bits, 8 | 16 | 32 | 64 | 128).then(|| (family.chars().next().unwrap(), bits))
}

/// `true` when `old` and `new` are integers of the same signedness and `new` is wider.
fn is_widening(old: &str, new: &str) -> bool {
    match (int_rank(old), int_rank(new)) {
        (Some((fo, bo)), Some((fn_, bn))) => fo == fn_ && bn > bo,
        _ => false,
    }
}

/// Compares two layouts and lists every difference.
pub fn diff(old: &[Field], new: &[Field]) -> Vec<Change> {
    let mut changes = Vec::new();
    let find = |fields: &[Field], name: &str| fields.iter().position(|f| f.name == name);

    for f in old {
        if find(new, &f.name).is_none() {
            changes.push(Change {
                field: f.name.clone(),
                kind: ChangeKind::Removed,
                severity: Severity::Breaking,
                detail: format!("field `{}: {}` was removed; later fields shift and storage is corrupted", f.name, f.ty),
            });
        }
    }

    // Relative order of the fields both versions have.
    let old_common: Vec<&str> =
        old.iter().filter(|f| find(new, &f.name).is_some()).map(|f| f.name.as_str()).collect();
    let new_common: Vec<&str> =
        new.iter().filter(|f| find(old, &f.name).is_some()).map(|f| f.name.as_str()).collect();
    for (rank, name) in new_common.iter().enumerate() {
        if old_common[rank] != *name {
            changes.push(Change {
                field: (*name).to_string(),
                kind: ChangeKind::Reordered,
                severity: Severity::Breaking,
                detail: format!(
                    "field `{}` moved: it was number {} of the surviving fields and is now number {}",
                    name,
                    old_common.iter().position(|n| n == name).map(|p| p + 1).unwrap_or(0),
                    rank + 1
                ),
            });
        }
    }

    for f in old {
        if let Some(i) = find(new, &f.name) {
            let new_ty = &new[i].ty;
            if *new_ty != f.ty {
                let widening = is_widening(&f.ty, new_ty);
                changes.push(Change {
                    field: f.name.clone(),
                    kind: if widening { ChangeKind::TypeWidened } else { ChangeKind::TypeChanged },
                    severity: if widening { Severity::Warning } else { Severity::Breaking },
                    detail: format!("type changed from `{}` to `{}`", f.ty, new_ty),
                });
            }
        }
    }

    // New fields: appended only if no existing field follows them.
    let last_existing = new.iter().rposition(|f| find(old, &f.name).is_some());
    for (i, f) in new.iter().enumerate() {
        if find(old, &f.name).is_some() {
            continue;
        }
        let appended = last_existing.map(|last| i > last).unwrap_or(true);
        changes.push(Change {
            field: f.name.clone(),
            kind: if appended { ChangeKind::Appended } else { ChangeKind::Inserted },
            severity: if appended { Severity::Safe } else { Severity::Breaking },
            detail: if appended {
                format!("new field `{}: {}` appended after all existing fields", f.name, f.ty)
            } else {
                format!("new field `{}: {}` inserted before existing fields; they shift", f.name, f.ty)
            },
        });
    }
    changes
}

pub fn worst(changes: &[Change]) -> Severity {
    changes.iter().map(|c| c.severity).max().unwrap_or(Severity::Safe)
}

fn json_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

pub fn render_text(old: &[Field], new: &[Field], changes: &[Change]) -> String {
    let mut out = String::from("Storage layout comparison\n\n");
    out.push_str(&format!("{:<4} {:<24} {:<28} {:<28}\n", "#", "field", "old type", "new type"));
    let mut names: Vec<&str> = old.iter().map(|f| f.name.as_str()).collect();
    for f in new {
        if !names.contains(&f.name.as_str()) {
            names.push(&f.name);
        }
    }
    for (i, name) in names.iter().enumerate() {
        let o = old.iter().find(|f| f.name == *name).map(|f| f.ty.as_str()).unwrap_or("-");
        let n = new.iter().find(|f| f.name == *name).map(|f| f.ty.as_str()).unwrap_or("-");
        out.push_str(&format!("{:<4} {:<24} {:<28} {:<28}\n", i + 1, name, o, n));
    }
    out.push('\n');
    if changes.is_empty() {
        out.push_str("No storage changes.\n");
    }
    for c in changes {
        out.push_str(&format!("[{}] {} ({}): {}\n", c.severity.as_str().to_uppercase(), c.field, c.kind.as_str(), c.detail));
    }
    out.push_str(&format!("\nResult: {}\n", worst(changes).as_str().to_uppercase()));
    out
}

pub fn render_json(old: &[Field], new: &[Field], changes: &[Change]) -> String {
    let fields = |fs: &[Field]| {
        fs.iter()
            .map(|f| format!("{{\"name\":\"{}\",\"type\":\"{}\"}}", json_escape(&f.name), json_escape(&f.ty)))
            .collect::<Vec<_>>()
            .join(",")
    };
    let items = changes
        .iter()
        .map(|c| {
            format!(
                "{{\"field\":\"{}\",\"kind\":\"{}\",\"severity\":\"{}\",\"detail\":\"{}\"}}",
                json_escape(&c.field),
                c.kind.as_str(),
                c.severity.as_str(),
                json_escape(&c.detail)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"result\":\"{}\",\"old\":[{}],\"new\":[{}],\"changes\":[{}]}}\n",
        worst(changes).as_str(),
        fields(old),
        fields(new),
        items
    )
}

fn usage() -> ExitCode {
    eprintln!("usage: storage-layout-diff --old OLD.rs --new NEW.rs [--format text|json]");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut old_path, mut new_path, mut format) = (None, None, "text".to_string());
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--old" => old_path = args.get(i + 1).cloned(),
            "--new" => new_path = args.get(i + 1).cloned(),
            "--format" => format = args.get(i + 1).cloned().unwrap_or_default(),
            _ => return usage(),
        }
        i += 2;
    }
    let (Some(old_path), Some(new_path)) = (old_path, new_path) else { return usage() };
    if format != "text" && format != "json" {
        return usage();
    }

    let read = |p: &str| fs::read_to_string(p).map_err(|e| format!("{p}: {e}"));
    let layouts = read(&old_path)
        .and_then(|s| parse_storage(&s).map_err(|e| format!("{old_path}: {e}")))
        .and_then(|o| {
            read(&new_path)
                .and_then(|s| parse_storage(&s).map_err(|e| format!("{new_path}: {e}")))
                .map(|n| (o, n))
        });
    let (old, new) = match layouts {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let changes = diff(&old, &new);
    if format == "json" {
        print!("{}", render_json(&old, &new, &changes));
    } else {
        print!("{}", render_text(&old, &new, &changes));
    }
    if worst(&changes) == Severity::Breaking {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(fields: &str) -> String {
        format!(
            "#[ink::contract]\nmod c {{\n    #[ink(storage)]\n    pub struct C {{\n{fields}\n    }}\n}}\n"
        )
    }

    fn layout(fields: &str) -> Vec<Field> {
        parse_storage(&contract(fields)).unwrap()
    }

    fn kinds(changes: &[Change]) -> Vec<(String, ChangeKind, Severity)> {
        changes.iter().map(|c| (c.field.clone(), c.kind.clone(), c.severity)).collect()
    }

    #[test]
    fn parses_names_types_and_order() {
        let l = layout("owner: AccountId,\nbalances: Mapping<AccountId, u128>,\ntotal: u128,");
        assert_eq!(
            l,
            vec![
                Field { name: "owner".into(), ty: "AccountId".into() },
                Field { name: "balances".into(), ty: "Mapping<AccountId,u128>".into() },
                Field { name: "total".into(), ty: "u128".into() },
            ]
        );
    }

    #[test]
    fn ignores_structs_without_the_storage_attribute() {
        let src = "mod c { pub struct Other { a: u8 } #[ink(storage)] pub struct S { b: u16 } }";
        assert_eq!(parse_storage(src).unwrap(), vec![Field { name: "b".into(), ty: "u16".into() }]);
    }

    #[test]
    fn reports_a_missing_storage_struct() {
        assert!(parse_storage("mod c { pub struct S { a: u8 } }").is_err());
    }

    #[test]
    fn identical_layouts_have_no_changes() {
        let l = layout("a: u32,\nb: bool,");
        assert!(diff(&l, &l).is_empty());
        assert_eq!(worst(&diff(&l, &l)), Severity::Safe);
    }

    #[test]
    fn formatting_differences_are_not_changes() {
        let a = layout("m: Mapping<AccountId, u128>,");
        let b = layout("m:   Mapping< AccountId,u128 >,");
        assert!(diff(&a, &b).is_empty());
    }

    #[test]
    fn appending_a_field_is_safe() {
        let old = layout("a: u32,\nb: bool,");
        let new = layout("a: u32,\nb: bool,\nc: u128,");
        let changes = diff(&old, &new);
        assert_eq!(kinds(&changes), vec![("c".into(), ChangeKind::Appended, Severity::Safe)]);
        assert_eq!(worst(&changes), Severity::Safe);
    }

    #[test]
    fn removing_a_field_is_breaking() {
        let old = layout("a: u32,\nb: bool,\nc: u8,");
        let new = layout("a: u32,\nc: u8,");
        let changes = diff(&old, &new);
        assert_eq!(kinds(&changes), vec![("b".into(), ChangeKind::Removed, Severity::Breaking)]);
        assert_eq!(worst(&changes), Severity::Breaking);
    }

    #[test]
    fn reordering_fields_is_breaking() {
        let old = layout("a: u32,\nb: bool,");
        let new = layout("b: bool,\na: u32,");
        let changes = diff(&old, &new);
        assert_eq!(worst(&changes), Severity::Breaking);
        assert!(changes.iter().all(|c| c.kind == ChangeKind::Reordered));
        assert_eq!(changes.len(), 2);
    }

    #[test]
    fn inserting_a_field_in_the_middle_is_breaking() {
        let old = layout("a: u32,\nc: u8,");
        let new = layout("a: u32,\nb: bool,\nc: u8,");
        let changes = diff(&old, &new);
        assert_eq!(kinds(&changes), vec![("b".into(), ChangeKind::Inserted, Severity::Breaking)]);
    }

    #[test]
    fn inserting_a_field_at_the_front_is_breaking() {
        let old = layout("a: u32,");
        let new = layout("z: u8,\na: u32,");
        assert_eq!(kinds(&diff(&old, &new)), vec![("z".into(), ChangeKind::Inserted, Severity::Breaking)]);
    }

    #[test]
    fn an_unrelated_type_change_is_breaking() {
        let old = layout("a: u32,");
        let new = layout("a: AccountId,");
        let changes = diff(&old, &new);
        assert_eq!(kinds(&changes), vec![("a".into(), ChangeKind::TypeChanged, Severity::Breaking)]);
    }

    #[test]
    fn widening_an_integer_is_a_warning() {
        let old = layout("a: u32,\nb: i16,");
        let new = layout("a: u64,\nb: i64,");
        let changes = diff(&old, &new);
        assert_eq!(worst(&changes), Severity::Warning);
        assert!(changes.iter().all(|c| c.kind == ChangeKind::TypeWidened));
    }

    #[test]
    fn narrowing_or_changing_signedness_is_breaking() {
        assert_eq!(worst(&diff(&layout("a: u64,"), &layout("a: u32,"))), Severity::Breaking);
        assert_eq!(worst(&diff(&layout("a: u32,"), &layout("a: i64,"))), Severity::Breaking);
    }

    #[test]
    fn the_worst_change_decides_the_result() {
        let old = layout("a: u32,\nb: bool,");
        let new = layout("a: u64,\nc: u8,");
        let changes = diff(&old, &new);
        assert_eq!(worst(&changes), Severity::Breaking); // `b` removed
        assert!(changes.iter().any(|c| c.severity == Severity::Warning));
        assert!(changes.iter().any(|c| c.severity == Severity::Safe));
    }

    #[test]
    fn json_output_lists_the_result_and_changes() {
        let old = layout("a: u32,");
        let new = layout("a: u32,\nb: u8,");
        let json = render_json(&old, &new, &diff(&old, &new));
        assert!(json.starts_with("{\"result\":\"safe\""));
        assert!(json.contains("\"kind\":\"appended\""));
        assert!(json.contains("\"name\":\"b\""));
    }

    #[test]
    fn text_output_shows_a_field_by_field_table() {
        let old = layout("a: u32,");
        let new = layout("a: u64,");
        let text = render_text(&old, &new, &diff(&old, &new));
        assert!(text.contains("u32"));
        assert!(text.contains("u64"));
        assert!(text.contains("Result: WARNING"));
    }
}
