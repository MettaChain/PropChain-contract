//! WASM contract size analyzer and optimizer report (#1470).
//!
//! Reads a compiled contract (`.wasm`), breaks its size down by section, ranks
//! the largest functions, and prints optimization recommendations. It can also
//! build a contract first (`cargo contract build --release`), compare against
//! sizes of other contracts, and diff two builds.
//!
//! ```text
//! wasm-size-analyzer --wasm FILE [options]
//! wasm-size-analyzer --build CONTRACT_DIR [options]
//! wasm-size-analyzer --diff OLD.wasm NEW.wasm
//!
//! options:
//!   --max-size BYTES     exit 1 if the contract is larger (default: no limit)
//!   --top N              number of largest functions to list (default 10)
//!   --source DIR         contract source dir, enables dependency / Vec checks
//!   --benchmarks DIR     dir of *.json size reports ({"contract":..,"size_bytes":..}) or *.wasm files
//!   --json               machine-readable output
//! ```
//!
//! Function names come from the `name` custom section when it is present
//! (release builds usually strip it); otherwise functions are reported by index.
//! The binary parser only walks section/function boundaries; it does not
//! validate or decode instructions.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SectionInfo {
    pub id: u8,
    /// Standard section name, or the custom section's own name.
    pub name: String,
    pub size: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FunctionInfo {
    /// Index in the module's function index space (imports come first).
    pub index: u32,
    pub size: usize,
    pub name: Option<String>,
}

