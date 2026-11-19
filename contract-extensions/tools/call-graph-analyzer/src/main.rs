//! Call graph analyzer for ink! contracts (#1475).
//!
//! Parses contract sources with `syn` (nothing is compiled) and builds a directed graph whose nodes
//! are the contract's messages, constructors and helper functions, plus one node per distinct
//! external call target.
//!
//! Edges:
//!
//! * **internal**: `self.helper(..)`, `Self::helper(..)` and `helper(..)` where `helper` is a function
//!   defined in the analysed sources,
//! * **cross-contract** (drawn red): `build_call(..)` / `build_create(..)`, `.invoke()`,
//!   `.try_invoke()`, `.call_v1()`, `.delegate_call()`, native `.transfer(..)`, and any method call on a
//!   storage field whose type name contains `Ref` (a generated contract reference such as `TokenRef`),
//!   which becomes the node `FieldType::method`.
//!
//! The report lists:
//!
//! * **cycles** (strongly connected components and self-calls). A cycle that contains a cross-contract
//!   call is a potential reentrancy path,
//! * **cyclomatic complexity** per function: `1 +` each `if`, `match` arm beyond the first, `for`,
//!   `while`, `loop`, `&&`, `||` and `?`,
//! * the **deepest call chains** from messages (longest acyclic path, in edges).
//!
//! Calls through trait objects, closures and macros are not followed.
//!
//! ```text
//! call-graph-analyzer [--dot FILE] [--json FILE] [--top N] [--fail-on-cycles] PATH...
//! ```
//! `PATH` is a `.rs` file or a folder searched recursively. Without `--dot`/`--json` a text summary
//! is printed. Exit code 1 with `--fail-on-cycles` when a cycle exists, 2 on usage or parse errors.

use petgraph::algo::tarjan_scc;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use quote::ToTokens;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use syn::visit::Visit;
use syn::{Attribute, BinOp, Expr, ExprBinary, ExprCall, ExprForLoop, ExprIf, ExprLoop, ExprMatch, ExprMethodCall, ExprTry, ExprWhile, ImplItemFn, ItemFn, ItemStruct};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeKind {
    Message,
    Constructor,
    Internal,
    External,
}

impl NodeKind {
    fn as_str(self) -> &'static str {
        match self {
            NodeKind::Message => "message",
            NodeKind::Constructor => "constructor",
            NodeKind::Internal => "internal",
            NodeKind::External => "external",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeData {
    pub name: String,
    pub kind: NodeKind,
    pub complexity: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EdgeKind {
    Internal,
    CrossContract,
}

pub type CallGraph = DiGraph<NodeData, EdgeKind>;

fn ink_attr(attrs: &[Attribute], what: &str) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("ink")
            && a.meta
                .require_list()
                .map(|l| l.tokens.to_string().split(',').any(|t| t.trim() == what))
                .unwrap_or(false)
    })
}

fn is_self(expr: &Expr) -> bool {
    matches!(expr, Expr::Path(p) if p.path.is_ident("self"))
}

// ---------------------------------------------------------------------------------------------
// Cyclomatic complexity
// ---------------------------------------------------------------------------------------------

#[derive(Default)]
struct Complexity(u32);

impl<'ast> Visit<'ast> for Complexity {
    fn visit_expr_if(&mut self, i: &'ast ExprIf) {
        self.0 += 1;
        syn::visit::visit_expr_if(self, i);
    }
    fn visit_expr_match(&mut self, m: &'ast ExprMatch) {
        self.0 += m.arms.len().saturating_sub(1) as u32;
        syn::visit::visit_expr_match(self, m);
    }
    fn visit_expr_for_loop(&mut self, l: &'ast ExprForLoop) {
        self.0 += 1;
        syn::visit::visit_expr_for_loop(self, l);
    }
    fn visit_expr_while(&mut self, l: &'ast ExprWhile) {
        self.0 += 1;
        syn::visit::visit_expr_while(self, l);
    }
    fn visit_expr_loop(&mut self, l: &'ast ExprLoop) {
        self.0 += 1;
        syn::visit::visit_expr_loop(self, l);
    }
    fn visit_expr_binary(&mut self, b: &'ast ExprBinary) {
        if matches!(b.op, BinOp::And(_) | BinOp::Or(_)) {
            self.0 += 1;
        }
        syn::visit::visit_expr_binary(self, b);
    }
    fn visit_expr_try(&mut self, t: &'ast ExprTry) {
        self.0 += 1;
        syn::visit::visit_expr_try(self, t);
    }
}

