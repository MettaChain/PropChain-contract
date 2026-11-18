//! Event emission completeness checker for ink! contracts (#1469).
//!
//! For every `#[ink(message)]` it decides whether the message changes state and
//! whether it emits an event:
//!
//! * **State-changing** - takes `&mut self`, or its body assigns to `self.*`, or
//!   calls a mutating method (`insert`, `remove`, `take`, `push`, `push_back`,
//!   `pop`, `clear`, `set`, `transfer`) on a `self.*` expression.
//! * **Emits** - the body calls `emit_event` or any function/method whose name
//!   starts with `emit` (helpers such as `self.emit_transfer(..)`).
//!
//! Statuses: `Emits`, `MissingEvent` (state-changing without an event), `View`
//! (not state-changing) and `Whitelisted` (listed in the config).
//!
//! ```text
//! event-completeness-checker [--config FILE] [--format json|markdown|text] [--fail-on-missing] PATH...
//! ```
//!
//! Config: `allow = message` or `allow = Contract::message` lines, `#` comments.
//! Syntactic only: a mutation hidden inside a helper that takes `&mut self` is
//! seen only through the message's own receiver.

use quote::ToTokens;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use syn::spanned::Spanned;
use syn::visit::Visit;
use syn::{Expr, ExprAssign, ExprBinary, ExprCall, ExprMethodCall, ImplItem, ItemImpl, ItemMod};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Status {
    Emits,
    MissingEvent,
    View,
    Whitelisted,
}

impl Status {
    fn as_str(self) -> &'static str {
        match self {
            Status::Emits => "Emits",
            Status::MissingEvent => "MissingEvent",
            Status::View => "View",
            Status::Whitelisted => "Whitelisted",
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
    pub state_changing: bool,
    pub emits_event: bool,
    pub status: Status,
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub allow: HashSet<String>,
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
                if key.trim() == "allow" {
                    cfg.allow.insert(value.trim().to_string());
                }
            }
        }
        cfg
    }
}

const MUTATING_METHODS: &[&str] = &[
    "insert", "remove", "take", "push", "push_back", "pop", "clear", "set", "transfer",
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

#[derive(Default)]
struct BodyScan {
    mutates: bool,
    emits: bool,
}

impl<'ast> Visit<'ast> for BodyScan {
    fn visit_expr_assign(&mut self, node: &'ast ExprAssign) {
        if is_self_rooted(&node.left) {
            self.mutates = true;
        }
        syn::visit::visit_expr_assign(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast ExprBinary) {
        use syn::BinOp::*;
        let compound = matches!(
            node.op,
            AddAssign(_) | SubAssign(_) | MulAssign(_) | DivAssign(_) | RemAssign(_)
        );
        if compound && is_self_rooted(&node.left) {
            self.mutates = true;
        }
        syn::visit::visit_expr_binary(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let name = node.method.to_string();
        if name == "emit_event" || name.starts_with("emit") {
            self.emits = true;
        }
        if MUTATING_METHODS.contains(&name.as_str()) && is_self_rooted(&node.receiver) {
            self.mutates = true;
        }
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(p) = &*node.func {
            if let Some(seg) = p.path.segments.last() {
                let name = seg.ident.to_string();
                if name == "emit_event" || name.starts_with("emit") {
                    self.emits = true;
                }
            }
        }
        syn::visit::visit_expr_call(self, node);
    }
}

fn is_ink_message(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("ink") && a.meta.to_token_stream().to_string().contains("message")
    })
}

struct Collector<'a> {
    file: &'a str,
    cfg: &'a Config,
    modules: Vec<String>,
    out: Vec<MessageReport>,
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
            let mut scan = BodyScan::default();
            scan.visit_block(&f.block);
            let mut_receiver = f
                .sig
                .receiver()
                .map(|r| r.reference.is_some() && r.mutability.is_some())
                .unwrap_or(false);
            let state_changing = mut_receiver || scan.mutates;
            let key = format!("{contract}::{name}");
            let status = if self.cfg.allow.contains(&name) || self.cfg.allow.contains(&key) {
                Status::Whitelisted
            } else if !state_changing {
                Status::View
            } else if scan.emits {
                Status::Emits
            } else {
                Status::MissingEvent
            };
            self.out.push(MessageReport {
                file: self.file.to_string(),
                module: self.modules.last().cloned().unwrap_or_default(),
                contract: contract.clone(),
                name,
                line: f.sig.span().start().line,
                state_changing,
                emits_event: scan.emits,
                status,
            });
        }
        syn::visit::visit_item_impl(self, node);
    }
}