impl FunctionInfo {
    pub fn label(&self) -> String {
        match &self.name {
            Some(n) => n.clone(),
            None => format!("func[{}]", self.index),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Analysis {
    pub total_size: usize,
    pub sections: Vec<SectionInfo>,
    pub functions: Vec<FunctionInfo>,
    pub imported_functions: u32,
}

impl Analysis {
    pub fn section_size(&self, name: &str) -> usize {
        self.sections
            .iter()
            .filter(|s| s.name == name)
            .map(|s| s.size)
            .sum()
    }

    pub fn custom_size(&self) -> usize {
        self.sections.iter().filter(|s| s.id == 0).map(|s| s.size).sum()
    }

    /// Functions sorted by size, largest first (ties by index).
    pub fn largest_functions(&self, n: usize) -> Vec<&FunctionInfo> {
        let mut v: Vec<&FunctionInfo> = self.functions.iter().collect();
        v.sort_by(|a, b| b.size.cmp(&a.size).then(a.index.cmp(&b.index)));
        v.truncate(n);
        v
    }
}

fn section_name(id: u8) -> &'static str {
    match id {
        1 => "type",
        2 => "import",
        3 => "function",
        4 => "table",
        5 => "memory",
        6 => "global",
        7 => "export",
        8 => "start",
        9 => "element",
        10 => "code",
        11 => "data",
        12 => "data_count",
        13 => "tag",
        _ => "unknown",
    }
}

fn read_leb_u32(bytes: &[u8], pos: &mut usize) -> Result<u32, String> {
    let mut result: u64 = 0;
    let mut shift = 0;
    loop {
        let b = *bytes.get(*pos).ok_or("unexpected end of data in LEB128")?;
        *pos += 1;
        result |= ((b & 0x7f) as u64) << shift;
        if b & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift >= 35 {
            return Err("LEB128 value too large".to_string());
        }
    }
    u32::try_from(result).map_err(|_| "LEB128 value exceeds u32".to_string())
}

fn read_name(bytes: &[u8], pos: &mut usize) -> Result<String, String> {
    let len = read_leb_u32(bytes, pos)? as usize;
    let end = pos.checked_add(len).ok_or("name length overflow")?;
    let slice = bytes.get(*pos..end).ok_or("name runs past the end")?;
    *pos = end;
    Ok(String::from_utf8_lossy(slice).into_owned())
}

fn skip_limits(bytes: &[u8], pos: &mut usize) -> Result<(), String> {
    let flag = *bytes.get(*pos).ok_or("missing limits flag")?;
    *pos += 1;
    read_leb_u32(bytes, pos)?;
    if flag & 1 == 1 {
        read_leb_u32(bytes, pos)?;
    }
    Ok(())
}

fn count_imported_functions(payload: &[u8]) -> Result<u32, String> {
    let mut pos = 0;
    let count = read_leb_u32(payload, &mut pos)?;
    let mut funcs = 0;
    for _ in 0..count {
        read_name(payload, &mut pos)?; // module
        read_name(payload, &mut pos)?; // field
        let kind = *payload.get(pos).ok_or("missing import kind")?;
        pos += 1;
        match kind {
            0 => {
                read_leb_u32(payload, &mut pos)?;
                funcs += 1;
            }
            1 => {
                pos += 1; // reference type
                skip_limits(payload, &mut pos)?;
            }
            2 => skip_limits(payload, &mut pos)?,
            3 => pos += 2, // value type + mutability
            other => return Err(format!("unsupported import kind {other}")),
        }
    }
    Ok(funcs)
}

fn parse_function_names(payload: &[u8]) -> Result<BTreeMap<u32, String>, String> {
    let mut pos = 0;
    let mut names = BTreeMap::new();
    while pos < payload.len() {
        let id = payload[pos];
        pos += 1;
        let size = read_leb_u32(payload, &mut pos)? as usize;
        let end = pos.checked_add(size).ok_or("subsection overflow")?;
        if end > payload.len() {
            return Err("name subsection runs past the end".to_string());
        }
        if id == 1 {
            let mut p = pos;
            let count = read_leb_u32(payload, &mut p)?;
            for _ in 0..count {
                let idx = read_leb_u32(payload, &mut p)?;
                let name = read_name(payload, &mut p)?;
                names.insert(idx, name);
            }
        }
        pos = end;
    }
    Ok(names)
}

/// Parse a WASM binary into section and function sizes.
pub fn analyze(bytes: &[u8]) -> Result<Analysis, String> {
    if bytes.len() < 8 || &bytes[0..4] != b"\0asm" {
        return Err("not a WASM module (missing \\0asm header)".to_string());
    }
    let mut pos = 8;
    let mut sections = Vec::new();
    let mut imported = 0u32;
    let mut code_sizes: Vec<usize> = Vec::new();
    let mut names = BTreeMap::new();
    while pos < bytes.len() {
        let id = bytes[pos];
        pos += 1;
        let size = read_leb_u32(bytes, &mut pos)? as usize;
        let end = pos.checked_add(size).ok_or("section size overflow")?;
        if end > bytes.len() {
            return Err(format!("section {id} runs past the end of the file"));
        }
        let payload = &bytes[pos..end];
        let mut name = section_name(id).to_string();
        match id {
            0 => {
                let mut p = 0;
                name = read_name(payload, &mut p)?;
                if name == "name" {
                    names = parse_function_names(&payload[p..]).unwrap_or_default();
                }
            }
            2 => imported = count_imported_functions(payload)?,
            10 => {
                let mut p = 0;
                let count = read_leb_u32(payload, &mut p)?;
                for _ in 0..count {
                    let body = read_leb_u32(payload, &mut p)? as usize;
                    p = p
                        .checked_add(body)
                        .filter(|e| *e <= payload.len())
                        .ok_or("function body runs past the code section")?;
                    code_sizes.push(body);
                }
            }
            _ => {}
        }
        sections.push(SectionInfo { id, name, size });
        pos = end;
    }
    let functions = code_sizes
        .iter()
        .enumerate()
        .map(|(i, size)| {
            let index = imported + i as u32;
            FunctionInfo {
                index,
                size: *size,
                name: names.get(&index).cloned(),
            }
        })
        .collect();
    Ok(Analysis {
        total_size: bytes.len(),
        sections,
        functions,
        imported_functions: imported,
    })
}

// ── Recommendations ───────────────────────────────────────────────────────

/// Recommendations derived from the binary alone.
pub fn binary_recommendations(a: &Analysis, max_size: Option<usize>) -> Vec<String> {
    let mut out = Vec::new();
    let total = a.total_size.max(1);
    let pct = |n: usize| n * 100 / total;
    if let Some(limit) = max_size {
        if a.total_size > limit {
            out.push(format!(
                "Contract is {} bytes, {} over the {} byte limit.",
                a.total_size,
                a.total_size - limit,
                limit
            ));
        }
    }
    if pct(a.custom_size()) >= 10 {
        out.push(format!(
            "Custom sections take {}% of the binary; strip the `name`/debug sections (build with --release and `strip = true`).",
            pct(a.custom_size())
        ));
    }
    let data = a.section_size("data");
    if pct(data) >= 20 {
        out.push(format!(
            "The data section is {}% of the binary; look for large string literals or static tables and move them off-chain.",
            pct(data)
        ));
    }
    let code = a.section_size("code").max(1);
    if let Some(top) = a.largest_functions(1).first() {
        if top.size * 100 / code >= 10 {
            out.push(format!(
                "`{}` is {} bytes ({}% of the code section); split it up or simplify it.",
                top.label(),
                top.size,
                top.size * 100 / code
            ));
        }
    }
    out
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Names under `[dependencies]` in a Cargo.toml (`name = ...` or `name.workspace = true`).
pub fn parse_dependencies(cargo_toml: &str) -> Vec<String> {
    let mut in_deps = false;
    let mut deps = Vec::new();
    for line in cargo_toml.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_deps = line == "[dependencies]";
            continue;
        }
        if !in_deps || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, _)) = line.split_once('=') {
            let name = key.trim().split('.').next().unwrap_or("").trim();
            if !name.is_empty() {
                deps.push(name.to_string());
            }
        }
    }
    deps
}

