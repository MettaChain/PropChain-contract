//! Access control audit for ink! contracts (#1468).
//!
//! Finds every `#[ink(message)]` and decides, from the source alone, whether it
//! has *visible* access control:
//!
//! * **Protected** - the body calls a recognised guard (`ensure_owner`,
//!   `only_admin`, `require_role`, `has_role`, ... or anything configured with
//!   `guard = NAME`), or reads `caller()` together with an owner/admin/role/
//!   operator style identifier.
//! * **IntentionallyPublic** - the message is annotated with a `// public`
//!   comment on the line above it (or after its signature).
//! * **Whitelisted** - listed in the config (`allow = message` or
//!   `allow = Contract::message`).
//! * **ReadOnly** - takes `&self`; excluded from coverage unless
//!   `--include-views` is given.
//! * **Unprotected** - none of the above.
//!
//! This is a syntactic heuristic. It cannot prove a guard is correct, only that
//! one is present.
//!
//! ```text
//! access-control-audit [--config FILE] [--json] [--include-views] [--fail-on-unprotected] PATH...
//! ```

use quote::ToTokens;
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprMethodCall, ImplItem, ItemImpl, ItemMod};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Protected,
    IntentionallyPublic,
    Whitelisted,
    ReadOnly,
    Unprotected,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Protected => "Protected",
            Status::IntentionallyPublic => "IntentionallyPublic",
            Status::Whitelisted => "Whitelisted",
            Status::ReadOnly => "ReadOnly",
            Status::Unprotected => "Unprotected",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MessageReport {
    pub file: String,
    pub module: String,
    pub contract: String,
    pub name: String,
    pub line: usize,
    pub status: Status,
    pub reason: String,
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub guards: HashSet<String>,
    pub allow: HashSet<String>,
    pub include_views: bool,
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
                    "guard" => {
                        cfg.guards.insert(value);
                    }
                    "allow" => {
                        cfg.allow.insert(value);
                    }
                    _ => {}
                }
            }
        }
        cfg
    }
}

const BUILTIN_GUARDS: &[&str] = &[
    "ensure_owner",
    "only_owner",
    "require_owner",
    "ensure_admin",
    "only_admin",
    "require_admin",
    "require_role",
    "ensure_role",
    "check_role",
    "only_role",
    "has_role",
    "is_admin",
    "is_owner",
    "is_authorized",
    "ensure_authorized",
    "require_authorized",
];

const ACCESS_WORDS: &[&str] = &[
    "owner",
    "admin",
    "role",
    "authorized",
    "authorised",
    "guardian",
    "operator",
    "verifier",
    "governance",
    "permission",
];

fn is_guard_name(name: &str, cfg: &Config) -> bool {
    if cfg.guards.contains(name) || BUILTIN_GUARDS.contains(&name) {
        return true;
    }
    let prefixed = ["ensure_", "require_", "only_", "check_", "assert_"]
        .iter()
        .any(|p| name.starts_with(p));
    prefixed && ACCESS_WORDS.iter().any(|w| name.contains(w))
}

fn is_ink_message(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("ink") && a.meta.to_token_stream().to_string().contains("message")
    })
}

#[derive(Default)]
struct BodyScan {
    guard_calls: Vec<String>,
    reads_caller: bool,
    access_words: bool,
}

struct BodyVisitor<'a> {
    cfg: &'a Config,
    scan: BodyScan,
}