/// Check one file's text.
pub fn check_source(file: &str, src: &str, cfg: &Config) -> Result<Vec<MessageReport>, String> {
    let ast = syn::parse_file(src).map_err(|e| format!("{file}: {e}"))?;
    let mut c = Collector {
        file,
        cfg,
        modules: Vec::new(),
        out: Vec::new(),
    };
    c.visit_file(&ast);
    Ok(c.out)
}

pub fn missing(reports: &[MessageReport]) -> Vec<&MessageReport> {
    reports
        .iter()
        .filter(|r| r.status == Status::MissingEvent)
        .collect()
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

pub fn to_json(reports: &[MessageReport]) -> String {
    let count = |s: Status| reports.iter().filter(|r| r.status == s).count();
    let msgs: Vec<String> = reports
        .iter()
        .map(|r| {
            format!(
                "{{\"file\":\"{}\",\"module\":\"{}\",\"contract\":\"{}\",\"message\":\"{}\",\"line\":{},\"state_changing\":{},\"emits_event\":{},\"status\":\"{}\"}}",
                json_escape(&r.file),
                json_escape(&r.module),
                json_escape(&r.contract),
                json_escape(&r.name),
                r.line,
                r.state_changing,
                r.emits_event,
                r.status.as_str()
            )
        })
        .collect();
    format!(
        "{{\"summary\":{{\"total\":{},\"emits\":{},\"missing_event\":{},\"view\":{},\"whitelisted\":{}}},\"messages\":[{}]}}",
        reports.len(),
        count(Status::Emits),
        count(Status::MissingEvent),
        count(Status::View),
        count(Status::Whitelisted),
        msgs.join(",")
    )
}

pub fn to_markdown(reports: &[MessageReport]) -> String {
    let mut out = String::from("# Event emission report\n\n");
    out.push_str("| Contract | Message | Line | State-changing | Status |\n|---|---|---|---|---|\n");
    for r in reports {
        out.push_str(&format!(
            "| {} | `{}` | {} | {} | {} |\n",
            r.contract,
            r.name,
            r.line,
            if r.state_changing { "yes" } else { "no" },
            r.status.as_str()
        ));
    }
    let m = missing(reports);
    out.push_str(&format!(
        "\n{} message(s) total, {} missing an event.\n",
        reports.len(),
        m.len()
    ));
    out
}

pub fn to_text(reports: &[MessageReport]) -> String {
    let mut out = String::new();
    for r in reports {
        out.push_str(&format!(
            "{}:{}: {}::{} {}\n",
            r.file,
            r.line,
            r.contract,
            r.name,
            r.status.as_str()
        ));
    }
    out.push_str(&format!("{} missing an event\n", missing(reports).len()));
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
    let mut fail = false;
    while let Some(a) = args.next() {
        match a.as_str() {
            "--format" => format = args.next().unwrap_or_default(),
            "--config" => config_path = args.next(),
            "--fail-on-missing" => fail = true,
            _ => paths.push(PathBuf::from(a)),
        }
    }
    if paths.is_empty() {
        eprintln!("usage: event-completeness-checker [--config FILE] [--format json|markdown|text] [--fail-on-missing] PATH...");
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
    let mut reports = Vec::new();
    for f in &files {
        let name = f.display().to_string();
        match fs::read_to_string(f) {
            Ok(src) => match check_source(&name, &src, &cfg) {
                Ok(mut r) => reports.append(&mut r),
                Err(e) => eprintln!("warning: skipping {e}"),
            },
            Err(e) => eprintln!("warning: cannot read {name}: {e}"),
        }
    }
    match format.as_str() {
        "json" => println!("{}", to_json(&reports)),
        "markdown" | "md" => print!("{}", to_markdown(&reports)),
        _ => print!("{}", to_text(&reports)),
    }
    if fail && !missing(&reports).is_empty() {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"
mod token {
    impl Token {
        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, v: u128) {
            self.balances.insert(to, &v);
            self.env().emit_event(Transferred { to, v });
        }

        #[ink(message)]
        pub fn burn(&mut self, v: u128) {
            self.supply -= v;
        }

        #[ink(message)]
        pub fn balance_of(&self, who: AccountId) -> u128 {
            self.balances.get(who).unwrap_or(0)
        }

        #[ink(message)]
        pub fn approve(&mut self, who: AccountId) {
            self.allowed.insert(who, &true);
            self.emit_approval(who);
        }

        #[ink(message)]
        pub fn touch(&mut self) {}
    }
}
"#;

    fn run(cfg: &Config) -> Vec<MessageReport> {
        check_source("t.rs", SRC, cfg).unwrap()
    }

    fn get<'a>(r: &'a [MessageReport], n: &str) -> &'a MessageReport {
        r.iter().find(|m| m.name == n).unwrap()
    }

    #[test]
    fn state_change_with_event_is_ok() {
        let r = run(&Config::default());
        assert_eq!(get(&r, "transfer").status, Status::Emits);
    }

    #[test]
    fn state_change_without_event_is_missing() {
        let r = run(&Config::default());
        let burn = get(&r, "burn");
        assert!(burn.state_changing && !burn.emits_event);
        assert_eq!(burn.status, Status::MissingEvent);
    }

    #[test]
    fn views_are_excluded() {
        let r = run(&Config::default());
        assert_eq!(get(&r, "balance_of").status, Status::View);
        assert!(!get(&r, "balance_of").state_changing);
    }

    #[test]
    fn emit_helpers_count_as_emitting() {
        let r = run(&Config::default());
        assert_eq!(get(&r, "approve").status, Status::Emits);
    }

    #[test]
    fn mut_receiver_alone_counts_as_state_changing() {
        let r = run(&Config::default());
        assert_eq!(get(&r, "touch").status, Status::MissingEvent);
    }

    #[test]
    fn whitelist_overrides_missing_event() {
        let cfg = Config::parse("# intentionally silent\nallow = burn\nallow = Token::touch\n");
        let r = run(&cfg);
        assert_eq!(get(&r, "burn").status, Status::Whitelisted);
        assert_eq!(get(&r, "touch").status, Status::Whitelisted);
        assert!(missing(&r).is_empty());
    }

    #[test]
    fn mapping_write_on_a_ref_receiver_is_state_changing() {
        let src = "mod m { impl C { #[ink(message)] pub fn f(&self) { self.map.insert(1, &2); } } }";
        let r = check_source("c.rs", src, &Config::default()).unwrap();
        assert!(r[0].state_changing);
        assert_eq!(r[0].status, Status::MissingEvent);
    }

    #[test]
    fn non_messages_are_ignored() {
        let src = "mod m { impl C { pub fn helper(&mut self) { self.x = 1; } } }";
        assert!(check_source("c.rs", src, &Config::default()).unwrap().is_empty());
    }

    #[test]
    fn markdown_and_json_reports_list_every_message() {
        let r = run(&Config::default());
        let md = to_markdown(&r);
        assert!(md.contains("| Token | `burn` |"));
        assert!(md.contains("2 missing an event"));
        let json = to_json(&r);
        assert!(json.contains("\"missing_event\":2"));
        assert!(json.contains("\"message\":\"transfer\""));
    }

    #[test]
    fn unparsable_source_is_an_error() {
        assert!(check_source("bad.rs", "impl {", &Config::default()).is_err());
    }
}