// ---------------------------------------------------------------------------------------------
// Collecting functions and storage reference fields
// ---------------------------------------------------------------------------------------------

#[derive(Clone)]
struct FnInfo {
    name: String,
    kind: NodeKind,
    block: syn::Block,
}

#[derive(Default)]
struct Collector {
    fns: Vec<FnInfo>,
    /// Storage field name -> type name, for fields whose type is a contract reference.
    ref_fields: HashMap<String, String>,
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        let kind = if ink_attr(&f.attrs, "message") {
            NodeKind::Message
        } else if ink_attr(&f.attrs, "constructor") {
            NodeKind::Constructor
        } else {
            NodeKind::Internal
        };
        self.fns.push(FnInfo { name: f.sig.ident.to_string(), kind, block: f.block.clone() });
        syn::visit::visit_impl_item_fn(self, f);
    }

    fn visit_item_fn(&mut self, f: &'ast ItemFn) {
        self.fns.push(FnInfo { name: f.sig.ident.to_string(), kind: NodeKind::Internal, block: (*f.block).clone() });
        syn::visit::visit_item_fn(self, f);
    }

    fn visit_item_struct(&mut self, s: &'ast ItemStruct) {
        if ink_attr(&s.attrs, "storage") {
            for field in &s.fields {
                let ty: String = field.ty.to_token_stream().to_string().chars().filter(|c| !c.is_whitespace()).collect();
                if ty.contains("Ref") {
                    if let Some(name) = &field.ident {
                        self.ref_fields.insert(name.to_string(), ty);
                    }
                }
            }
        }
        syn::visit::visit_item_struct(self, s);
    }
}

// ---------------------------------------------------------------------------------------------
// Extracting calls from one function body
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Call {
    pub target: String,
    pub cross_contract: bool,
}

struct CallFinder<'a> {
    known: &'a HashSet<String>,
    ref_fields: &'a HashMap<String, String>,
    calls: Vec<Call>,
}

const CROSS_METHODS: &[&str] = &["invoke", "try_invoke", "call_v1", "delegate_call", "transfer"];
const CROSS_FUNCTIONS: &[&str] = &["build_call", "build_create"];

impl CallFinder<'_> {
    fn add(&mut self, target: String, cross_contract: bool) {
        let call = Call { target, cross_contract };
        if !self.calls.contains(&call) {
            self.calls.push(call);
        }
    }
}

impl<'ast> Visit<'ast> for CallFinder<'_> {
    fn visit_expr_method_call(&mut self, m: &'ast ExprMethodCall) {
        let method = m.method.to_string();
        if is_self(&m.receiver) {
            if self.known.contains(&method) {
                self.add(method.clone(), false);
            }
        } else if let Expr::Field(f) = &*m.receiver {
            let field = f.member.to_token_stream().to_string();
            if is_self(&f.base) {
                if let Some(ty) = self.ref_fields.get(&field) {
                    self.add(format!("{ty}::{method}"), true);
                }
            }
        }
        if CROSS_METHODS.contains(&method.as_str()) {
            self.add(format!("external::{method}"), true);
        }
        syn::visit::visit_expr_method_call(self, m);
    }

    fn visit_expr_call(&mut self, c: &'ast ExprCall) {
        if let Expr::Path(p) = &*c.func {
            let segments: Vec<String> = p.path.segments.iter().map(|s| s.ident.to_string()).collect();
            if let Some(last) = segments.last() {
                let is_cross = CROSS_FUNCTIONS.contains(&last.as_str())
                    || (segments.len() > 2 && segments[..segments.len() - 1].join("::").ends_with("env::call"));
                let internal = (segments.len() == 1 || (segments.len() == 2 && segments[0] == "Self")) && self.known.contains(last);
                if is_cross {
                    self.add(format!("external::{last}"), true);
                } else if internal {
                    self.add(last.clone(), false);
                }
            }
        }
        syn::visit::visit_expr_call(self, c);
    }
}

