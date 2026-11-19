//! Fuzzing harness generator for ink! contracts (#1474).
//!
//! Reads a contract's source with `syn` and writes a small crate of `proptest` harnesses, one test
//! file per `#[ink(message)]`. Each harness:
//!
//! * builds the contract with the first `#[ink(constructor)]` using generated arguments,
//! * sets a fuzzed caller with `ink::env::test::set_caller`,
//! * calls the message up to `--depth` times with generated arguments, catching panics,
//! * after every call, checks invariants: every argument-less `&self` message must still run
//!   without panicking, and, when the contract has `total_supply()` and `balance_of(AccountId)`,
//!   the caller's balance may never exceed the total supply.
//!
//! A panic or a broken invariant fails the property, and `proptest` shrinks the failing input and
//! prints it, which is the crash / panic / invariant report. Generated inputs cover the ink!
//! primitives (`AccountId`, `Balance`, `Hash`, integers, `bool`, `String`, `Vec<T>`, `Option<T>`,
//! byte arrays). A parameter of any other type falls back to `Default::default()` and is listed in a
//! comment at the top of the harness so it can be improved by hand.
//!
//! ```text
//! fuzz-harness-gen --contract lib.rs --out harness --crate-name my_contract --crate-path ../my_contract
//!                  [--cases 256] [--depth 4] [--max-len 32] [--message NAME]...
//! cd harness && cargo test
//! ```
//! Exit code 2 on usage or parse errors.

use quote::ToTokens;
use std::fs;
use std::path::Path;
use std::process::ExitCode;
use syn::visit::Visit;
use syn::{Attribute, FnArg, ImplItemFn, ItemMod, ItemStruct, Pat, ReturnType};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Param {
    pub name: String,
    pub ty: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Func {
    pub name: String,
    pub params: Vec<Param>,
    /// `true` for `&mut self`.
    pub mutates: bool,
    pub returns: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Contract {
    pub module: String,
    pub name: String,
    pub constructor: Option<Func>,
    pub messages: Vec<Func>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub crate_name: String,
    pub cases: u32,
    pub depth: u32,
    pub max_len: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self { crate_name: "contract".into(), cases: 256, depth: 4, max_len: 32 }
    }
}

fn ink_attr(attrs: &[Attribute], what: &str) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("ink")
            && a.meta
                .require_list()
                .map(|l| l.tokens.to_string().split(',').any(|t| t.trim() == what))
                .unwrap_or(false)
    })
}

fn compact(ty: &syn::Type) -> String {
    ty.to_token_stream().to_string().chars().filter(|c| !c.is_whitespace()).collect()
}

fn func_of(f: &ImplItemFn) -> Func {
    let mut mutates = false;
    let mut params = Vec::new();
    for input in &f.sig.inputs {
        match input {
            FnArg::Receiver(r) => mutates = r.reference.is_some() && r.mutability.is_some(),
            FnArg::Typed(t) => {
                let name = match &*t.pat {
                    Pat::Ident(i) => i.ident.to_string(),
                    other => other.to_token_stream().to_string(),
                };
                params.push(Param { name, ty: compact(&t.ty) });
            }
        }
    }
    let returns = match &f.sig.output {
        ReturnType::Default => None,
        ReturnType::Type(_, t) => Some(compact(t)),
    };
    Func { name: f.sig.ident.to_string(), params, mutates, returns }
}

#[derive(Default)]
struct Collector {
    module_stack: Vec<String>,
    storage: Option<(String, String)>, // (module, struct)
    constructor: Option<Func>,
    messages: Vec<Func>,
}

impl<'ast> Visit<'ast> for Collector {
    fn visit_item_mod(&mut self, m: &'ast ItemMod) {
        self.module_stack.push(m.ident.to_string());
        syn::visit::visit_item_mod(self, m);
        self.module_stack.pop();
    }

    fn visit_item_struct(&mut self, s: &'ast ItemStruct) {
        if self.storage.is_none() && ink_attr(&s.attrs, "storage") {
            self.storage = Some((self.module_stack.join("::"), s.ident.to_string()));
        }
        syn::visit::visit_item_struct(self, s);
    }

    fn visit_impl_item_fn(&mut self, f: &'ast ImplItemFn) {
        if ink_attr(&f.attrs, "message") {
            self.messages.push(func_of(f));
        } else if ink_attr(&f.attrs, "constructor") && self.constructor.is_none() {
            self.constructor = Some(func_of(f));
        }
        syn::visit::visit_impl_item_fn(self, f);
    }
}

