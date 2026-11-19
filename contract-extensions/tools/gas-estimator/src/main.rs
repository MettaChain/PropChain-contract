//! Static gas estimator for ink! messages (#1473).
//!
//! Parses contract sources with `syn` (nothing is compiled or executed) and, for every
//! `#[ink(message)]`, counts the operations that dominate `pallet-contracts` weight:
//!
//! * **storage reads**: `self.<field>.get/contains/len/size/...` and plain reads of `self.<field>`,
//! * **storage writes**: `insert/remove/set/push/clear/...` on a `self.<field>`, and assignments to it
//!   (`take` counts as a read and a write),
//! * **external calls**: `build_call`, `build_create`, `.invoke()`, `.try_invoke()`, `.call_v1()`,
//!   `.delegate_call()` and native `transfer`,
//! * **event emissions**: `emit_event`,
//! * **compute**: every other expression-level operation.
//!
//! Operations inside `for` / `while` / `loop` are multiplied by the assumed iteration count
//! (nested loops multiply), because the real count depends on runtime data the analysis cannot see.
//!
//! The totals are turned into gas with a configurable cost model whose defaults are of the order of
//! `pallet-contracts` reference weights. **These are approximations from code shape, not measurements;**
//! use them to rank messages and spot outliers, and measure real execution before relying on a number.
//!
//! ```text
//! gas-estimator [--config FILE] [--set key=value]... [--iterations N] [--format text|json] FILE...
//! ```
//! Config keys: `storage_read`, `storage_write`, `external_call`, `event_emit`, `compute_op`,
//! `call_overhead`, `block_gas_limit`, `loop_iterations` (one `key = value` per line, `#` comments).
//! Exit code 1 if any message may exceed the block gas limit, 2 on usage or parse errors.

use std::fs;
use std::process::ExitCode;
use syn::visit::Visit;
use syn::{Attribute, BinOp, Expr, ExprAssign, ExprBinary, ExprCall, ExprField, ExprForLoop, ExprLoop, ExprMethodCall, ExprWhile, ImplItemFn};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CostModel {
    pub storage_read: u64,
    pub storage_write: u64,
    pub external_call: u64,
    pub event_emit: u64,
    pub compute_op: u64,
    pub call_overhead: u64,
    pub block_gas_limit: u64,
    pub loop_iterations: u64,
}

impl Default for CostModel {
    fn default() -> Self {
        Self {
            storage_read: 30_000_000,
            storage_write: 90_000_000,
            external_call: 250_000_000,
            event_emit: 40_000_000,
            compute_op: 50_000,
            call_overhead: 120_000_000,
            block_gas_limit: 1_500_000_000_000,
            loop_iterations: 10,
        }
    }
}