// ---------------------------------------------------------------------------------------------
// Graph construction
// ---------------------------------------------------------------------------------------------

/// Builds the call graph of the given sources (all files are analysed together).
pub fn build_graph(sources: &[(&str, &str)]) -> Result<CallGraph, String> {
    let mut collector = Collector::default();
    for (label, src) in sources {
        let file = syn::parse_file(src).map_err(|e| format!("{label}: parse error: {e}"))?;
        collector.visit_file(&file);
    }

    // The first definition of a name wins; a message and a helper cannot share a name in one impl.
    let mut fns: Vec<FnInfo> = Vec::new();
    for f in &collector.fns {
        if !fns.iter().any(|g| g.name == f.name) {
            fns.push(f.clone());
        }
    }
    let known: HashSet<String> = fns.iter().map(|f| f.name.clone()).collect();

    let mut graph = CallGraph::new();
    let mut index: HashMap<String, NodeIndex> = HashMap::new();
    for f in &fns {
        let mut c = Complexity::default();
        c.visit_block(&f.block);
        let idx = graph.add_node(NodeData { name: f.name.clone(), kind: f.kind, complexity: 1 + c.0 });
        index.insert(f.name.clone(), idx);
    }
    for f in &fns {
        let mut finder = CallFinder { known: &known, ref_fields: &collector.ref_fields, calls: Vec::new() };
        finder.visit_block(&f.block);
        let from = index[&f.name];
        for call in finder.calls {
            let to = match index.get(&call.target) {
                Some(i) => *i,
                None => {
                    let i = graph.add_node(NodeData { name: call.target.clone(), kind: NodeKind::External, complexity: 0 });
                    index.insert(call.target.clone(), i);
                    i
                }
            };
            if graph.find_edge(from, to).is_none() {
                graph.add_edge(from, to, if call.cross_contract { EdgeKind::CrossContract } else { EdgeKind::Internal });
            }
        }
    }
    Ok(graph)
}

// ---------------------------------------------------------------------------------------------
// Analysis
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Cycle {
    pub functions: Vec<String>,
    /// `true` when a function in the cycle also makes a cross-contract call.
    pub reaches_external_call: bool,
}

/// Cycles: strongly connected components of more than one node, and self-calls.
pub fn find_cycles(graph: &CallGraph) -> Vec<Cycle> {
    let mut cycles = Vec::new();
    for scc in tarjan_scc(graph) {
        let is_cycle = scc.len() > 1 || scc.iter().any(|n| graph.find_edge(*n, *n).is_some());
        if !is_cycle {
            continue;
        }
        let mut functions: Vec<String> = scc.iter().map(|n| graph[*n].name.clone()).collect();
        functions.sort();
        let reaches_external_call = scc.iter().any(|n| {
            graph.edges_directed(*n, Direction::Outgoing).any(|e| *e.weight() == EdgeKind::CrossContract)
        });
        cycles.push(Cycle { functions, reaches_external_call });
    }
    cycles.sort_by(|a, b| a.functions.cmp(&b.functions));
    cycles
}

fn longest_chain(graph: &CallGraph, node: NodeIndex, on_path: &mut Vec<NodeIndex>) -> Vec<NodeIndex> {
    on_path.push(node);
    let mut best: Vec<NodeIndex> = Vec::new();
    for next in graph.neighbors_directed(node, Direction::Outgoing) {
        if on_path.contains(&next) {
            continue; // a back edge: the cycle report covers it
        }
        let chain = longest_chain(graph, next, on_path);
        if chain.len() > best.len() || (chain.len() == best.len() && names(graph, &chain) < names(graph, &best)) {
            best = chain;
        }
    }
    on_path.pop();
    let mut out = vec![node];
    out.extend(best);
    out
}