/// Finds the storage struct, the first constructor and every message of a contract source.
pub fn parse_contract(source: &str) -> Result<Contract, String> {
    let file = syn::parse_file(source).map_err(|e| format!("parse error: {e}"))?;
    let mut c = Collector::default();
    c.visit_file(&file);
    let (module, name) = c.storage.ok_or("no #[ink(storage)] struct found")?;
    if c.messages.is_empty() {
        return Err("no #[ink(message)] functions found".into());
    }
    Ok(Contract { module, name, constructor: c.constructor, messages: c.messages })
}

fn split_generic<'a>(ty: &'a str, outer: &str) -> Option<&'a str> {
    ty.strip_prefix(outer)?.strip_prefix('<')?.strip_suffix('>')
}

/// A `proptest` strategy expression producing values of Rust type `ty`, or `None` if unsupported.
pub fn strategy_for(ty: &str, max_len: u32) -> Option<String> {
    const INTS: &[&str] = &["u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize", "bool"];
    if INTS.contains(&ty) {
        return Some(format!("any::<{ty}>()"));
    }
    match ty {
        "Balance" => return Some("any::<u128>()".into()),
        "BlockNumber" => return Some("any::<u32>()".into()),
        "Timestamp" => return Some("any::<u64>()".into()),
        "AccountId" | "ink::primitives::AccountId" => {
            return Some("any::<[u8; 32]>().prop_map(AccountId::from)".into())
        }
        "Hash" | "ink::primitives::Hash" => return Some("any::<[u8; 32]>().prop_map(Hash::from)".into()),
        "String" | "ink::prelude::string::String" => return Some(format!("\"[a-zA-Z0-9 ]{{0,{max_len}}}\"")),
        _ => {}
    }
    if let Some(inner) = ty.strip_prefix("[u8;").and_then(|r| r.strip_suffix(']')) {
        if inner.parse::<usize>().map(|n| n <= 32).unwrap_or(false) {
            return Some(format!("any::<[u8; {inner}]>()"));
        }
    }
    for outer in ["Vec", "ink::prelude::vec::Vec"] {
        if let Some(inner) = split_generic(ty, outer) {
            return strategy_for(inner, max_len).map(|s| format!("prop::collection::vec({s}, 0..={max_len})"));
        }
    }
    if let Some(inner) = split_generic(ty, "Option") {
        return strategy_for(inner, max_len).map(|s| format!("proptest::option::of({s})"));
    }
    None
}

fn strategy_or_default(ty: &str, max_len: u32, unsupported: &mut Vec<String>) -> String {
    strategy_for(ty, max_len).unwrap_or_else(|| {
        unsupported.push(ty.to_string());
        format!("Just(<{ty} as Default>::default())")
    })
}

fn invariants_fn(contract: &Contract) -> String {
    let mut body = String::new();
    for m in contract.messages.iter().filter(|m| m.params.is_empty() && !m.mutates) {
        body.push_str(&format!("    let _ = contract.{}();\n", m.name));
    }
    let has_supply = contract.messages.iter().any(|m| m.name == "total_supply" && m.params.is_empty());
    let has_balance = contract
        .messages
        .iter()
        .any(|m| m.name == "balance_of" && m.params.len() == 1 && m.params[0].ty.ends_with("AccountId"));
    if has_supply && has_balance {
        body.push_str("    assert!(\n        contract.balance_of(caller) <= contract.total_supply(),\n        \"invariant violated: a balance exceeds the total supply\"\n    );\n");
    }
    if body.is_empty() {
        body.push_str("    let _ = (contract, caller);\n");
    } else if !(has_supply && has_balance) {
        body.push_str("    let _ = caller;\n");
    }
    format!("fn check_invariants(contract: &{}, caller: AccountId) {{\n{body}}}\n", contract.name)
}