/// Dependencies whose crate name never appears in `source`.
///
/// `ink`, `scale` and `scale-info` are skipped: their derive macros reference
/// them without a visible `use`.
pub fn unused_dependencies(cargo_toml: &str, source: &str) -> Vec<String> {
    const MACRO_USED: &[&str] = &["ink", "scale", "scale-info", "parity-scale-codec"];
    parse_dependencies(cargo_toml)
        .into_iter()
        .filter(|d| !MACRO_USED.contains(&d.as_str()))
        .filter(|d| {
            let ident = d.replace('-', "_");
            !source.match_indices(&ident).any(|(i, _)| {
                let before = source[..i].chars().next_back();
                let after = source[i + ident.len()..].chars().next();
                !before.map(is_ident_char).unwrap_or(false)
                    && !after.map(is_ident_char).unwrap_or(false)
            })
        })
        .collect()
}

/// Recommendations derived from contract source and manifest.
pub fn source_recommendations(cargo_toml: &str, source: &str) -> Vec<String> {
    let mut out = Vec::new();
    for dep in unused_dependencies(cargo_toml, source) {
        out.push(format!(
            "Dependency `{dep}` is never referenced in the source; remove it to shrink the binary."
        ));
    }
    let vecs = source.matches("Vec<").count();
    if vecs >= 5 {
        out.push(format!(
            "`Vec<` appears {vecs} times; unbounded Vecs in storage are expensive. Prefer `Mapping` keyed by id for large collections."
        ));
    }
    let inline = source.matches("#[inline").count();
    let small_fns = source.matches("fn ").count();
    if small_fns >= 20 && inline == 0 {
        out.push(
            "No `#[inline]` hints in a contract with many small helpers; consider inlining tiny accessors (measure before and after)."
                .to_string(),
        );
    }
    out
}

// ── Comparison ────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diff {
    pub old_total: usize,
    pub new_total: usize,
    pub sections: Vec<(String, i64)>,
    /// `(function label, delta)` for functions whose size changed, biggest change first.
    pub functions: Vec<(String, i64)>,
}

impl Diff {
    pub fn delta(&self) -> i64 {
        self.new_total as i64 - self.old_total as i64
    }
}

fn section_totals(a: &Analysis) -> BTreeMap<String, usize> {
    let mut m = BTreeMap::new();
    for s in &a.sections {
        *m.entry(s.name.clone()).or_insert(0) += s.size;
    }
    m
}

/// Compare two analyses.
///
/// Functions are matched by name when both builds have names, otherwise by
/// index (which shifts when code is added or removed, so unnamed function diffs
/// are indicative only).
pub fn diff(old: &Analysis, new: &Analysis) -> Diff {
    let (o, n) = (section_totals(old), section_totals(new));
    let mut names: Vec<&String> = o.keys().chain(n.keys()).collect();
    names.sort();
    names.dedup();
    let sections = names
        .into_iter()
        .map(|k| {
            let a = *o.get(k).unwrap_or(&0) as i64;
            let b = *n.get(k).unwrap_or(&0) as i64;
            (k.clone(), b - a)
        })
        .filter(|(_, d)| *d != 0)
        .collect();

    let map = |a: &Analysis| -> BTreeMap<String, usize> {
        a.functions.iter().map(|f| (f.label(), f.size)).collect()
    };
    let (fo, fnew) = (map(old), map(new));
    let mut keys: Vec<&String> = fo.keys().chain(fnew.keys()).collect();
    keys.sort();
    keys.dedup();
    let mut functions: Vec<(String, i64)> = keys
        .into_iter()
        .map(|k| {
            let a = *fo.get(k).unwrap_or(&0) as i64;
            let b = *fnew.get(k).unwrap_or(&0) as i64;
            (k.clone(), b - a)
        })
        .filter(|(_, d)| *d != 0)
        .collect();
    functions.sort_by(|a, b| b.1.abs().cmp(&a.1.abs()).then(a.0.cmp(&b.0)));
    Diff {
        old_total: old.total_size,
        new_total: new.total_size,
        sections,
        functions,
    }
}

