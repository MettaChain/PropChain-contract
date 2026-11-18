//! Reentrancy vulnerability scanner for ink! contracts (#1466).
//!
//! Parses Rust sources with `syn` (no compilation of the target contracts) and,
//! for every function, looks at the *order* of external calls and state writes:
//!
//! * **Critical** - state is written after an external call and the function
//!   has no reentrancy guard (the classic check-effects-interactions violation).
//! * **Warning** - an external call without a guard, but no later state write
//!   was found.
//! * **Info** - an external call in a function that uses a guard.
//!
//! External calls recognised: `build_call(..)` / `ink::env::call::*`,
//! `.invoke()`, `.try_invoke()`, `.call_v1()`, `.delegate_call(..)`, native
//! `self.env().transfer(..)`, and any name listed as `external = NAME` in the
//! config. Calls through generated contract refs (`self.token.transfer(..)`)
//! have no syntactic marker, so add them with `external = transfer` if needed.
//!
//! Guards recognised: the `non_reentrant!` macro, `ReentrancyGuard::enter` /
//! `.enter()` / `.exit()` on a receiver whose name mentions `guard` or `lock`,
//! `ensure_not_reentrant`, and names listed as `guard = NAME`.
//!
//! Statements inside macro invocations are opaque to `syn`; a `non_reentrant!`
//! block is therefore treated as guarded, and its tokens are searched textually
//! for external-call names.
//!
//! ```text
//! reentrancy-scanner [--config FILE] [--format json|text] PATH...
//! ```
//! Exit code 1 if any Critical finding exists, 2 on usage errors.

use quote::ToTokens;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprAssign, ExprBinary, ExprCall, ExprMacro, ExprMethodCall, ImplItemFn, ItemFn};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Severity {
    Info,
    Warning,
    Critical,
}

impl Severity {
    fn as_str(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Critical => "critical",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    pub file: String,
    pub function: String,
    pub line: usize,
    pub severity: Severity,
    pub description: String,
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub external: HashSet<String>,
    pub guards: HashSet<String>,
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
                let value = value.trim().to_string();
                match key.trim() {
                    "external" => {
                        cfg.external.insert(value);
                    }
                    "guard" => {
                        cfg.guards.insert(value);
                    }
                    _ => {}
                }
            }
        }
        cfg
    }
}

const BUILTIN_EXTERNAL: &[&str] = &[
    "build_call",
    "invoke",
    "try_invoke",
    "call_v1",
    "delegate_call",
];
const MUTATING_METHODS: &[&str] = &[
    "insert", "remove", "take", "push", "push_back", "pop", "clear", "set",
];

fn is_self_rooted(expr: &Expr) -> bool {
    match expr {
        Expr::Path(p) => p.path.is_ident("self"),
        Expr::Field(f) => is_self_rooted(&f.base),
        Expr::MethodCall(m) => is_self_rooted(&m.receiver),
        Expr::Index(i) => is_self_rooted(&i.expr),
        Expr::Paren(p) => is_self_rooted(&p.expr),
        Expr::Reference(r) => is_self_rooted(&r.expr),
        Expr::Unary(u) => is_self_rooted(&u.expr),
        _ => false,
    }
}

fn mentions_env(expr: &Expr) -> bool {
    expr.to_token_stream().to_string().replace(' ', "").contains("env()")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Kind {
    Call,
    Write,
}

#[derive(Clone, Copy, Debug)]
struct Event {
    line: usize,
    kind: Kind,
}

struct FnScan<'a> {
    cfg: &'a Config,
    events: Vec<Event>,
    guarded: bool,
}

impl<'a> FnScan<'a> {
    fn is_external(&self, name: &str) -> bool {
        BUILTIN_EXTERNAL.contains(&name) || self.cfg.external.contains(name)
    }

    fn call(&mut self, line: usize) {
        self.events.push(Event { line, kind: Kind::Call });
    }