/// The source of one harness file for `message`.
pub fn generate_harness(contract: &Contract, message: &Func, cfg: &Config) -> String {
    let mut unsupported = Vec::new();
    let ctor_params: Vec<&Param> = contract.constructor.iter().flat_map(|c| c.params.iter()).collect();
    let ctor_strategies: Vec<String> =
        ctor_params.iter().map(|p| strategy_or_default(&p.ty, cfg.max_len, &mut unsupported)).collect();
    let arg_strategies: Vec<String> =
        message.params.iter().map(|p| strategy_or_default(&p.ty, cfg.max_len, &mut unsupported)).collect();

    let ctor_inputs: String = ctor_strategies.iter().enumerate().map(|(i, s)| format!("        c{i} in {s},\n")).collect();
    let ctor_call = match &contract.constructor {
        Some(c) => format!(
            "{}::{}({})",
            contract.name,
            c.name,
            (0..c.params.len()).map(|i| format!("c{i}")).collect::<Vec<_>>().join(", ")
        ),
        None => format!("{}::default()", contract.name),
    };
    let tuple_strategy = match arg_strategies.len() {
        0 => "Just(())".to_string(),
        1 => format!("({},)", arg_strategies[0]),
        _ => format!("({})", arg_strategies.join(", ")),
    };
    let pattern = match message.params.len() {
        0 => "()".to_string(),
        1 => "(a0,)".to_string(),
        n => format!("({})", (0..n).map(|i| format!("a{i}")).collect::<Vec<_>>().join(", ")),
    };
    let call_args = (0..message.params.len()).map(|i| format!("a{i}.clone()")).collect::<Vec<_>>().join(", ");

    let signature = format!(
        "{}({}){}",
        message.name,
        message.params.iter().map(|p| format!("{}: {}", p.name, p.ty)).collect::<Vec<_>>().join(", "),
        message.returns.as_ref().map(|r| format!(" -> {r}")).unwrap_or_default()
    );
    let contract_path = if contract.module.is_empty() {
        format!("{}::{}", cfg.crate_name, contract.name)
    } else {
        format!("{}::{}::{}", cfg.crate_name, contract.module, contract.name)
    };

    let mut out = String::new();
    out.push_str(&format!(
        "//! Generated by fuzz-harness-gen for `{}::{signature}`. Regenerate instead of editing.\n",
        contract.name
    ));
    if !unsupported.is_empty() {
        out.push_str(&format!(
            "//! No generator for: {}. They use `Default::default()`; improve the strategies by hand if needed.\n",
            unsupported.join(", ")
        ));
    }
    out.push_str(&format!(
        "\nuse proptest::prelude::*;\nuse {contract_path};\n\ntype AccountId = ink::primitives::AccountId;\n#[allow(dead_code)]\ntype Hash = ink::primitives::Hash;\n\n"
    ));
    out.push_str(&invariants_fn(contract));
    out.push_str(&format!(
        "\nproptest! {{\n    #![proptest_config(ProptestConfig::with_cases({cases}))]\n\n    #[test]\n    fn fuzz_{name}(\n        caller in any::<[u8; 32]>(),\n{ctor_inputs}        calls in prop::collection::vec({tuple}, 1..={depth}),\n    ) {{\n        let caller = AccountId::from(caller);\n        ink::env::test::set_caller::<ink::env::DefaultEnvironment>(caller);\n        #[allow(unused_mut)]\n        let mut contract = {ctor};\n        for {pattern} in calls {{\n            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {{\n                let _ = contract.{name}({call_args});\n            }}));\n            prop_assert!(outcome.is_ok(), \"message `{name}` panicked\");\n            let invariants = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| check_invariants(&contract, caller)));\n            prop_assert!(invariants.is_ok(), \"an invariant was violated after `{name}`\");\n        }}\n    }}\n}}\n",
        cases = cfg.cases,
        name = message.name,
        ctor_inputs = ctor_inputs,
        tuple = tuple_strategy,
        depth = cfg.depth,
        ctor = ctor_call,
        pattern = pattern,
        call_args = call_args,
    ));
    out
}

/// `Cargo.toml` for the generated harness crate.
pub fn generate_manifest(contract_name: &str, cfg: &Config, crate_path: &str) -> String {
    format!(
        "[package]\nname = \"{pkg}-fuzz\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[dependencies]\n{krate} = {{ path = \"{path}\" }}\nink = {{ version = \"5.0.0\", default-features = false, features = [\"std\"] }}\n\n[dev-dependencies]\nproptest = \"1\"\n\n# Standalone: not part of the main workspace.\n[workspace]\n",
        pkg = contract_name.to_lowercase(),
        krate = cfg.crate_name,
        path = crate_path,
    )
}