fn names(graph: &CallGraph, chain: &[NodeIndex]) -> Vec<String> {
    chain.iter().map(|n| graph[*n].name.clone()).collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Chain {
    pub message: String,
    /// Number of calls along the longest acyclic path.
    pub depth: usize,
    pub path: Vec<String>,
}

/// The longest call chain starting from each message, deepest first.
pub fn deepest_chains(graph: &CallGraph) -> Vec<Chain> {
    let mut chains: Vec<Chain> = graph
        .node_indices()
        .filter(|n| graph[*n].kind == NodeKind::Message)
        .map(|n| {
            let path = longest_chain(graph, n, &mut Vec::new());
            Chain { message: graph[n].name.clone(), depth: path.len() - 1, path: names(graph, &path) }
        })
        .collect();
    chains.sort_by(|a, b| b.depth.cmp(&a.depth).then_with(|| a.message.cmp(&b.message)));
    chains
}

fn quote_dot(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Graphviz DOT output. Cross-contract edges are red and thick.
pub fn to_dot(graph: &CallGraph) -> String {
    let mut out = String::from("digraph contract {\n  rankdir=LR;\n  node [shape=box, fontname=\"Helvetica\"];\n");
    for n in graph.node_indices() {
        let d = &graph[n];
        let style = match d.kind {
            NodeKind::Message => "style=filled, fillcolor=lightblue",
            NodeKind::Constructor => "style=filled, fillcolor=lightgreen",
            NodeKind::Internal => "style=solid",
            NodeKind::External => "shape=ellipse, style=filled, fillcolor=\"#ffcccc\"",
        };
        let label = if d.kind == NodeKind::External {
            d.name.clone()
        } else {
            format!("{}\\nCC={}", d.name, d.complexity)
        };
        let label = format!("\"{}\"", label.replace('"', "\\\""));
        out.push_str(&format!("  {} [label={label}, {style}];\n", quote_dot(&d.name)));
    }
    for e in graph.edge_references() {
        let (from, to) = (quote_dot(&graph[e.source()].name), quote_dot(&graph[e.target()].name));
        match e.weight() {
            EdgeKind::Internal => out.push_str(&format!("  {from} -> {to};\n")),
            EdgeKind::CrossContract => {
                out.push_str(&format!("  {from} -> {to} [color=red, penwidth=2, label=\"cross-contract\"];\n"))
            }
        }
    }
    out.push_str("}\n");
    out
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

fn json_strings(items: &[String]) -> String {
    format!("[{}]", items.iter().map(|s| format!("\"{}\"", json_escape(s))).collect::<Vec<_>>().join(","))
}

/// JSON with nodes, edges, cycles and the deepest chains.
pub fn to_json(graph: &CallGraph) -> String {
    let nodes = graph
        .node_indices()
        .map(|n| {
            let d = &graph[n];
            format!("{{\"name\":\"{}\",\"kind\":\"{}\",\"complexity\":{}}}", json_escape(&d.name), d.kind.as_str(), d.complexity)
        })
        .collect::<Vec<_>>()
        .join(",");
    let edges = graph
        .edge_references()
        .map(|e| {
            format!(
                "{{\"from\":\"{}\",\"to\":\"{}\",\"cross_contract\":{}}}",
                json_escape(&graph[e.source()].name),
                json_escape(&graph[e.target()].name),
                *e.weight() == EdgeKind::CrossContract
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let cycles = find_cycles(graph)
        .iter()
        .map(|c| format!("{{\"functions\":{},\"reaches_external_call\":{}}}", json_strings(&c.functions), c.reaches_external_call))
        .collect::<Vec<_>>()
        .join(",");
    let chains = deepest_chains(graph)
        .iter()
        .map(|c| format!("{{\"message\":\"{}\",\"depth\":{},\"path\":{}}}", json_escape(&c.message), c.depth, json_strings(&c.path)))
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"nodes\":[{nodes}],\"edges\":[{edges}],\"cycles\":[{cycles}],\"deepest_chains\":[{chains}]}}\n")
}

pub fn to_text(graph: &CallGraph, top: usize) -> String {
    let count = |k: NodeKind| graph.node_weights().filter(|n| n.kind == k).count();
    let cross = graph.edge_weights().filter(|w| **w == EdgeKind::CrossContract).count();
    let mut out = format!(
        "Call graph: {} messages, {} constructors, {} internal functions, {} external targets, {} cross-contract edges\n",
        count(NodeKind::Message),
        count(NodeKind::Constructor),
        count(NodeKind::Internal),
        count(NodeKind::External),
        cross
    );
    let cycles = find_cycles(graph);
    out.push_str(&format!("\nCycles: {}\n", cycles.len()));
    for c in &cycles {
        out.push_str(&format!(
            "  - {}{}\n",
            c.functions.join(" <-> "),
            if c.reaches_external_call { "  [POTENTIAL REENTRANCY: makes a cross-contract call]" } else { "" }
        ));
    }
    out.push_str("\nDeepest call chains from messages:\n");
    for c in deepest_chains(graph).iter().take(top) {
        out.push_str(&format!("  - {} (depth {}): {}\n", c.message, c.depth, c.path.join(" -> ")));
    }
    let mut by_complexity: BTreeMap<std::cmp::Reverse<u32>, Vec<&str>> = BTreeMap::new();
    for n in graph.node_weights().filter(|n| n.kind != NodeKind::External) {
        by_complexity.entry(std::cmp::Reverse(n.complexity)).or_default().push(&n.name);
    }
    out.push_str("\nCyclomatic complexity (highest first):\n");
    for (std::cmp::Reverse(cc), fns) in by_complexity.iter().take(top) {
        out.push_str(&format!("  - CC {cc}: {}\n", fns.join(", ")));
    }
    out
}

fn collect_rs(path: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    if path.is_dir() {
        let mut entries: Vec<_> = fs::read_dir(path).map_err(|e| format!("{}: {e}", path.display()))?.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let p = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            if p.is_dir() && name != "target" && name != ".git" {
                collect_rs(&p, out)?;
            } else if p.extension().map(|x| x == "rs").unwrap_or(false) {
                out.push(p);
            }
        }
        Ok(())
    } else if path.exists() {
        out.push(path.to_path_buf());
        Ok(())
    } else {
        Err(format!("{}: no such file or folder", path.display()))
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: call-graph-analyzer [--dot FILE] [--json FILE] [--top N] [--fail-on-cycles] PATH...");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut dot, mut json, mut top, mut fail_on_cycles) = (None::<String>, None::<String>, 5usize, false);
    let mut paths = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--dot" => dot = args.get(i + 1).cloned(),
            "--json" => json = args.get(i + 1).cloned(),
            "--top" => match args.get(i + 1).and_then(|s| s.parse().ok()) {
                Some(n) => top = n,
                None => return usage(),
            },
            "--fail-on-cycles" => {
                fail_on_cycles = true;
                i += 1;
                continue;
            }
            s if s.starts_with("--") => return usage(),
            _ => {
                paths.push(PathBuf::from(&args[i]));
                i += 1;
                continue;
            }
        }
        i += 2;
    }
    if paths.is_empty() {
        return usage();
    }

    let mut files = Vec::new();
    for p in &paths {
        if let Err(e) = collect_rs(p, &mut files) {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    }
    let mut texts = Vec::new();
    for f in &files {
        match fs::read_to_string(f) {
            Ok(t) => texts.push((f.display().to_string(), t)),
            Err(e) => {
                eprintln!("error: {}: {e}", f.display());
                return ExitCode::from(2);
            }
        }
    }
    let sources: Vec<(&str, &str)> = texts.iter().map(|(l, t)| (l.as_str(), t.as_str())).collect();
    let graph = match build_graph(&sources) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    let mut wrote = false;
    for (target, content) in [(&dot, to_dot(&graph)), (&json, to_json(&graph))] {
        if let Some(path) = target {
            if let Err(e) = fs::write(path, content) {
                eprintln!("error: {path}: {e}");
                return ExitCode::from(2);
            }
            println!("wrote {path}");
            wrote = true;
        }
    }
    if !wrote {
        print!("{}", to_text(&graph, top));
    }
    if fail_on_cycles && !find_cycles(&graph).is_empty() {
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
#[ink::contract]
mod vault {
    #[ink(storage)]
    pub struct Vault {
        owner: AccountId,
        token: TokenRef,
        total: u128,
    }

    impl Vault {
        #[ink(constructor)]
        pub fn new(token: TokenRef) -> Self { Self { owner: Self::env().caller(), token, total: 0 } }

        #[ink(message)]
        pub fn deposit(&mut self, amount: u128) -> Result<(), ()> {
            self.check(amount)?;
            self.total += amount;
            self.token.transfer_from(self.owner, amount);
            Ok(())
        }

        #[ink(message)]
        pub fn withdraw(&mut self, amount: u128) {
            if amount > self.total && amount > 0 || amount == 7 {
                return;
            }
            self.pay(amount);
        }

        #[ink(message)]
        pub fn total(&self) -> u128 { self.total }

        fn check(&self, amount: u128) -> Result<(), ()> {
            if amount == 0 { return Err(()); }
            Ok(())
        }

        fn pay(&mut self, amount: u128) {
            self.payout(amount);
        }

        fn payout(&mut self, amount: u128) {
            build_call::<E>().invoke();
            if amount > 1 {
                self.pay(amount - 1);
            }
        }
    }
}
"#;

    fn graph() -> CallGraph {
        build_graph(&[("vault.rs", SAMPLE)]).unwrap()
    }

    fn node<'a>(g: &'a CallGraph, name: &str) -> &'a NodeData {
        g.node_weights().find(|n| n.name == name).unwrap_or_else(|| panic!("no node {name}"))
    }

    fn has_edge(g: &CallGraph, from: &str, to: &str, kind: EdgeKind) -> bool {
        g.edge_references().any(|e| g[e.source()].name == from && g[e.target()].name == to && *e.weight() == kind)
    }

    #[test]
    fn classifies_nodes() {
        let g = graph();
        assert_eq!(node(&g, "new").kind, NodeKind::Constructor);
        assert_eq!(node(&g, "deposit").kind, NodeKind::Message);
        assert_eq!(node(&g, "check").kind, NodeKind::Internal);
        assert_eq!(node(&g, "external::invoke").kind, NodeKind::External);
    }

    #[test]
    fn builds_internal_edges() {
        let g = graph();
        assert!(has_edge(&g, "deposit", "check", EdgeKind::Internal));
        assert!(has_edge(&g, "withdraw", "pay", EdgeKind::Internal));
        assert!(has_edge(&g, "pay", "payout", EdgeKind::Internal));
        assert!(!has_edge(&g, "total", "check", EdgeKind::Internal));
    }

    #[test]
    fn finds_cross_contract_calls_through_build_call_and_contract_refs() {
        let g = graph();
        assert!(has_edge(&g, "payout", "external::invoke", EdgeKind::CrossContract));
        assert!(has_edge(&g, "payout", "external::build_call", EdgeKind::CrossContract));
        assert!(has_edge(&g, "deposit", "TokenRef::transfer_from", EdgeKind::CrossContract));
        assert!(!has_edge(&g, "deposit", "check", EdgeKind::CrossContract));
    }

    #[test]
    fn an_edge_is_added_once_per_pair() {
        let src = "impl C { #[ink(message)] pub fn a(&self) { self.b(); self.b(); } fn b(&self) {} }";
        let g = build_graph(&[("c.rs", src)]).unwrap();
        assert_eq!(g.edge_count(), 1);
    }

    #[test]
    fn detects_a_cycle_and_marks_it_when_it_reaches_an_external_call() {
        let cycles = find_cycles(&graph());
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0].functions, vec!["pay".to_string(), "payout".to_string()]);
        assert!(cycles[0].reaches_external_call);
    }

    #[test]
    fn detects_a_self_call() {
        let src = "impl C { #[ink(message)] pub fn a(&self) { self.a(); } }";
        let cycles = find_cycles(&build_graph(&[("c.rs", src)]).unwrap());
        assert_eq!(cycles, vec![Cycle { functions: vec!["a".into()], reaches_external_call: false }]);
    }

    #[test]
    fn an_acyclic_graph_has_no_cycles() {
        let src = "impl C { #[ink(message)] pub fn a(&self) { self.b(); } fn b(&self) { self.c(); } fn c(&self) {} }";
        assert!(find_cycles(&build_graph(&[("c.rs", src)]).unwrap()).is_empty());
    }

    #[test]
    fn computes_cyclomatic_complexity() {
        let g = graph();
        assert_eq!(node(&g, "total").complexity, 1);
        assert_eq!(node(&g, "check").complexity, 2); // one `if`
        assert_eq!(node(&g, "deposit").complexity, 2); // one `?`
        // `if` + `&&` + `||`
        assert_eq!(node(&g, "withdraw").complexity, 4);
    }

    #[test]
    fn match_arms_and_loops_add_complexity() {
        let src = "impl C { #[ink(message)] pub fn a(&self, x: u8) { match x { 0 => {}, 1 => {}, _ => {} } for _ in 0..x {} while x > 1 {} loop { break; } } }";
        let g = build_graph(&[("c.rs", src)]).unwrap();
        assert_eq!(node(&g, "a").complexity, 1 + 2 + 1 + 1 + 1);
    }

    #[test]
    fn ranks_the_deepest_chains_without_looping_on_cycles() {
        let chains = deepest_chains(&graph());
        assert_eq!(chains[0].message, "withdraw");
        assert_eq!(chains[0].path, vec!["withdraw", "pay", "payout", "external::build_call"]);
        assert_eq!(chains[0].depth, 3);
        assert!(chains.iter().any(|c| c.message == "total" && c.depth == 0));
        assert!(chains.windows(2).all(|w| w[0].depth >= w[1].depth));
    }

    #[test]
    fn dot_output_highlights_cross_contract_edges() {
        let dot = to_dot(&graph());
        assert!(dot.starts_with("digraph contract {"));
        assert!(dot.contains("\"payout\" -> \"external::invoke\" [color=red, penwidth=2"));
        assert!(dot.contains("\"deposit\" -> \"check\";"));
        assert!(dot.contains("CC=4"));
        assert!(dot.trim_end().ends_with('}'));
    }

    #[test]
    fn json_output_has_nodes_edges_cycles_and_chains() {
        let json = to_json(&graph());
        assert!(json.contains("\"nodes\":["));
        assert!(json.contains("\"cross_contract\":true"));
        assert!(json.contains("\"reaches_external_call\":true"));
        assert!(json.contains("\"deepest_chains\":[{\"message\":\"withdraw\""));
    }

    #[test]
    fn text_summary_flags_potential_reentrancy() {
        let text = to_text(&graph(), 3);
        assert!(text.contains("POTENTIAL REENTRANCY"));
        assert!(text.contains("withdraw (depth 3)"));
    }

    #[test]
    fn merges_functions_across_files() {
        let a = "impl C { #[ink(message)] pub fn a(&self) { self.b(); } }";
        let b = "impl C { fn b(&self) {} }";
        let g = build_graph(&[("a.rs", a), ("b.rs", b)]).unwrap();
        assert!(has_edge(&g, "a", "b", EdgeKind::Internal));
    }

    #[test]
    fn reports_parse_errors_with_the_file_name() {
        let e = build_graph(&[("bad.rs", "fn {")]).unwrap_err();
        assert!(e.starts_with("bad.rs"));
    }
}