impl CostModel {
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        let v: u64 = value.trim().parse().map_err(|_| format!("`{key}`: `{value}` is not a non-negative integer"))?;
        match key.trim() {
            "storage_read" => self.storage_read = v,
            "storage_write" => self.storage_write = v,
            "external_call" => self.external_call = v,
            "event_emit" => self.event_emit = v,
            "compute_op" => self.compute_op = v,
            "call_overhead" => self.call_overhead = v,
            "block_gas_limit" => self.block_gas_limit = v,
            "loop_iterations" => {
                if v == 0 {
                    return Err("`loop_iterations` must be at least 1".into());
                }
                self.loop_iterations = v
            }
            other => return Err(format!("unknown cost model key `{other}`")),
        }
        Ok(())
    }

    /// Applies `key = value` lines (blank lines and `#` comments are ignored).
    pub fn apply_config(&mut self, text: &str) -> Result<(), String> {
        for (n, line) in text.lines().enumerate() {
            let line = line.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let (k, v) = line.split_once('=').ok_or_else(|| format!("line {}: expected `key = value`", n + 1))?;
            self.set(k, v).map_err(|e| format!("line {}: {e}", n + 1))?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Counts {
    pub reads: u64,
    pub writes: u64,
    pub external_calls: u64,
    pub events: u64,
    pub ops: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Breakdown {
    pub storage: u64,
    pub compute: u64,
    pub calls: u64,
    pub events: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MessageEstimate {
    pub contract_file: String,
    pub message: String,
    pub counts: Counts,
    pub loops: u32,
    pub max_loop_depth: u32,
    pub gas: u64,
    pub breakdown: Breakdown,
    pub exceeds_block_limit: bool,
    pub suggestions: Vec<String>,
}

const EXTERNAL_CALLS: &[&str] = &["invoke", "try_invoke", "call_v1", "delegate_call", "transfer"];
const STORAGE_READS: &[&str] = &["get", "contains", "len", "size", "is_empty", "iter", "keys", "values"];
const STORAGE_WRITES: &[&str] =
    &["insert", "remove", "set", "push", "push_back", "pop", "clear", "extend", "retain", "swap", "append"];

fn is_ink_message(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("ink")
            && a.meta
                .require_list()
                .map(|l| l.tokens.to_string().split(',').any(|t| t.trim() == "message"))
                .unwrap_or(false)
    })
}

fn is_self(expr: &Expr) -> bool {
    matches!(expr, Expr::Path(p) if p.path.is_ident("self"))
}

/// `true` for `self.field`, `self.field[i]`, `(&self.field)` and `self.a.b`; false once a method call intervenes.
fn rooted_in_self_field(expr: &Expr) -> bool {
    let mut cur = expr;
    loop {
        match cur {
            Expr::Field(f) => {
                if is_self(&f.base) {
                    return true;
                }
                cur = &f.base;
            }
            Expr::Index(i) => cur = &i.expr,
            Expr::Paren(p) => cur = &p.expr,
            Expr::Reference(r) => cur = &r.expr,
            _ => return false,
        }
    }
}

struct Analyzer {
    iterations: u64,
    weight: u64,
    weights: Vec<u64>,
    counts: Counts,
    loops: u32,
    max_depth: u32,
    writes_in_loop: bool,
    reads_in_loop: bool,
    calls_in_loop: bool,
}

impl Analyzer {
    fn new(iterations: u64) -> Self {
        Self {
            iterations,
            weight: 1,
            weights: Vec::new(),
            counts: Counts::default(),
            loops: 0,
            max_depth: 0,
            writes_in_loop: false,
            reads_in_loop: false,
            calls_in_loop: false,
        }
    }

    fn in_loop(&self) -> bool {
        !self.weights.is_empty()
    }

    fn enter_loop(&mut self) {
        self.weights.push(self.weight);
        self.weight = self.weight.saturating_mul(self.iterations);
        self.loops += 1;
        self.max_depth = self.max_depth.max(self.weights.len() as u32);
    }

    fn leave_loop(&mut self) {
        if let Some(w) = self.weights.pop() {
            self.weight = w;
        }
    }

    fn read(&mut self) {
        self.counts.reads = self.counts.reads.saturating_add(self.weight);
        self.reads_in_loop |= self.in_loop();
    }

    fn write(&mut self) {
        self.counts.writes = self.counts.writes.saturating_add(self.weight);
        self.writes_in_loop |= self.in_loop();
    }

    fn external(&mut self) {
        self.counts.external_calls = self.counts.external_calls.saturating_add(self.weight);
        self.calls_in_loop |= self.in_loop();
    }

    fn op(&mut self) {
        self.counts.ops = self.counts.ops.saturating_add(self.weight);
    }
}

impl<'ast> Visit<'ast> for Analyzer {
    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        let name = m.method.to_string();
        let on_storage = rooted_in_self_field(&m.receiver);
        if EXTERNAL_CALLS.contains(&name.as_str()) {
            self.external();
        } else if name == "emit_event" {
            self.counts.events = self.counts.events.saturating_add(self.weight);
        } else if on_storage && name == "take" {
            self.read();
            self.write();
        } else if on_storage && STORAGE_READS.contains(&name.as_str()) {
            self.read();
        } else if on_storage && STORAGE_WRITES.contains(&name.as_str()) {
            self.write();
        } else {
            self.op();
        }
        let storage_op = on_storage
            && (name == "take" || STORAGE_READS.contains(&name.as_str()) || STORAGE_WRITES.contains(&name.as_str()));
        if storage_op {
            // The receiver is the storage access that was just counted: only look at the arguments.
            for arg in &m.args {
                self.visit_expr(arg);
            }
        } else {
            syn::visit::visit_expr_method_call(self, m);
        }
    }

    fn visit_expr_call(&mut self, c: &'ast ExprCall) {
        let callee = match &*c.func {
            Expr::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
            _ => None,
        };
        match callee.as_deref() {
            Some("build_call") | Some("build_create") => self.external(),
            _ => self.op(),
        }
        syn::visit::visit_expr_call(self, c);
    }

    fn visit_expr_assign(&mut self, a: &'ast ExprAssign) {
        if rooted_in_self_field(&a.left) {
            self.write();
            self.visit_expr(&a.right);
        } else {
            self.op();
            syn::visit::visit_expr_assign(self, a);
        }
    }

    fn visit_expr_binary(&mut self, b: &'ast ExprBinary) {
        let compound = matches!(
            b.op,
            BinOp::AddAssign(_)
                | BinOp::SubAssign(_)
                | BinOp::MulAssign(_)
                | BinOp::DivAssign(_)
                | BinOp::RemAssign(_)
                | BinOp::BitXorAssign(_)
                | BinOp::BitAndAssign(_)
                | BinOp::BitOrAssign(_)
                | BinOp::ShlAssign(_)
                | BinOp::ShrAssign(_)
        );
        if compound && rooted_in_self_field(&b.left) {
            self.read();
            self.write();
            self.op();
            self.visit_expr(&b.right);
        } else {
            self.op();
            syn::visit::visit_expr_binary(self, b);
        }
    }

    fn visit_expr_field(&mut self, f: &'ast ExprField) {
        if is_self(&f.base) {
            self.read();
        } else {
            syn::visit::visit_expr_field(self, f);
        }
    }

    fn visit_expr_for_loop(&mut self, l: &'ast ExprForLoop) {
        self.visit_expr(&l.expr);
        self.enter_loop();
        self.visit_block(&l.body);
        self.leave_loop();
    }

    fn visit_expr_while(&mut self, w: &'ast ExprWhile) {
        self.enter_loop();
        self.visit_expr(&w.cond);
        self.visit_block(&w.body);
        self.leave_loop();
    }

    fn visit_expr_loop(&mut self, l: &'ast ExprLoop) {
        self.enter_loop();
        self.visit_block(&l.body);
        self.leave_loop();
    }
}

fn gas_of(counts: &Counts, model: &CostModel) -> (u64, Breakdown) {
    let storage = counts
        .reads
        .saturating_mul(model.storage_read)
        .saturating_add(counts.writes.saturating_mul(model.storage_write));
    let compute = counts.ops.saturating_mul(model.compute_op);
    let calls = counts.external_calls.saturating_mul(model.external_call).saturating_add(model.call_overhead);
    let events = counts.events.saturating_mul(model.event_emit);
    let total = storage.saturating_add(compute).saturating_add(calls).saturating_add(events);
    (total, Breakdown { storage, compute, calls, events })
}

fn suggestions(a: &Analyzer, counts: &Counts, exceeds: bool) -> Vec<String> {
    let mut s = Vec::new();
    if a.writes_in_loop {
        s.push("storage writes inside a loop: batch the updates or cap the iteration count".to_string());
    }
    if a.calls_in_loop {
        s.push("external calls inside a loop: each iteration pays the full call cost and widens the reentrancy surface".to_string());
    }
    if a.reads_in_loop {
        s.push("storage reads inside a loop: read the value once into a local variable".to_string());
    }
    if a.max_depth >= 2 {
        s.push("nested loops: cost grows multiplicatively with each level".to_string());
    }
    if counts.writes >= 5 && !a.writes_in_loop {
        s.push("many storage writes: consider packing related fields into one stored struct".to_string());
    }
    if counts.external_calls > 0 && !a.calls_in_loop {
        s.push("external call: do it last (checks-effects-interactions) and handle its failure".to_string());
    }
    if exceeds {
        s.push("estimate exceeds the configured block gas limit: split the work or bound the loops".to_string());
    }
    s
}

struct MessageCollector<'a> {
    file: &'a str,
    model: &'a CostModel,
    out: Vec<MessageEstimate>,
}