/// Median of `sizes` (average of the two middle values for an even count).
pub fn median(sizes: &[usize]) -> Option<usize> {
    if sizes.is_empty() {
        return None;
    }
    let mut v = sizes.to_vec();
    v.sort_unstable();
    let mid = v.len() / 2;
    Some(if v.len() % 2 == 1 {
        v[mid]
    } else {
        (v[mid - 1] + v[mid]) / 2
    })
}

/// Extract `"size_bytes": N` from a JSON size report such as the one written by
/// `scripts/check-wasm-size.sh`.
pub fn parse_size_report(json: &str) -> Option<usize> {
    let key = json.find("\"size_bytes\"")?;
    let rest = &json[key + "\"size_bytes\"".len()..];
    let rest = rest.trim_start().strip_prefix(':')?.trim_start();
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok()
}

fn load_benchmarks(dir: &Path) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        paths.sort();
        for p in paths {
            let stem = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            match p.extension().and_then(|e| e.to_str()) {
                Some("json") => {
                    if let Some(size) = fs::read_to_string(&p).ok().and_then(|t| parse_size_report(&t)) {
                        out.push((stem, size));
                    }
                }
                Some("wasm") => {
                    if let Ok(meta) = fs::metadata(&p) {
                        out.push((stem, meta.len() as usize));
                    }
                }
                _ => {}
            }
        }
    }
    out
}

// ── Output ────────────────────────────────────────────────────────────────

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

pub fn report_text(
    name: &str,
    a: &Analysis,
    top: usize,
    recs: &[String],
    benchmarks: &[(String, usize)],
) -> String {
    let mut out = format!("{name}: {} bytes\n\nSections:\n", a.total_size);
    for s in &a.sections {
        out.push_str(&format!(
            "  {:<14} {:>9} bytes ({}%)\n",
            s.name,
            s.size,
            s.size * 100 / a.total_size.max(1)
        ));
    }
    out.push_str(&format!("\nLargest functions (top {top}):\n"));
    for f in a.largest_functions(top) {
        out.push_str(&format!("  {:<24} {:>8} bytes\n", f.label(), f.size));
    }
    if !benchmarks.is_empty() {
        let sizes: Vec<usize> = benchmarks.iter().map(|(_, s)| *s).collect();
        let med = median(&sizes).unwrap_or(0);
        out.push_str(&format!(
            "\nBenchmark: {} other contract(s), median {} bytes; this contract is {}% of the median.\n",
            benchmarks.len(),
            med,
            if med == 0 { 0 } else { a.total_size * 100 / med }
        ));
    }
    out.push_str("\nRecommendations:\n");
    if recs.is_empty() {
        out.push_str("  none\n");
    }
    for r in recs {
        out.push_str(&format!("  - {r}\n"));
    }
    out
}

pub fn report_json(name: &str, a: &Analysis, top: usize, recs: &[String]) -> String {
    let sections: Vec<String> = a
        .sections
        .iter()
        .map(|s| format!("{{\"name\":\"{}\",\"size\":{}}}", json_escape(&s.name), s.size))
        .collect();
    let funcs: Vec<String> = a
        .largest_functions(top)
        .iter()
        .map(|f| format!("{{\"name\":\"{}\",\"size\":{}}}", json_escape(&f.label()), f.size))
        .collect();
    let recs: Vec<String> = recs.iter().map(|r| format!("\"{}\"", json_escape(r))).collect();
    format!(
        "{{\"contract\":\"{}\",\"size_bytes\":{},\"sections\":[{}],\"largest_functions\":[{}],\"recommendations\":[{}]}}",
        json_escape(name),
        a.total_size,
        sections.join(","),
        funcs.join(","),
        recs.join(",")
    )
}

