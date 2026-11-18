//! Overflow detector for ink! contract sources (#1467).
//!
//! Parses Rust files with `syn` and reports arithmetic that is not wrapped in a
//! checked/saturating form, plus `as` casts that can truncate. It works on the
//! syntax tree only (no type information), so it reports *candidates*: a
//! `a + b` on two `u8` values is flagged exactly like one on `u128`.
//!
//! ```text
//! overflow-detector [--config FILE] [--json] [--fail-on high|medium|low] PATH...
//! ```
//!
//! Safe forms are method calls (`checked_add`, `saturating_mul`, `wrapping_*`,
//! `overflowing_*`, ...), which are not binary expressions and so are never
//! flagged. Anything under `#[cfg(test)]` or `#[test]` is skipped. A config
//! file (one entry per line, `#` comments) can add `safe_call = NAME` entries:
//! arithmetic inside the arguments of a call to `NAME` is treated as safe.

use proc_macro2::LineColumn;
use quote::ToTokens;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{BinOp, Expr, ExprBinary, ExprCall, ExprCast, ExprMethodCall, File, ItemFn, ItemMod};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Risk {
    Low,
    Medium,
    High,
}

impl Risk {
    fn as_str(self) -> &'static str {
        match self {
            Risk::Low => "low",
            Risk::Medium => "medium",
            Risk::High => "high",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    pub file: String,
    pub line: usize,
    pub column: usize,
    /// `unchecked_add`, `unchecked_sub`, `unchecked_mul`, `unchecked_div`, or `truncating_cast`.
    pub kind: String,
    pub operation: String,
    pub risk: Risk,
    pub message: String,
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    /// Function / method names whose arguments are considered overflow-safe.
    pub safe_calls: HashSet<String>,
}

impl Config {
    pub fn parse(text: &str) -> Config {
        let mut cfg = Config::default();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                if key.trim() == "safe_call" {
                    cfg.safe_calls.insert(value.trim().to_string());
                }
            }
        }
        cfg
    }
}

fn is_literal(expr: &Expr) -> bool {
    match expr {
        Expr::Lit(_) => true,
        Expr::Paren(p) => is_literal(&p.expr),
        Expr::Unary(u) => is_literal(&u.expr),
        Expr::Binary(b) => is_literal(&b.left) && is_literal(&b.right),
        _ => false,
    }
}

fn is_zero_literal(expr: &Expr) -> bool {
    match expr {
        Expr::Lit(l) => match &l.lit {
            syn::Lit::Int(i) => i.base10_parse::<u128>().map(|v| v == 0).unwrap_or(false),
            _ => false,
        },
        Expr::Paren(p) => is_zero_literal(&p.expr),
        _ => false,
    }
}

fn has_attr(attrs: &[syn::Attribute], name: &str) -> bool {
    attrs.iter().any(|a| a.path().is_ident(name))
}

fn is_cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg") && {
            let s = a.meta.to_token_stream().to_string();
            s.contains("test") && !s.contains("not (test)") && !s.contains("not(test)")
        }
    })
}

/// Narrow integer targets that can lose bits when cast from a wider type.
fn truncating_target(ty: &str) -> Option<Risk> {
    match ty {
        "u8" | "u16" | "u32" | "i8" | "i16" | "i32" => Some(Risk::High),
        "u64" | "i64" | "usize" | "isize" => Some(Risk::Medium),
        _ => None,
    }
}

struct Analyzer<'a> {
    file: &'a str,
    cfg: &'a Config,
    findings: Vec<Finding>,
}

impl<'a> Analyzer<'a> {
    fn push(&mut self, at: LineColumn, kind: &str, operation: String, risk: Risk, message: &str) {
        self.findings.push(Finding {
            file: self.file.to_string(),
            line: at.line,
            column: at.column + 1,
            kind: kind.to_string(),
            operation,
            risk,
            message: message.to_string(),
        });
    }
}