impl<'ast> Visit<'ast> for MessageCollector<'_> {
    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        if is_ink_message(&f.attrs) {
            let mut a = Analyzer::new(self.model.loop_iterations);
            a.visit_block(&f.block);
            let (gas, breakdown) = gas_of(&a.counts, self.model);
            let exceeds = gas > self.model.block_gas_limit;
            let suggestions = suggestions(&a, &a.counts, exceeds);
            self.out.push(MessageEstimate {
                contract_file: self.file.to_string(),
                message: f.sig.ident.to_string(),
                counts: a.counts.clone(),
                loops: a.loops,
                max_loop_depth: a.max_depth,
                gas,
                breakdown,
                exceeds_block_limit: exceeds,
                suggestions,
            });
        }
        syn::visit::visit_impl_item_fn(self, f);
    }
}

/// Estimates every `#[ink(message)]` in `source`.
pub fn estimate_source(file: &str, source: &str, model: &CostModel) -> Result<Vec<MessageEstimate>, String> {
    let ast = syn::parse_file(source).map_err(|e| format!("{file}: parse error: {e}"))?;
    let mut c = MessageCollector { file, model, out: Vec::new() };
    c.visit_file(&ast);
    Ok(c.out)
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

pub fn render_json(model: &CostModel, estimates: &[MessageEstimate]) -> String {
    let items = estimates
        .iter()
        .map(|e| {
            let sug = e.suggestions.iter().map(|s| format!("\"{}\"", json_escape(s))).collect::<Vec<_>>().join(",");
            format!(
                "{{\"file\":\"{}\",\"message\":\"{}\",\"gas\":{},\"exceeds_block_limit\":{},\"breakdown\":{{\"storage\":{},\"compute\":{},\"calls\":{},\"events\":{}}},\"counts\":{{\"reads\":{},\"writes\":{},\"external_calls\":{},\"events\":{},\"ops\":{}}},\"loops\":{},\"max_loop_depth\":{},\"suggestions\":[{}]}}",
                json_escape(&e.contract_file),
                json_escape(&e.message),
                e.gas,
                e.exceeds_block_limit,
                e.breakdown.storage,
                e.breakdown.compute,
                e.breakdown.calls,
                e.breakdown.events,
                e.counts.reads,
                e.counts.writes,
                e.counts.external_calls,
                e.counts.events,
                e.counts.ops,
                e.loops,
                e.max_loop_depth,
                sug
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"block_gas_limit\":{},\"loop_iterations\":{},\"estimates\":[{}]}}\n",
        model.block_gas_limit, model.loop_iterations, items
    )
}

pub fn render_text(model: &CostModel, estimates: &[MessageEstimate]) -> String {
    let mut sorted: Vec<&MessageEstimate> = estimates.iter().collect();
    sorted.sort_by(|a, b| b.gas.cmp(&a.gas));
    let mut out = format!(
        "Gas estimates (approximate, static analysis; loops assumed to run {} times; block limit {})\n\n",
        model.loop_iterations, model.block_gas_limit
    );
    out.push_str(&format!(
        "{:<28} {:>16} {:>14} {:>14} {:>14} {:>12}\n",
        "message", "gas", "storage", "compute", "calls", "events"
    ));
    for e in &sorted {
        out.push_str(&format!(
            "{:<28} {:>16} {:>14} {:>14} {:>14} {:>12}{}\n",
            e.message,
            e.gas,
            e.breakdown.storage,
            e.breakdown.compute,
            e.breakdown.calls,
            e.breakdown.events,
            if e.exceeds_block_limit { "  EXCEEDS BLOCK LIMIT" } else { "" }
        ));
    }
    for e in sorted.iter().filter(|e| !e.suggestions.is_empty()) {
        out.push_str(&format!("\n{} ({} reads, {} writes, {} external calls):\n", e.message, e.counts.reads, e.counts.writes, e.counts.external_calls));
        for s in &e.suggestions {
            out.push_str(&format!("  - {s}\n"));
        }
    }
    out
}

fn usage() -> ExitCode {
    eprintln!("usage: gas-estimator [--config FILE] [--set key=value]... [--iterations N] [--format text|json] FILE...");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut model = CostModel::default();
    let mut files = Vec::new();
    let mut format = "text".to_string();
    let mut i = 0;
    while i < args.len() {
        let value = args.get(i + 1).cloned();
        match args[i].as_str() {
            "--config" => {
                let Some(path) = value else { return usage() };
                let applied = fs::read_to_string(&path).map_err(|e| format!("{path}: {e}")).and_then(|t| model.apply_config(&t));
                if let Err(e) = applied {
                    eprintln!("error: {e}");
                    return ExitCode::from(2);
                }
            }
            "--set" => {
                let Some((k, v)) = value.as_deref().and_then(|s| s.split_once('=')) else { return usage() };
                if let Err(e) = model.set(k, v) {
                    eprintln!("error: {e}");
                    return ExitCode::from(2);
                }
            }
            "--iterations" => {
                let Some(v) = value else { return usage() };
                if let Err(e) = model.set("loop_iterations", &v) {
                    eprintln!("error: {e}");
                    return ExitCode::from(2);
                }
            }
            "--format" => format = value.unwrap_or_default(),
            s if s.starts_with("--") => return usage(),
            _ => {
                files.push(args[i].clone());
                i += 1;
                continue;
            }
        }
        i += 2;
    }
    if files.is_empty() || (format != "text" && format != "json") {
        return usage();
    }

    let mut estimates = Vec::new();
    for file in &files {
        let result = fs::read_to_string(file)
            .map_err(|e| format!("{file}: {e}"))
            .and_then(|src| estimate_source(file, &src, &model));
        match result {
            Ok(mut e) => estimates.append(&mut e),
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        }
    }
    if format == "json" {
        print!("{}", render_json(&model, &estimates));
    } else {
        print!("{}", render_text(&model, &estimates));
    }
    if estimates.iter().any(|e| e.exceeds_block_limit) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(body: &str) -> String {
        format!(
            "#[ink::contract]\nmod c {{\n    impl C {{\n        #[ink(message)]\n        pub fn m(&mut self, n: u32) {{\n{body}\n        }}\n        pub fn helper(&self) {{ self.total = 1; }}\n    }}\n}}\n"
        )
    }

    fn estimate(body: &str) -> MessageEstimate {
        let mut v = estimate_source("c.rs", &contract(body), &CostModel::default()).unwrap();
        assert_eq!(v.len(), 1, "only #[ink(message)] fns are estimated");
        v.remove(0)
    }

    #[test]
    fn finds_only_messages() {
        let src = "impl C { #[ink(message)] pub fn a(&self) {} #[ink(constructor)] pub fn new() -> Self { Self } pub fn b(&self) {} }";
        let v = estimate_source("c.rs", src, &CostModel::default()).unwrap();
        assert_eq!(v.iter().map(|e| e.message.as_str()).collect::<Vec<_>>(), vec!["a"]);
    }

    #[test]
    fn an_empty_message_costs_only_the_call_overhead() {
        let e = estimate("");
        assert_eq!(e.counts, Counts::default());
        assert_eq!(e.gas, CostModel::default().call_overhead);
    }

    #[test]
    fn counts_storage_reads() {
        let e = estimate("let _a = self.balances.get(n); let _b = self.balances.contains(n);");
        assert_eq!(e.counts.reads, 2);
        assert_eq!(e.counts.writes, 0);
    }

    #[test]
    fn counts_storage_writes_from_methods_and_assignments() {
        let e = estimate("self.balances.insert(n, &1); self.total = 5; self.count += 1;");
        assert_eq!(e.counts.writes, 3);
        assert!(e.counts.reads >= 1, "`+=` reads the field first");
    }

    #[test]
    fn take_is_a_read_and_a_write() {
        let e = estimate("let _x = self.pending.take(n);");
        assert_eq!((e.counts.reads, e.counts.writes), (1, 1));
    }

    #[test]
    fn counts_external_calls_and_events() {
        let e = estimate("build_call::<E>().invoke(); self.env().emit_event(Ev {}); self.env().transfer(a, 1);");
        assert_eq!(e.counts.external_calls, 3); // build_call, invoke, transfer
        assert_eq!(e.counts.events, 1);
    }

    #[test]
    fn local_method_calls_are_compute_not_storage() {
        let e = estimate("let v = Vec::new(); v.push(1); v.len();");
        assert_eq!(e.counts.reads, 0);
        assert_eq!(e.counts.writes, 0);
        assert!(e.counts.ops > 0);
    }

    #[test]
    fn loops_multiply_by_the_assumed_iterations() {
        let e = estimate("for i in 0..n { self.balances.insert(i, &1); }");
        assert_eq!(e.counts.writes, 10);
        assert_eq!(e.loops, 1);
        assert_eq!(e.max_loop_depth, 1);
        assert!(e.suggestions.iter().any(|s| s.contains("writes inside a loop")));
    }

    #[test]
    fn nested_loops_multiply_and_are_flagged() {
        let e = estimate("for i in 0..n { for j in 0..n { self.balances.insert(i, &j); } }");
        assert_eq!(e.counts.writes, 100);
        assert_eq!(e.max_loop_depth, 2);
        assert!(e.suggestions.iter().any(|s| s.contains("nested loops")));
    }

    #[test]
    fn while_and_loop_count_as_loops() {
        let w = estimate("while self.pending.len() > 0 { self.pending.pop(); }");
        assert_eq!(w.loops, 1);
        assert_eq!(w.counts.writes, 10);
        let l = estimate("loop { self.count += 1; break; }");
        assert_eq!(l.loops, 1);
    }

    #[test]
    fn work_after_a_loop_is_not_multiplied() {
        let e = estimate("for i in 0..n { let _ = i; } self.total = 1;");
        assert_eq!(e.counts.writes, 1);
    }

    #[test]
    fn the_iteration_assumption_is_configurable() {
        let mut model = CostModel::default();
        model.set("loop_iterations", "3").unwrap();
        let v = estimate_source("c.rs", &contract("for i in 0..n { self.m.insert(i, &1); }"), &model).unwrap();
        assert_eq!(v[0].counts.writes, 3);
    }

    #[test]
    fn gas_follows_the_cost_model_and_the_breakdown_adds_up() {
        let mut model = CostModel::default();
        for (k, v) in [("storage_read", "10"), ("storage_write", "100"), ("external_call", "1000"), ("event_emit", "7"), ("compute_op", "0"), ("call_overhead", "5")] {
            model.set(k, v).unwrap();
        }
        let v = estimate_source(
            "c.rs",
            &contract("let _a = self.m.get(n); self.m.insert(n, &1); build_call::<E>().invoke(); self.env().emit_event(Ev {});"),
            &model,
        )
        .unwrap();
        let e = &v[0];
        // 1 read, 1 write, 2 external (build_call + invoke), 1 event, overhead 5
        assert_eq!(e.breakdown, Breakdown { storage: 10 + 100, compute: 0, calls: 2 * 1000 + 5, events: 7 });
        assert_eq!(e.gas, 110 + 2005 + 7);
    }

    #[test]
    fn flags_messages_over_the_block_gas_limit() {
        let mut model = CostModel::default();
        model.set("block_gas_limit", "1000").unwrap();
        let v = estimate_source("c.rs", &contract("self.total = 1;"), &model).unwrap();
        assert!(v[0].exceeds_block_limit);
        assert!(v[0].suggestions.iter().any(|s| s.contains("block gas limit")));
        let ok = estimate_source("c.rs", &contract("self.total = 1;"), &CostModel::default()).unwrap();
        assert!(!ok[0].exceeds_block_limit);
    }

    #[test]
    fn suggests_keeping_the_external_call_last() {
        let e = estimate("build_call::<E>().invoke();");
        assert!(e.suggestions.iter().any(|s| s.contains("external call")));
    }

    #[test]
    fn parses_config_text() {
        let mut m = CostModel::default();
        m.apply_config("# comment\nstorage_read = 5\n\nstorage_write=9 # trailing\n").unwrap();
        assert_eq!((m.storage_read, m.storage_write), (5, 9));
    }

    #[test]
    fn rejects_bad_config() {
        let mut m = CostModel::default();
        assert!(m.apply_config("bogus = 1").is_err());
        assert!(m.apply_config("storage_read = x").is_err());
        assert!(m.apply_config("no equals sign").is_err());
        assert!(m.set("loop_iterations", "0").is_err());
    }

    #[test]
    fn reports_parse_errors() {
        assert!(estimate_source("bad.rs", "fn {", &CostModel::default()).is_err());
    }

    #[test]
    fn renders_text_and_json() {
        let model = CostModel::default();
        let v = estimate_source("c.rs", &contract("for i in 0..n { self.m.insert(i, &1); }"), &model).unwrap();
        let text = render_text(&model, &v);
        assert!(text.contains("m "));
        assert!(text.contains("storage writes inside a loop"));
        let json = render_json(&model, &v);
        assert!(json.starts_with("{\"block_gas_limit\":"));
        assert!(json.contains("\"message\":\"m\""));
        assert!(json.contains("\"writes\":10"));
    }
}