pub fn diff_text(d: &Diff) -> String {
    let mut out = format!(
        "{} -> {} bytes ({:+})\n\nSection changes:\n",
        d.old_total,
        d.new_total,
        d.delta()
    );
    if d.sections.is_empty() {
        out.push_str("  none\n");
    }
    for (name, delta) in &d.sections {
        out.push_str(&format!("  {name:<14} {delta:+}\n"));
    }
    out.push_str("\nFunction changes:\n");
    if d.functions.is_empty() {
        out.push_str("  none\n");
    }
    for (name, delta) in d.functions.iter().take(20) {
        out.push_str(&format!("  {name:<24} {delta:+}\n"));
    }
    out
}

// ── CLI ───────────────────────────────────────────────────────────────────

fn read_source_tree(dir: &Path) -> String {
    let mut text = String::new();
    fn walk(p: &Path, text: &mut String) {
        if let Ok(entries) = fs::read_dir(p) {
            let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            paths.sort();
            for path in paths {
                if path.is_dir() {
                    if path.file_name().map(|n| n == "target").unwrap_or(false) {
                        continue;
                    }
                    walk(&path, text);
                } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                    if let Ok(s) = fs::read_to_string(&path) {
                        text.push_str(&s);
                        text.push('\n');
                    }
                }
            }
        }
    }
    walk(dir, &mut text);
    text
}

fn fail(msg: &str) -> ! {
    eprintln!("error: {msg}");
    std::process::exit(2)
}