fn usage() -> ExitCode {
    eprintln!(
        "usage: fuzz-harness-gen --contract FILE --out DIR --crate-name NAME --crate-path PATH [--cases N] [--depth N] [--max-len N] [--message NAME]..."
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (mut contract_file, mut out_dir, mut crate_path) = (None, None, None);
    let mut cfg = Config::default();
    let mut only: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let value = args.get(i + 1).cloned();
        let number = |v: &Option<String>| v.as_deref().and_then(|s| s.parse::<u32>().ok()).filter(|n| *n > 0);
        match args[i].as_str() {
            "--contract" => contract_file = value,
            "--out" => out_dir = value,
            "--crate-name" => match value {
                Some(v) => cfg.crate_name = v,
                None => return usage(),
            },
            "--crate-path" => crate_path = value,
            "--cases" => match number(&value) {
                Some(n) => cfg.cases = n,
                None => return usage(),
            },
            "--depth" => match number(&value) {
                Some(n) => cfg.depth = n,
                None => return usage(),
            },
            "--max-len" => match number(&value) {
                Some(n) => cfg.max_len = n,
                None => return usage(),
            },
            "--message" => only.extend(value),
            _ => return usage(),
        }
        i += 2;
    }
    let (Some(contract_file), Some(out_dir), Some(crate_path)) = (contract_file, out_dir, crate_path) else {
        return usage();
    };

    let contract = match fs::read_to_string(&contract_file)
        .map_err(|e| format!("{contract_file}: {e}"))
        .and_then(|s| parse_contract(&s).map_err(|e| format!("{contract_file}: {e}")))
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    for name in &only {
        if !contract.messages.iter().any(|m| &m.name == name) {
            eprintln!("error: no message named `{name}`");
            return ExitCode::from(2);
        }
    }

    let out = Path::new(&out_dir);
    let tests = out.join("tests");
    if let Err(e) = fs::create_dir_all(&tests) {
        eprintln!("error: {}: {e}", tests.display());
        return ExitCode::from(2);
    }
    let mut written = vec![out.join("Cargo.toml")];
    let mut result = fs::write(out.join("Cargo.toml"), generate_manifest(&contract.name, &cfg, &crate_path));
    for m in contract.messages.iter().filter(|m| only.is_empty() || only.contains(&m.name)) {
        let path = tests.join(format!("fuzz_{}.rs", m.name));
        result = result.and_then(|_| fs::write(&path, generate_harness(&contract, m, &cfg)));
        written.push(path);
    }
    if let Err(e) = result {
        eprintln!("error: {e}");
        return ExitCode::from(2);
    }
    for p in &written {
        println!("wrote {}", p.display());
    }
    println!("run them with: cd {out_dir} && cargo test");
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
#[ink::contract]
mod token {
    use ink::storage::Mapping;

    #[ink(storage)]
    #[derive(Default)]
    pub struct Token {
        total_supply: Balance,
        balances: Mapping<AccountId, Balance>,
    }

    impl Token {
        #[ink(constructor)]
        pub fn new(supply: Balance) -> Self {
            Self { total_supply: supply, balances: Mapping::default() }
        }

        #[ink(message)]
        pub fn total_supply(&self) -> Balance { self.total_supply }

        #[ink(message)]
        pub fn balance_of(&self, owner: AccountId) -> Balance { self.balances.get(owner).unwrap_or(0) }

        #[ink(message)]
        pub fn transfer(&mut self, to: AccountId, value: Balance, memo: Option<Vec<u8>>) -> bool { true }

        #[ink(message, payable)]
        pub fn ping(&mut self) {}

        fn internal(&self) {}
    }
}
"#;

    fn sample() -> Contract {
        parse_contract(SAMPLE).unwrap()
    }

    fn cfg() -> Config {
        Config { crate_name: "token_contract".into(), cases: 64, depth: 3, max_len: 16 }
    }

    #[test]
    fn parses_the_contract_shape() {
        let c = sample();
        assert_eq!((c.module.as_str(), c.name.as_str()), ("token", "Token"));
        assert_eq!(c.messages.iter().map(|m| m.name.as_str()).collect::<Vec<_>>(), vec!["total_supply", "balance_of", "transfer", "ping"]);
        assert_eq!(c.constructor.as_ref().unwrap().name, "new");
        assert_eq!(c.constructor.as_ref().unwrap().params, vec![Param { name: "supply".into(), ty: "Balance".into() }]);
    }

    #[test]
    fn records_parameters_receivers_and_returns() {
        let c = sample();
        let transfer = c.messages.iter().find(|m| m.name == "transfer").unwrap();
        assert!(transfer.mutates);
        assert_eq!(transfer.params.len(), 3);
        assert_eq!(transfer.params[2].ty, "Option<Vec<u8>>");
        assert_eq!(transfer.returns.as_deref(), Some("bool"));
        let supply = c.messages.iter().find(|m| m.name == "total_supply").unwrap();
        assert!(!supply.mutates);
    }

    #[test]
    fn requires_storage_and_messages() {
        assert!(parse_contract("mod m { pub struct S; }").is_err());
        assert!(parse_contract("mod m { #[ink(storage)] pub struct S {} }").is_err());
        assert!(parse_contract("fn {").is_err());
    }

    #[test]
    fn maps_ink_types_to_strategies() {
        assert_eq!(strategy_for("u32", 8).unwrap(), "any::<u32>()");
        assert_eq!(strategy_for("bool", 8).unwrap(), "any::<bool>()");
        assert_eq!(strategy_for("Balance", 8).unwrap(), "any::<u128>()");
        assert_eq!(strategy_for("AccountId", 8).unwrap(), "any::<[u8; 32]>().prop_map(AccountId::from)");
        assert_eq!(strategy_for("Vec<u8>", 8).unwrap(), "prop::collection::vec(any::<u8>(), 0..=8)");
        assert_eq!(strategy_for("Option<u64>", 8).unwrap(), "proptest::option::of(any::<u64>())");
        assert_eq!(strategy_for("[u8;32]", 8).unwrap(), "any::<[u8; 32]>()");
        assert_eq!(strategy_for("String", 5).unwrap(), "\"[a-zA-Z0-9 ]{0,5}\"");
        assert!(strategy_for("MyCustomType", 8).is_none());
        assert!(strategy_for("Vec<MyCustomType>", 8).is_none());
        assert!(strategy_for("[u8;64]", 8).is_none());
    }

    #[test]
    fn generates_a_harness_per_message_with_the_configured_depth_and_cases() {
        let c = sample();
        let transfer = c.messages.iter().find(|m| m.name == "transfer").unwrap();
        let h = generate_harness(&c, transfer, &cfg());
        assert!(h.contains("fn fuzz_transfer("));
        assert!(h.contains("ProptestConfig::with_cases(64)"));
        assert!(h.contains("1..=3"));
        assert!(h.contains("use token_contract::token::Token;"));
        assert!(h.contains("let mut contract = Token::new(c0);"));
        assert!(h.contains("c0 in any::<u128>()"));
        assert!(h.contains("contract.transfer(a0.clone(), a1.clone(), a2.clone())"));
        assert!(h.contains("proptest::option::of(prop::collection::vec(any::<u8>(), 0..=16))"));
        assert!(h.contains("catch_unwind"));
        assert!(h.contains("ink::env::test::set_caller"));
    }

    #[test]
    fn includes_the_invariant_checks() {
        let c = sample();
        let h = generate_harness(&c, &c.messages[2], &cfg());
        assert!(h.contains("fn check_invariants(contract: &Token, caller: AccountId)"));
        assert!(h.contains("let _ = contract.total_supply();"));
        assert!(h.contains("contract.balance_of(caller) <= contract.total_supply()"));
        assert!(!h.contains("let _ = contract.balance_of("), "messages with arguments are not called bare");
    }

    #[test]
    fn a_message_without_arguments_gets_a_unit_strategy() {
        let c = sample();
        let ping = c.messages.iter().find(|m| m.name == "ping").unwrap();
        let h = generate_harness(&c, ping, &cfg());
        assert!(h.contains("vec(Just(()), 1..=3)"));
        assert!(h.contains("for () in calls"));
        assert!(h.contains("contract.ping()"));
    }

    #[test]
    fn unsupported_types_fall_back_and_are_noted() {
        let src = SAMPLE.replace("memo: Option<Vec<u8>>", "memo: Weird");
        let c = parse_contract(&src).unwrap();
        let transfer = c.messages.iter().find(|m| m.name == "transfer").unwrap();
        let h = generate_harness(&c, transfer, &cfg());
        assert!(h.contains("Just(<Weird as Default>::default())"));
        assert!(h.contains("No generator for: Weird"));
    }

    #[test]
    fn a_contract_without_a_constructor_uses_default() {
        let src = SAMPLE.replace("#[ink(constructor)]", "");
        let c = parse_contract(&src).unwrap();
        assert!(c.constructor.is_none());
        let h = generate_harness(&c, &c.messages[0], &cfg());
        assert!(h.contains("let mut contract = Token::default();"));
    }

    #[test]
    fn the_sample_generated_harness_is_valid_rust() {
        let c = sample();
        for m in &c.messages {
            let h = generate_harness(&c, m, &cfg());
            syn::parse_file(&h).unwrap_or_else(|e| panic!("harness for `{}` does not parse: {e}\n{h}", m.name));
        }
    }

    #[test]
    fn generates_a_manifest_for_the_harness_crate() {
        let m = generate_manifest("Token", &cfg(), "../token");
        assert!(m.contains("name = \"token-fuzz\""));
        assert!(m.contains("token_contract = { path = \"../token\" }"));
        assert!(m.contains("proptest = \"1\""));
        assert!(m.contains("[workspace]"));
    }
}