    fn write(&mut self, line: usize) {
        self.events.push(Event { line, kind: Kind::Write });
    }
}

impl<'ast, 'a> Visit<'ast> for FnScan<'a> {
    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(p) = &*node.func {
            let path = p.path.to_token_stream().to_string().replace(' ', "");
            let last = p
                .path
                .segments
                .last()
                .map(|s| s.ident.to_string())
                .unwrap_or_default();
            if self.is_external(&last) || path.contains("env::call") {
                self.call(node.span().start().line);
            }
            if last == "ensure_not_reentrant" || self.cfg.guards.contains(&last) {
                self.guarded = true;
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let name = node.method.to_string();
        // Arguments and receiver are evaluated before the call itself.
        syn::visit::visit_expr_method_call(self, node);
        let line = node.span().start().line;
        if self.is_external(&name) || (name == "transfer" && mentions_env(&node.receiver)) {
            self.call(line);
        }
        if MUTATING_METHODS.contains(&name.as_str()) && is_self_rooted(&node.receiver) {
            self.write(line);
        }
        let recv = node.receiver.to_token_stream().to_string().to_lowercase();
        if (name == "enter" || name == "exit") && (recv.contains("guard") || recv.contains("lock")) {
            self.guarded = true;
        }
        if self.cfg.guards.contains(&name) {
            self.guarded = true;
        }
    }

    fn visit_expr_assign(&mut self, node: &'ast ExprAssign) {
        // The right-hand side is evaluated first.
        self.visit_expr(&node.right);
        if is_self_rooted(&node.left) {
            self.write(node.span().start().line);
        }
        self.visit_expr(&node.left);
    }

    fn visit_expr_binary(&mut self, node: &'ast ExprBinary) {
        use syn::BinOp::*;
        let compound = matches!(
            node.op,
            AddAssign(_) | SubAssign(_) | MulAssign(_) | DivAssign(_) | RemAssign(_)
        );
        if compound {
            self.visit_expr(&node.right);
            if is_self_rooted(&node.left) {
                self.write(node.span().start().line);
            }
            self.visit_expr(&node.left);
        } else {
            syn::visit::visit_expr_binary(self, node);
        }
    }

    fn visit_expr_macro(&mut self, node: &'ast ExprMacro) {
        let name = node
            .mac
            .path
            .segments
            .last()
            .map(|s| s.ident.to_string())
            .unwrap_or_default();
        if name == "non_reentrant" {
            self.guarded = true;
            let tokens = node.mac.tokens.to_string().replace(' ', "");
            let has_call = BUILTIN_EXTERNAL
                .iter()
                .any(|c| tokens.contains(&format!("{c}(")))
                || tokens.contains("env::call")
                || self.cfg.external.iter().any(|c| tokens.contains(&format!("{c}(")));
            if has_call {
                self.call(node.span().start().line);
            }
        }
        syn::visit::visit_expr_macro(self, node);
    }
}

fn analyze_fn(file: &str, name: &str, block: &syn::Block, cfg: &Config, out: &mut Vec<Finding>) {
    let mut scan = FnScan {
        cfg,
        events: Vec::new(),
        guarded: false,
    };
    scan.visit_block(block);
    let Some(first_call) = scan.events.iter().position(|e| e.kind == Kind::Call) else {
        return;
    };
    let call_line = scan.events[first_call].line;
    let push = |out: &mut Vec<Finding>, line: usize, severity: Severity, description: String| {
        out.push(Finding {
            file: file.to_string(),
            function: name.to_string(),
            line,
            severity,
            description,
        });
    };
    if scan.guarded {
        push(
            out,
            call_line,
            Severity::Info,
            "external call in a function protected by a reentrancy guard".to_string(),
        );
        return;
    }
    let write_after = scan.events[first_call + 1..]
        .iter()
        .find(|e| e.kind == Kind::Write);
    match write_after {
        Some(w) => push(
            out,
            w.line,
            Severity::Critical,
            format!(
                "state modified after the external call at line {call_line} with no reentrancy guard"
            ),
        ),
        None => push(
            out,
            call_line,
            Severity::Warning,
            "external call without a reentrancy guard".to_string(),
        ),
    }
}

struct FileVisitor<'a> {
    file: &'a str,
    cfg: &'a Config,
    out: Vec<Finding>,
}