fn main() {
    let mut args = std::env::args().skip(1);
    let (mut wasm, mut build, mut source, mut bench) = (None, None, None, None);
    let mut diff_paths: Option<(String, String)> = None;
    let (mut max_size, mut top, mut json) = (None, 10usize, false);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--wasm" => wasm = args.next(),
            "--build" => build = args.next(),
            "--source" => source = args.next(),
            "--benchmarks" => bench = args.next(),
            "--diff" => {
                let (o, n) = (args.next(), args.next());
                match (o, n) {
                    (Some(o), Some(n)) => diff_paths = Some((o, n)),
                    _ => fail("--diff needs two files"),
                }
            }
            "--max-size" => max_size = args.next().and_then(|v| v.parse::<usize>().ok()),
            "--top" => top = args.next().and_then(|v| v.parse().ok()).unwrap_or(10),
            "--json" => json = true,
            other => fail(&format!("unknown argument `{other}`")),
        }
    }

    if let Some((o, n)) = diff_paths {
        let read = |p: &str| -> Analysis {
            let bytes = fs::read(p).unwrap_or_else(|e| fail(&format!("cannot read {p}: {e}")));
            analyze(&bytes).unwrap_or_else(|e| fail(&format!("{p}: {e}")))
        };
        let d = diff(&read(&o), &read(&n));
        print!("{}", diff_text(&d));
        if let Some(limit) = max_size {
            if d.new_total > limit {
                std::process::exit(1);
            }
        }
        return;
    }

    let mut source_dir = source.map(PathBuf::from);
    let (name, path) = if let Some(dir) = build {
        let manifest = Path::new(&dir).join("Cargo.toml");
        let status = Command::new("cargo")
            .args(["contract", "build", "--release", "--manifest-path"])
            .arg(&manifest)
            .status()
            .unwrap_or_else(|e| fail(&format!("could not run `cargo contract build`: {e}")));
        if !status.success() {
            fail("`cargo contract build` failed");
        }
        let name = Path::new(&dir)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        source_dir.get_or_insert_with(|| PathBuf::from(&dir));
        let wasm_path = Path::new(&dir).join("target/ink").join(format!("{name}.wasm"));
        (name, wasm_path)
    } else if let Some(w) = wasm {
        let name = Path::new(&w)
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        (name, PathBuf::from(w))
    } else {
        fail("give --wasm FILE, --build CONTRACT_DIR or --diff OLD NEW");
    };

    let bytes = fs::read(&path).unwrap_or_else(|e| fail(&format!("cannot read {}: {e}", path.display())));
    let analysis = analyze(&bytes).unwrap_or_else(|e| fail(&format!("{}: {e}", path.display())));

    let mut recs = binary_recommendations(&analysis, max_size);
    if let Some(dir) = &source_dir {
        let manifest = fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
        recs.extend(source_recommendations(&manifest, &read_source_tree(dir)));
    }
    let benchmarks = bench.map(|d| load_benchmarks(Path::new(&d))).unwrap_or_default();

    if json {
        println!("{}", report_json(&name, &analysis, top, &recs));
    } else {
        print!("{}", report_text(&name, &analysis, top, &recs, &benchmarks));
    }
    if let Some(limit) = max_size {
        if analysis.total_size > limit {
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn leb(mut n: u32) -> Vec<u8> {
        let mut v = Vec::new();
        loop {
            let b = (n & 0x7f) as u8;
            n >>= 7;
            if n != 0 {
                v.push(b | 0x80);
            } else {
                v.push(b);
                return v;
            }
        }
    }

    fn section(id: u8, payload: &[u8]) -> Vec<u8> {
        let mut v = vec![id];
        v.extend(leb(payload.len() as u32));
        v.extend_from_slice(payload);
        v
    }

    fn code_section(bodies: &[usize]) -> Vec<u8> {
        let mut p = leb(bodies.len() as u32);
        for len in bodies {
            p.extend(leb(*len as u32));
            p.extend(std::iter::repeat_n(0u8, *len));
        }
        section(10, &p)
    }

    fn name_section(names: &[(u32, &str)]) -> Vec<u8> {
        let mut sub = leb(names.len() as u32);
        for (idx, n) in names {
            sub.extend(leb(*idx));
            sub.extend(leb(n.len() as u32));
            sub.extend_from_slice(n.as_bytes());
        }
        let mut p = leb(4);
        p.extend_from_slice(b"name");
        p.push(1);
        p.extend(leb(sub.len() as u32));
        p.extend(sub);
        section(0, &p)
    }

    fn module(sections: Vec<Vec<u8>>) -> Vec<u8> {
        let mut v = b"\0asm\x01\0\0\0".to_vec();
        for s in sections {
            v.extend(s);
        }
        v
    }

    /// One imported function plus bodies of the given sizes.
    fn sample(bodies: &[usize], names: Option<&[(u32, &str)]>) -> Vec<u8> {
        let mut import = leb(1);
        import.extend(leb(3));
        import.extend_from_slice(b"env");
        import.extend(leb(1));
        import.extend_from_slice(b"f");
        import.push(0); // function import
        import.extend(leb(0)); // type index
        let mut secs = vec![section(2, &import), code_section(bodies)];
        if let Some(n) = names {
            secs.push(name_section(n));
        }
        module(secs)
    }

    #[test]
    fn rejects_non_wasm_input() {
        assert!(analyze(b"hello world").is_err());
        assert!(analyze(&[]).is_err());
    }

    #[test]
    fn empty_module_has_no_sections() {
        let a = analyze(&module(vec![])).unwrap();
        assert_eq!(a.total_size, 8);
        assert!(a.sections.is_empty() && a.functions.is_empty());
    }

    #[test]
    fn reports_sections_and_function_sizes() {
        let a = analyze(&sample(&[10, 300, 25], None)).unwrap();
        assert_eq!(a.imported_functions, 1);
        assert_eq!(a.functions.len(), 3);
        // Imports come first in the index space.
        assert_eq!(a.functions[0].index, 1);
        assert_eq!(a.functions[1].size, 300);
        assert!(a.section_size("import") > 0);
        assert!(a.section_size("code") > 335);
    }

    #[test]
    fn largest_functions_are_sorted_descending() {
        let a = analyze(&sample(&[10, 300, 25], None)).unwrap();
        let top: Vec<usize> = a.largest_functions(2).iter().map(|f| f.size).collect();
        assert_eq!(top, vec![300, 25]);
    }

    #[test]
    fn function_names_come_from_the_name_section() {
        let a = analyze(&sample(&[10, 20], Some(&[(1, "deploy"), (2, "call")]))).unwrap();
        assert_eq!(a.functions[0].label(), "deploy");
        assert_eq!(a.functions[1].label(), "call");
        assert!(a.custom_size() > 0);
    }

    #[test]
    fn unnamed_functions_use_their_index() {
        let a = analyze(&sample(&[5], None)).unwrap();
        assert_eq!(a.functions[0].label(), "func[1]");
    }

    #[test]
    fn truncated_sections_are_an_error_not_a_panic() {
        let mut bytes = sample(&[10], None);
        bytes.truncate(bytes.len() - 4);
        assert!(analyze(&bytes).is_err());
    }

    #[test]
    fn leb128_round_trips_multi_byte_values() {
        let mut pos = 0;
        assert_eq!(read_leb_u32(&leb(624_485), &mut pos), Ok(624_485));
        let mut pos = 0;
        assert!(read_leb_u32(&[0x80], &mut pos).is_err());
    }

    #[test]
    fn diff_reports_total_section_and_function_changes() {
        let old = analyze(&sample(&[10, 100], Some(&[(1, "a"), (2, "b")]))).unwrap();
        let new = analyze(&sample(&[10, 250], Some(&[(1, "a"), (2, "b")]))).unwrap();
        let d = diff(&old, &new);
        assert!(d.delta() > 0);
        assert_eq!(d.functions, vec![("b".to_string(), 150)]);
        assert!(d.sections.iter().any(|(n, delta)| n == "code" && *delta > 0));
        assert!(diff_text(&d).contains("Function changes"));
    }

    #[test]
    fn diff_of_identical_builds_is_empty() {
        let a = analyze(&sample(&[10, 100], None)).unwrap();
        let d = diff(&a, &a);
        assert_eq!(d.delta(), 0);
        assert!(d.sections.is_empty() && d.functions.is_empty());
    }

    #[test]
    fn size_limit_produces_a_recommendation() {
        let a = analyze(&sample(&[1000], None)).unwrap();
        let over = binary_recommendations(&a, Some(100));
        assert!(over.iter().any(|r| r.contains("over the 100 byte limit")));
        let ok = binary_recommendations(&a, Some(1_000_000));
        assert!(!ok.iter().any(|r| r.contains("byte limit")));
    }

    #[test]
    fn a_dominant_function_is_called_out() {
        let a = analyze(&sample(&[10, 5000], None)).unwrap();
        let recs = binary_recommendations(&a, None);
        assert!(recs.iter().any(|r| r.contains("func[2]")));
    }

    #[test]
    fn custom_sections_over_ten_percent_suggest_stripping() {
        let a = analyze(&sample(&[5], Some(&[(1, "a_very_long_function_name_to_inflate_custom")]))).unwrap();
        assert!(binary_recommendations(&a, None)
            .iter()
            .any(|r| r.contains("Custom sections")));
    }

    #[test]
    fn parses_dependency_names() {
        let toml = "[package]\nname = \"x\"\n\n[dependencies]\nink = { workspace = true }\nserde.workspace = true\n# note\nhex = \"0.4\"\n\n[dev-dependencies]\ntempfile = \"3\"\n";
        assert_eq!(parse_dependencies(toml), vec!["ink", "serde", "hex"]);
    }

    #[test]
    fn finds_unused_dependencies_by_whole_word_match() {
        let toml = "[dependencies]\nink = \"5\"\nhex = \"0.4\"\nserde-json = \"1\"\nsha2 = \"0.10\"\n";
        let src = "use hex::encode; fn f() { let sha2_like = 1; }";
        // `sha2_like` must not count as a use of `sha2`; `ink` is macro-used.
        assert_eq!(unused_dependencies(toml, src), vec!["serde-json", "sha2"]);
    }

    #[test]
    fn source_recommendations_cover_vecs_and_unused_deps() {
        let toml = "[dependencies]\nunused-crate = \"1\"\n";
        let src = "Vec<u8> Vec<u8> Vec<u8> Vec<u8> Vec<u8>";
        let recs = source_recommendations(toml, src);
        assert!(recs.iter().any(|r| r.contains("unused-crate")));
        assert!(recs.iter().any(|r| r.contains("Mapping")));
    }

    #[test]
    fn size_reports_and_medians() {
        assert_eq!(parse_size_report("{\"contract\": \"x\", \"size_bytes\": 12345}"), Some(12345));
        assert_eq!(parse_size_report("{}"), None);
        assert_eq!(median(&[]), None);
        assert_eq!(median(&[3, 1, 2]), Some(2));
        assert_eq!(median(&[1, 2, 3, 4]), Some(2));
    }

    #[test]
    fn text_and_json_reports_name_the_contract() {
        let a = analyze(&sample(&[10, 20], Some(&[(1, "x"), (2, "y")]))).unwrap();
        let text = report_text("demo", &a, 5, &[], &[("other".into(), 100)]);
        assert!(text.starts_with("demo:"));
        assert!(text.contains("Largest functions"));
        assert!(text.contains("Benchmark: 1 other contract(s)"));
        let json = report_json("demo", &a, 5, &["tip".to_string()]);
        assert!(json.contains("\"contract\":\"demo\""));
        assert!(json.contains("\"recommendations\":[\"tip\"]"));
    }
}