impl<'ast, 'a> Visit<'ast> for Analyzer<'a> {
    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        if is_cfg_test(&node.attrs) {
            return;
        }
        syn::visit::visit_item_mod(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if has_attr(&node.attrs, "test") || is_cfg_test(&node.attrs) {
            return;
        }
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        if has_attr(&node.attrs, "test") || is_cfg_test(&node.attrs) {
            return;
        }
        syn::visit::visit_impl_item_fn(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(p) = &*node.func {
            if let Some(seg) = p.path.segments.last() {
                if self.cfg.safe_calls.contains(&seg.ident.to_string()) {
                    return;
                }
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        if self.cfg.safe_calls.contains(&node.method.to_string()) {
            return;
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast ExprBinary) {
        let (kind, sym) = match node.op {
            BinOp::Add(_) | BinOp::AddAssign(_) => ("unchecked_add", "+"),
            BinOp::Sub(_) | BinOp::SubAssign(_) => ("unchecked_sub", "-"),
            BinOp::Mul(_) | BinOp::MulAssign(_) => ("unchecked_mul", "*"),
            BinOp::Div(_) | BinOp::DivAssign(_) => ("unchecked_div", "/"),
            _ => {
                syn::visit::visit_expr_binary(self, node);
                return;
            }
        };
        let both_literal = is_literal(&node.left) && is_literal(&node.right);
        let one_literal = is_literal(&node.left) || is_literal(&node.right);
        if !both_literal {
            let (risk, message) = match kind {
                "unchecked_mul" if !one_literal => (
                    Risk::High,
                    "multiplication of two non-constant values; use checked_mul",
                ),
                "unchecked_mul" => (Risk::Low, "multiplication by a constant; use checked_mul if the other operand is unbounded"),
                "unchecked_div" if is_zero_literal(&node.right) => {
                    (Risk::High, "division by a literal zero")
                }
                "unchecked_div" if is_literal(&node.right) => {
                    (Risk::Low, "division by a non-zero constant cannot fail")
                }
                "unchecked_div" => (Risk::Medium, "division by a non-constant value; check for zero"),
                _ if one_literal => (Risk::Low, "arithmetic with a constant operand; use a checked variant if the other side is unbounded"),
                _ => (Risk::Medium, "arithmetic on two non-constant values; use a checked or saturating variant"),
            };
            // A constant divisor other than zero is not a finding.
            if !(kind == "unchecked_div" && is_literal(&node.right) && !is_zero_literal(&node.right)) {
                self.push(
                    node.span().start(),
                    kind,
                    format!("{} {} {}", node.left.to_token_stream(), sym, node.right.to_token_stream()),
                    risk,
                    message,
                );
            }
        }
        syn::visit::visit_expr_binary(self, node);
    }

    fn visit_expr_cast(&mut self, node: &'ast ExprCast) {
        if !is_literal(&node.expr) {
            let target = node.ty.to_token_stream().to_string();
            if let Some(risk) = truncating_target(&target) {
                self.push(
                    node.span().start(),
                    "truncating_cast",
                    format!("{} as {}", node.expr.to_token_stream(), target),
                    risk,
                    "cast to a narrower integer type may truncate; use try_from",
                );
            }
        }
        syn::visit::visit_expr_cast(self, node);
    }
}

/// Analyse one source file's text.
pub fn analyze_source(file: &str, src: &str, cfg: &Config) -> Result<Vec<Finding>, String> {
    let ast: File = syn::parse_file(src).map_err(|e| format!("{file}: {e}"))?;
    let mut analyzer = Analyzer {
        file,
        cfg,
        findings: Vec::new(),
    };
    analyzer.visit_file(&ast);
    let mut findings = analyzer.findings;
    findings.sort_by(|a, b| (a.line, a.column).cmp(&(b.line, b.column)));
    Ok(findings)
}

fn json_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// CI-friendly JSON: `{"summary": {...}, "findings": [...]}`.
pub fn to_json(findings: &[Finding]) -> String {
    let count = |r: Risk| findings.iter().filter(|f| f.risk == r).count();
    let mut out = format!(
        "{{\"summary\":{{\"total\":{},\"high\":{},\"medium\":{},\"low\":{}}},\"findings\":[",
        findings.len(),
        count(Risk::High),
        count(Risk::Medium),
        count(Risk::Low)
    );
    for (i, f) in findings.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&format!(
            "{{\"file\":\"{}\",\"line\":{},\"column\":{},\"kind\":\"{}\",\"operation\":\"{}\",\"risk\":\"{}\",\"message\":\"{}\"}}",
            json_escape(&f.file),
            f.line,
            f.column,
            f.kind,
            json_escape(&f.operation),
            f.risk.as_str(),
            json_escape(&f.message)
        ));
    }
    out.push_str("]}");
    out
}

fn to_text(findings: &[Finding]) -> String {
    let mut out = String::new();
    for f in findings {
        out.push_str(&format!(
            "{}:{}:{}: [{}] {} `{}` - {}\n",
            f.file,
            f.line,
            f.column,
            f.risk.as_str(),
            f.kind,
            f.operation,
            f.message
        ));
    }
    out.push_str(&format!("{} finding(s)\n", findings.len()));
    out
}

fn collect_rs_files(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        if path.extension().map(|e| e == "rs").unwrap_or(false) {
            out.push(path.to_path_buf());
        }
        return;
    }
    if let Ok(entries) = fs::read_dir(path) {
        let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
        entries.sort();
        for p in entries {
            if p.file_name().map(|n| n == "target" || n == ".git").unwrap_or(false) {
                continue;
            }
            collect_rs_files(&p, out);
        }
    }
}

fn main() {
    let mut args = std::env::args().skip(1);
    let mut paths = Vec::new();
    let mut json = false;
    let mut config_path = None;
    let mut fail_on = Risk::High;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--json" => json = true,
            "--config" => config_path = args.next(),
            "--fail-on" => {
                fail_on = match args.next().as_deref() {
                    Some("low") => Risk::Low,
                    Some("medium") => Risk::Medium,
                    _ => Risk::High,
                }
            }
            _ => paths.push(PathBuf::from(a)),
        }
    }
    if paths.is_empty() {
        eprintln!("usage: overflow-detector [--config FILE] [--json] [--fail-on high|medium|low] PATH...");
        std::process::exit(2);
    }
    let cfg = match config_path {
        Some(p) => Config::parse(&fs::read_to_string(&p).unwrap_or_else(|e| {
            eprintln!("cannot read config {p}: {e}");
            std::process::exit(2)
        })),
        None => Config::default(),
    };
    let mut files = Vec::new();
    for p in &paths {
        collect_rs_files(p, &mut files);
    }
    let mut findings = Vec::new();
    for f in &files {
        let name = f.display().to_string();
        match fs::read_to_string(f) {
            Ok(src) => match analyze_source(&name, &src, &cfg) {
                Ok(mut found) => findings.append(&mut found),
                Err(e) => eprintln!("warning: skipping {e}"),
            },
            Err(e) => eprintln!("warning: cannot read {name}: {e}"),
        }
    }
    if json {
        println!("{}", to_json(&findings));
    } else {
        print!("{}", to_text(&findings));
    }
    if findings.iter().any(|f| f.risk >= fail_on) {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str) -> Vec<Finding> {
        analyze_source("t.rs", src, &Config::default()).unwrap()
    }

    #[test]
    fn flags_multiplication_of_two_variables_as_high() {
        let f = run("fn f(a: u128, b: u128) -> u128 { a * b }");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, "unchecked_mul");
        assert_eq!(f[0].risk, Risk::High);
    }

    #[test]
    fn checked_and_saturating_forms_are_safe() {
        let f = run(
            "fn f(a: u128, b: u128) -> Option<u128> {
                let x = a.checked_add(b)?;
                let y = x.saturating_mul(b);
                y.checked_sub(1)
            }",
        );
        assert!(f.is_empty(), "{f:?}");
    }

    #[test]
    fn flags_unchecked_add_and_sub_between_variables_as_medium() {
        let f = run("fn f(a: u64, b: u64) -> u64 { a + b - a }");
        assert_eq!(f.len(), 2);
        assert!(f.iter().all(|x| x.risk == Risk::Medium));
    }

    #[test]
    fn constant_only_arithmetic_is_not_flagged() {
        assert!(run("fn f() -> u32 { 2 * 3 + 4 }").is_empty());
    }

    #[test]
    fn arithmetic_with_a_constant_operand_is_low() {
        let f = run("fn f(a: u32) -> u32 { a + 1 }");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].risk, Risk::Low);
    }

    #[test]
    fn division_by_variable_is_medium_and_by_nonzero_constant_is_ignored() {
        let f = run("fn f(a: u32, b: u32) -> u32 { a / b }");
        assert_eq!(f[0].risk, Risk::Medium);
        assert!(run("fn f(a: u32) -> u32 { a / 10 }").is_empty());
    }

    #[test]
    fn division_by_literal_zero_is_high() {
        let f = run("fn f(a: u32) -> u32 { a / 0 }");
        assert_eq!(f[0].risk, Risk::High);
    }

    #[test]
    fn narrowing_casts_are_flagged() {
        let f = run("fn f(a: u128) -> u64 { a as u64 }");
        assert_eq!(f[0].kind, "truncating_cast");
        assert_eq!(f[0].risk, Risk::Medium);
        let g = run("fn f(a: u64) -> u32 { a as u32 }");
        assert_eq!(g[0].risk, Risk::High);
    }

    #[test]
    fn widening_and_literal_casts_are_not_flagged() {
        assert!(run("fn f(a: u32) -> u128 { a as u128 }").is_empty());
        assert!(run("fn f() -> u32 { 5u64 as u32 }").is_empty());
    }

    #[test]
    fn compound_assignment_is_checked_too() {
        let f = run("fn f(mut a: u64, b: u64) { a += b; }");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].kind, "unchecked_add");
    }

    #[test]
    fn test_code_is_skipped() {
        let f = run(
            "#[cfg(test)] mod tests { fn t(a: u64, b: u64) -> u64 { a * b } }
             #[test] fn single(a: u64, b: u64) -> u64 { a * b }",
        );
        assert!(f.is_empty());
    }

    #[test]
    fn configured_safe_calls_suppress_findings_inside_their_arguments() {
        let cfg = Config::parse("# comment\nsafe_call = safe_math\n");
        let src = "fn f(a: u64, b: u64) -> u64 { safe_math(a * b) }";
        assert!(analyze_source("t.rs", src, &cfg).unwrap().is_empty());
        assert_eq!(run(src).len(), 1);
    }

    #[test]
    fn reports_line_and_column() {
        let f = run("fn f(a: u64, b: u64) -> u64 {\n    a * b\n}");
        assert_eq!((f[0].line, f[0].column), (2, 5));
    }

    #[test]
    fn json_output_has_summary_and_findings() {
        let f = run("fn f(a: u64, b: u64) -> u64 { a * b }");
        let json = to_json(&f);
        assert!(json.starts_with("{\"summary\":{\"total\":1,\"high\":1"));
        assert!(json.contains("\"kind\":\"unchecked_mul\""));
    }

    #[test]
    fn unparsable_source_is_an_error_not_a_panic() {
        assert!(analyze_source("bad.rs", "fn (", &Config::default()).is_err());
    }
}