fn is_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("test")
            || (a.path().is_ident("cfg") && a.meta.to_token_stream().to_string().contains("test"))
            || (a.path().is_ident("ink") && a.meta.to_token_stream().to_string().contains("test"))
    })
}

impl<'ast, 'a> Visit<'ast> for FileVisitor<'a> {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if is_test_attr(&node.attrs) {
            return;
        }
        syn::visit::visit_item_mod(self, node);
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if !is_test_attr(&node.attrs) {
            analyze_fn(self.file, &node.sig.ident.to_string(), &node.block, self.cfg, &mut self.out);
        }
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        if !is_test_attr(&node.attrs) {
            analyze_fn(self.file, &node.sig.ident.to_string(), &node.block, self.cfg, &mut self.out);
        }
        syn::visit::visit_impl_item_fn(self, node);
    }
}

/// Scan one file's text.
pub fn scan_source(file: &str, src: &str, cfg: &Config) -> Result<Vec<Finding>, String> {
    let ast = syn::parse_file(src).map_err(|e| format!("{file}: {e}"))?;
    let mut v = FileVisitor {
        file,
        cfg,
        out: Vec::new(),
    };
    v.visit_file(&ast);
    Ok(v.out)
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

pub fn to_json(findings: &[Finding]) -> String {
    let count = |s: Severity| findings.iter().filter(|f| f.severity == s).count();
    let items: Vec<String> = findings
        .iter()
        .map(|f| {
            format!(
                "{{\"file\":\"{}\",\"function\":\"{}\",\"line\":{},\"severity\":\"{}\",\"description\":\"{}\"}}",
                json_escape(&f.file),
                json_escape(&f.function),
                f.line,
                f.severity.as_str(),
                json_escape(&f.description)
            )
        })
        .collect();
    format!(
        "{{\"summary\":{{\"total\":{},\"critical\":{},\"warning\":{},\"info\":{}}},\"findings\":[{}]}}",
        findings.len(),
        count(Severity::Critical),
        count(Severity::Warning),
        count(Severity::Info),
        items.join(",")
    )
}

pub fn to_text(findings: &[Finding]) -> String {
    let mut out = String::new();
    for f in findings {
        out.push_str(&format!(
            "{}:{}: [{}] fn {} - {}\n",
            f.file,
            f.line,
            f.severity.as_str(),
            f.function,
            f.description
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
    let mut format = String::from("text");
    let mut config_path = None;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--format" => format = args.next().unwrap_or_default(),
            "--config" => config_path = args.next(),
            _ => paths.push(PathBuf::from(a)),
        }
    }
    if paths.is_empty() {
        eprintln!("usage: reentrancy-scanner [--config FILE] [--format json|text] PATH...");
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
            Ok(src) => match scan_source(&name, &src, &cfg) {
                Ok(mut r) => findings.append(&mut r),
                Err(e) => eprintln!("warning: skipping {e}"),
            },
            Err(e) => eprintln!("warning: cannot read {name}: {e}"),
        }
    }
    if format == "json" {
        println!("{}", to_json(&findings));
    } else {
        print!("{}", to_text(&findings));
    }
    if findings.iter().any(|f| f.severity == Severity::Critical) {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(src: &str) -> Vec<Finding> {
        scan_source("t.rs", src, &Config::default()).unwrap()
    }

    #[test]
    fn state_write_after_an_unguarded_call_is_critical() {
        let f = scan(
            "impl C { fn withdraw(&mut self) {
                let r = build_call::<E>().invoke();
                self.balances.insert(1, &0);
            } }",
        );
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Critical);
        assert_eq!(f[0].function, "withdraw");
        assert!(f[0].description.contains("line 2"));
        assert_eq!(f[0].line, 3);
    }

    #[test]
    fn checks_effects_interactions_order_is_not_critical() {
        let f = scan(
            "impl C { fn withdraw(&mut self) {
                self.balances.insert(1, &0);
                build_call::<E>().invoke();
            } }",
        );
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Warning);
    }

    #[test]
    fn a_guard_downgrades_to_info() {
        let f = scan(
            "impl C { fn withdraw(&mut self) {
                self.guard.enter().unwrap();
                build_call::<E>().invoke();
                self.total = 0;
                self.guard.exit();
            } }",
        );
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Info);
    }

    #[test]
    fn the_non_reentrant_macro_counts_as_a_guard_and_its_calls_are_seen() {
        let f = scan(
            "impl C { fn go(&mut self) {
                non_reentrant!(self.guard, { build_call::<E>().invoke(); self.x = 1; });
            } }",
        );
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Info);
    }

    #[test]
    fn functions_without_external_calls_are_ignored() {
        assert!(scan("impl C { fn set(&mut self) { self.x = 1; self.map.insert(1, &2); } }").is_empty());
    }

    #[test]
    fn native_transfer_through_env_is_an_external_call() {
        let f = scan(
            "impl C { fn pay(&mut self, to: A) {
                self.env().transfer(to, 1).unwrap();
                self.paid += 1;
            } }",
        );
        assert_eq!(f[0].severity, Severity::Critical);
    }

    #[test]
    fn a_transfer_on_something_else_is_not_external() {
        assert!(scan("impl C { fn f(&mut self) { self.ledger.transfer(1); self.x = 1; } }").is_empty());
    }

    #[test]
    fn right_hand_side_calls_happen_before_the_assignment() {
        let f = scan("impl C { fn f(&mut self) { self.x = build_call::<E>().invoke(); } }");
        assert_eq!(f[0].severity, Severity::Critical);
    }

    #[test]
    fn compound_assignment_after_a_call_is_critical() {
        let f = scan(
            "impl C { fn f(&mut self) { build_call::<E>().invoke(); self.total += 1; } }",
        );
        assert_eq!(f[0].severity, Severity::Critical);
    }

    #[test]
    fn config_adds_external_names_and_guards() {
        let cfg = Config::parse("external = notify\nguard = lock_state\n");
        let src = "impl C { fn f(&mut self) { self.notify(); self.x = 1; } }";
        let f = scan_source("t.rs", src, &cfg).unwrap();
        assert_eq!(f[0].severity, Severity::Critical);
        let guarded = "impl C { fn f(&mut self) { self.lock_state(); self.notify(); self.x = 1; } }";
        assert_eq!(scan_source("t.rs", guarded, &cfg).unwrap()[0].severity, Severity::Info);
    }

    #[test]
    fn test_modules_are_skipped() {
        let f = scan(
            "#[cfg(test)] mod tests { fn t(c: &mut C) { build_call::<E>().invoke(); c.x = 1; } }",
        );
        assert!(f.is_empty());
    }

    #[test]
    fn free_functions_are_scanned_too() {
        let f = scan("fn go() { build_call::<E>().invoke(); }");
        assert_eq!(f[0].severity, Severity::Warning);
        assert_eq!(f[0].function, "go");
    }

    #[test]
    fn json_and_text_reports() {
        let f = scan("impl C { fn f(&mut self) { build_call::<E>().invoke(); self.x = 1; } }");
        let json = to_json(&f);
        assert!(json.contains("\"critical\":1"));
        assert!(json.contains("\"function\":\"f\""));
        assert!(to_text(&f).contains("[critical] fn f"));
    }

    #[test]
    fn unparsable_source_is_an_error() {
        assert!(scan_source("bad.rs", "fn (", &Config::default()).is_err());
    }
}