impl<'ast, 'a> Visit<'ast> for BodyVisitor<'a> {
    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let name = node.method.to_string();
        if name == "caller" {
            self.scan.reads_caller = true;
        }
        if is_guard_name(&name, self.cfg) {
            self.scan.guard_calls.push(name);
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let Expr::Path(p) = &*node.func {
            if let Some(seg) = p.path.segments.last() {
                let name = seg.ident.to_string();
                if is_guard_name(&name, self.cfg) {
                    self.scan.guard_calls.push(name);
                }
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_ident(&mut self, ident: &'ast proc_macro2::Ident) {
        let lower = ident.to_string().to_lowercase();
        if ACCESS_WORDS.iter().any(|w| lower.contains(w)) {
            self.scan.access_words = true;
        }
    }
}

struct Collector<'a> {
    file: &'a str,
    lines: Vec<&'a str>,
    cfg: &'a Config,
    modules: Vec<String>,
    out: Vec<MessageReport>,
}

impl<'a> Collector<'a> {
    fn marked_public(&self, start_line: usize, sig_line: usize) -> bool {
        let is_marker = |s: &str| {
            let s = s.to_lowercase();
            s.contains("// public") || s.contains("//public")
        };
        let above = start_line
            .checked_sub(2)
            .and_then(|i| self.lines.get(i))
            .map(|l| l.trim_start().starts_with("//") && is_marker(l))
            .unwrap_or(false);
        let same = self
            .lines
            .get(sig_line.saturating_sub(1))
            .map(|l| is_marker(l))
            .unwrap_or(false);
        above || same
    }
}

impl<'ast, 'a> Visit<'ast> for Collector<'a> {
    fn visit_item_mod(&mut self, node: &'ast ItemMod) {
        self.modules.push(node.ident.to_string());
        syn::visit::visit_item_mod(self, node);
        self.modules.pop();
    }

    fn visit_item_impl(&mut self, node: &'ast ItemImpl) {
        let contract = node.self_ty.to_token_stream().to_string().replace(' ', "");
        for item in &node.items {
            let ImplItem::Fn(f) = item else { continue };
            if !is_ink_message(&f.attrs) {
                continue;
            }
            let name = f.sig.ident.to_string();
            let sig_line = f.sig.span().start().line;
            let start_line = f
                .attrs
                .first()
                .map(|a| a.span().start().line)
                .unwrap_or(sig_line);
            let read_only = f
                .sig
                .receiver()
                .map(|r| r.reference.is_some() && r.mutability.is_none())
                .unwrap_or(false);

            let mut visitor = BodyVisitor {
                cfg: self.cfg,
                scan: BodyScan::default(),
            };
            visitor.visit_block(&f.block);
            let scan = visitor.scan;

            let allow_key = format!("{contract}::{name}");
            let (status, reason) = if !scan.guard_calls.is_empty() {
                (
                    Status::Protected,
                    format!("calls guard `{}`", scan.guard_calls[0]),
                )
            } else if scan.reads_caller && scan.access_words {
                (
                    Status::Protected,
                    "compares caller() against an owner/admin/role value".to_string(),
                )
            } else if self.marked_public(start_line, sig_line) {
                (Status::IntentionallyPublic, "marked `// public`".to_string())
            } else if self.cfg.allow.contains(&name) || self.cfg.allow.contains(&allow_key) {
                (Status::Whitelisted, "listed in config".to_string())
            } else if read_only && !self.cfg.include_views {
                (Status::ReadOnly, "takes &self".to_string())
            } else {
                (
                    Status::Unprotected,
                    "no visible access control".to_string(),
                )
            };
            self.out.push(MessageReport {
                file: self.file.to_string(),
                module: self.modules.last().cloned().unwrap_or_default(),
                contract: contract.clone(),
                name,
                line: sig_line,
                status,
                reason,
            });
        }
        syn::visit::visit_item_impl(self, node);
    }
}

/// Audit one file's text.
pub fn audit_source(file: &str, src: &str, cfg: &Config) -> Result<Vec<MessageReport>, String> {
    let ast = syn::parse_file(src).map_err(|e| format!("{file}: {e}"))?;
    let mut collector = Collector {
        file,
        lines: src.lines().collect(),
        cfg,
        modules: Vec::new(),
        out: Vec::new(),
    };
    collector.visit_file(&ast);
    Ok(collector.out)
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Summary {
    pub total: usize,
    pub protected: usize,
    pub public: usize,
    pub whitelisted: usize,
    pub read_only: usize,
    pub unprotected: usize,
}

impl Summary {
    pub fn add(&mut self, status: Status) {
        self.total += 1;
        match status {
            Status::Protected => self.protected += 1,
            Status::IntentionallyPublic => self.public += 1,
            Status::Whitelisted => self.whitelisted += 1,
            Status::ReadOnly => self.read_only += 1,
            Status::Unprotected => self.unprotected += 1,
        }
    }

    /// Messages that count towards coverage (everything except read-only views).
    pub fn counted(&self) -> usize {
        self.total - self.read_only
    }

    /// Percent of counted messages that are protected, public-by-design or whitelisted.
    pub fn coverage_percent(&self) -> f64 {
        if self.counted() == 0 {
            return 100.0;
        }
        (self.protected + self.public + self.whitelisted) as f64 * 100.0 / self.counted() as f64
    }

    /// Percent of counted messages with an actual access-control check.
    pub fn protected_percent(&self) -> f64 {
        if self.counted() == 0 {
            return 100.0;
        }
        self.protected as f64 * 100.0 / self.counted() as f64
    }
}

pub fn summarize<'a>(reports: impl Iterator<Item = &'a MessageReport>) -> Summary {
    let mut s = Summary::default();
    for r in reports {
        s.add(r.status);
    }
    s
}

/// Summaries keyed by contract (`Struct`) and by module, in name order.
pub fn group_summaries(
    reports: &[MessageReport],
) -> (BTreeMap<String, Summary>, BTreeMap<String, Summary>) {
    let mut by_contract: BTreeMap<String, Summary> = BTreeMap::new();
    let mut by_module: BTreeMap<String, Summary> = BTreeMap::new();
    for r in reports {
        by_contract.entry(r.contract.clone()).or_default().add(r.status);
        by_module.entry(r.module.clone()).or_default().add(r.status);
    }
    (by_contract, by_module)
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

fn summary_json(s: &Summary) -> String {
    format!(
        "{{\"total\":{},\"protected\":{},\"public\":{},\"whitelisted\":{},\"read_only\":{},\"unprotected\":{},\"coverage_percent\":{:.1}}}",
        s.total, s.protected, s.public, s.whitelisted, s.read_only, s.unprotected, s.coverage_percent()
    )
}

pub fn to_json(reports: &[MessageReport]) -> String {
    let overall = summarize(reports.iter());
    let (contracts, modules) = group_summaries(reports);
    let map_json = |m: &BTreeMap<String, Summary>| {
        let parts: Vec<String> = m
            .iter()
            .map(|(k, v)| format!("\"{}\":{}", json_escape(k), summary_json(v)))
            .collect();
        format!("{{{}}}", parts.join(","))
    };
    let msgs: Vec<String> = reports
        .iter()
        .map(|r| {
            format!(
                "{{\"file\":\"{}\",\"module\":\"{}\",\"contract\":\"{}\",\"message\":\"{}\",\"line\":{},\"status\":\"{}\",\"reason\":\"{}\"}}",
                json_escape(&r.file),
                json_escape(&r.module),
                json_escape(&r.contract),
                json_escape(&r.name),
                r.line,
                r.status.as_str(),
                json_escape(&r.reason)
            )
        })
        .collect();
    format!(
        "{{\"summary\":{},\"contracts\":{},\"modules\":{},\"messages\":[{}]}}",
        summary_json(&overall),
        map_json(&contracts),
        map_json(&modules),
        msgs.join(",")
    )
}

pub fn to_text(reports: &[MessageReport]) -> String {
    let mut out = String::new();
    for r in reports.iter().filter(|r| r.status == Status::Unprotected) {
        out.push_str(&format!(
            "{}:{}: Unprotected `{}::{}` - {}\n",
            r.file, r.line, r.contract, r.name, r.reason
        ));
    }
    let (contracts, modules) = group_summaries(reports);
    out.push_str("\nPer contract:\n");
    for (name, s) in &contracts {
        out.push_str(&format!(
            "  {name}: {} messages, {} protected, {} public, {} whitelisted, {} unprotected, {} read-only ({:.1}% covered)\n",
            s.total, s.protected, s.public, s.whitelisted, s.unprotected, s.read_only, s.coverage_percent()
        ));
    }
    out.push_str("Per module:\n");
    for (name, s) in &modules {
        out.push_str(&format!(
            "  {name}: {} messages, {:.1}% covered\n",
            s.total,
            s.coverage_percent()
        ));
    }
    let all = summarize(reports.iter());
    out.push_str(&format!(
        "Overall: {} of {} counted messages covered ({:.1}%), {} unprotected\n",
        all.protected + all.public + all.whitelisted,
        all.counted(),
        all.coverage_percent(),
        all.unprotected
    ));
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
    let mut include_views = false;
    let mut fail = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--json" => json = true,
            "--config" => config_path = args.next(),
            "--include-views" => include_views = true,
            "--fail-on-unprotected" => fail = true,
            _ => paths.push(PathBuf::from(a)),
        }
    }
    if paths.is_empty() {
        eprintln!("usage: access-control-audit [--config FILE] [--json] [--include-views] [--fail-on-unprotected] PATH...");
        std::process::exit(2);
    }
    let mut cfg = match config_path {
        Some(p) => Config::parse(&fs::read_to_string(&p).unwrap_or_else(|e| {
            eprintln!("cannot read config {p}: {e}");
            std::process::exit(2)
        })),
        None => Config::default(),
    };
    cfg.include_views = include_views;
    let mut files = Vec::new();
    for p in &paths {
        collect_rs_files(p, &mut files);
    }
    let mut reports = Vec::new();
    for f in &files {
        let name = f.display().to_string();
        match fs::read_to_string(f) {
            Ok(src) => match audit_source(&name, &src, &cfg) {
                Ok(mut r) => reports.append(&mut r),
                Err(e) => eprintln!("warning: skipping {e}"),
            },
            Err(e) => eprintln!("warning: cannot read {name}: {e}"),
        }
    }
    if json {
        println!("{}", to_json(&reports));
    } else {
        print!("{}", to_text(&reports));
    }
    if fail && reports.iter().any(|r| r.status == Status::Unprotected) {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTRACT: &str = r#"
#[ink::contract]
mod vault {
    pub struct Vault { owner: AccountId }
    impl Vault {
        #[ink(message)]
        pub fn set_fee(&mut self, fee: u32) {
            self.ensure_owner()?;
            self.fee = fee;
        }

        #[ink(message)]
        pub fn withdraw(&mut self) {
            let caller = self.env().caller();
            if caller != self.owner { return; }
        }

        // public
        #[ink(message)]
        pub fn deposit(&mut self) { self.total += 1; }

        #[ink(message)]
        pub fn danger(&mut self) { self.total = 0; }

        #[ink(message)]
        pub fn total(&self) -> u32 { self.total }

        pub fn not_a_message(&mut self) { self.total = 0; }
    }
}
"#;

    fn run(cfg: &Config) -> Vec<MessageReport> {
        audit_source("v.rs", CONTRACT, cfg).unwrap()
    }

    fn status_of(reports: &[MessageReport], name: &str) -> Status {
        reports.iter().find(|r| r.name == name).unwrap().status
    }

    #[test]
    fn finds_only_ink_messages() {
        let r = run(&Config::default());
        assert_eq!(r.len(), 5);
        assert!(r.iter().all(|m| m.name != "not_a_message"));
        assert_eq!(r[0].module, "vault");
        assert_eq!(r[0].contract, "Vault");
    }

    #[test]
    fn recognises_guard_calls_and_caller_checks() {
        let r = run(&Config::default());
        assert_eq!(status_of(&r, "set_fee"), Status::Protected);
        assert_eq!(status_of(&r, "withdraw"), Status::Protected);
    }

    #[test]
    fn public_comment_marks_intentionally_public() {
        let r = run(&Config::default());
        assert_eq!(status_of(&r, "deposit"), Status::IntentionallyPublic);
    }

    #[test]
    fn missing_protection_is_flagged() {
        let r = run(&Config::default());
        assert_eq!(status_of(&r, "danger"), Status::Unprotected);
    }

    #[test]
    fn views_are_read_only_unless_requested() {
        assert_eq!(status_of(&run(&Config::default()), "total"), Status::ReadOnly);
        let cfg = Config {
            include_views: true,
            ..Config::default()
        };
        assert_eq!(status_of(&run(&cfg), "total"), Status::Unprotected);
    }

    #[test]
    fn whitelist_accepts_plain_and_qualified_names() {
        let cfg = Config::parse("allow = danger\n");
        assert_eq!(status_of(&run(&cfg), "danger"), Status::Whitelisted);
        let cfg = Config::parse("allow = Vault::danger\n");
        assert_eq!(status_of(&run(&cfg), "danger"), Status::Whitelisted);
    }

    #[test]
    fn custom_guards_from_config_are_recognised() {
        let src = "mod m { impl C { #[ink(message)] pub fn f(&mut self) { self.my_gate(); } } }";
        let before = audit_source("c.rs", src, &Config::default()).unwrap();
        assert_eq!(before[0].status, Status::Unprotected);
        let cfg = Config::parse("guard = my_gate");
        let after = audit_source("c.rs", src, &cfg).unwrap();
        assert_eq!(after[0].status, Status::Protected);
    }

    #[test]
    fn coverage_excludes_read_only_messages() {
        let r = run(&Config::default());
        let s = summarize(r.iter());
        assert_eq!(s.total, 5);
        assert_eq!(s.read_only, 1);
        assert_eq!((s.protected, s.public, s.unprotected), (2, 1, 1));
        // 3 of 4 counted messages are covered.
        assert!((s.coverage_percent() - 75.0).abs() < 1e-9);
        assert!((s.protected_percent() - 50.0).abs() < 1e-9);
    }

    #[test]
    fn empty_input_is_fully_covered() {
        assert!((Summary::default().coverage_percent() - 100.0).abs() < 1e-9);
    }

    #[test]
    fn per_contract_and_module_summaries_are_grouped() {
        let r = run(&Config::default());
        let (contracts, modules) = group_summaries(&r);
        assert_eq!(contracts["Vault"].total, 5);
        assert_eq!(modules["vault"].unprotected, 1);
    }

    #[test]
    fn json_report_contains_summary_and_messages() {
        let json = to_json(&run(&Config::default()));
        assert!(json.contains("\"coverage_percent\":75.0"));
        assert!(json.contains("\"message\":\"danger\""));
        assert!(json.contains("\"status\":\"Unprotected\""));
    }

    #[test]
    fn unparsable_source_is_an_error() {
        assert!(audit_source("bad.rs", "mod {", &Config::default()).is_err());
    }
}
