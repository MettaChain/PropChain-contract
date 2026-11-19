//! Dependency audit and license compliance checker (#1472).
//!
//! Walks a folder for `Cargo.toml` files, extracts every dependency and reports,
//! per dependency:
//!
//! * **vulnerable**: it appears in a `cargo audit --json` report,
//! * **license issue**: its license (from `cargo metadata`) is not allowed, or unknown,
//! * **wildcard / unpinned**: its version requirement is `*`-like or open-ended
//!   (`>=1`), or only partly specified (`"1"`, `"1.2"`); an exact `=1.2.3` is pinned and
//!   a full `1.2.3` is a normal caret requirement,
//! * **duplicate**: different manifests ask for different versions of it.
//!
//! The vulnerability and license data come from the usual tools, either as files
//! (`--audit audit.json`, `--metadata metadata.json`) or by running them
//! (`--run-audit`, `--run-metadata`). Without either, that check is reported as
//! "not checked" rather than silently passing.
//!
//! Allowed licenses default to the policy in `deny.toml` (MIT, Apache-2.0,
//! BSD-3-Clause) plus a few equally permissive ones; add more with `--allow-license`.
//!
//! ```text
//! dependency-audit [--root DIR] [--audit FILE | --run-audit] [--metadata FILE | --run-metadata]
//!                  [--allow-license SPDX]... [--format text|json]
//! ```
//! Exit code 1 if a vulnerability or license issue is found, 2 on usage or I/O errors.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

pub const DEFAULT_ALLOWED_LICENSES: &[&str] = &[
    "MIT",
    "Apache-2.0",
    "BSD-3-Clause",
    "BSD-2-Clause",
    "ISC",
    "Zlib",
    "CC0-1.0",
    "Unlicense",
    "Unicode-3.0",
    "Unicode-DFS-2016",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum Kind {
    Normal,
    Dev,
    Build,
    Workspace,
}

impl Kind {
    fn as_str(self) -> &'static str {
        match self {
            Kind::Normal => "dependencies",
            Kind::Dev => "dev-dependencies",
            Kind::Build => "build-dependencies",
            Kind::Workspace => "workspace.dependencies",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Source {
    Registry,
    Path,
    Git { pinned: bool },
    /// `foo = { workspace = true }`: the requirement lives in the workspace table.
    Inherited,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dependency {
    pub name: String,
    pub requirement: String,
    pub kind: Kind,
    pub source: Source,
    pub manifest: String,
}

/// Extracts the dependencies declared by one `Cargo.toml`.
pub fn parse_manifest(label: &str, text: &str) -> Result<Vec<Dependency>, String> {
    let doc: toml::Table = text.parse().map_err(|e| format!("{label}: {e}"))?;
    let mut out = Vec::new();
    let mut collect = |table: Option<&toml::Value>, kind: Kind| {
        let Some(table) = table.and_then(|t| t.as_table()) else { return };
        for (name, value) in table {
            out.push(parse_dependency(label, name, value, kind));
        }
    };
    collect(doc.get("dependencies"), Kind::Normal);
    collect(doc.get("dev-dependencies"), Kind::Dev);
    collect(doc.get("build-dependencies"), Kind::Build);
    collect(doc.get("workspace").and_then(|w| w.get("dependencies")), Kind::Workspace);
    Ok(out)
}

fn parse_dependency(manifest: &str, name: &str, value: &toml::Value, kind: Kind) -> Dependency {
    let make = |requirement: String, source: Source| Dependency {
        name: name.to_string(),
        requirement,
        kind,
        source,
        manifest: manifest.to_string(),
    };
    match value {
        toml::Value::String(req) => make(req.clone(), Source::Registry),
        toml::Value::Table(t) => {
            let req = t.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if t.get("workspace").and_then(|v| v.as_bool()).unwrap_or(false) {
                make(req, Source::Inherited)
            } else if t.contains_key("path") {
                make(req, Source::Path)
            } else if t.contains_key("git") {
                let pinned = t.contains_key("rev") || t.contains_key("tag");
                make(req, Source::Git { pinned })
            } else {
                make(req, Source::Registry)
            }
        }
        _ => make(String::new(), Source::Registry),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pin {
    /// `=1.2.3`
    Exact,
    /// A complete requirement with a lower bound at least, such as `1.2.3`, `^1.2.3` or `~1.2.3`.
    Compatible,
    /// Only partly specified (`1`, `1.2`) or open-ended (`>=1.2`).
    Unpinned,
    /// `*`, `1.*`, or empty.
    Wildcard,
}

/// Classifies a version requirement string.
pub fn classify_version(req: &str) -> Pin {
    let req = req.trim();
    if req.is_empty() || req.contains('*') {
        return Pin::Wildcard;
    }
    if req.starts_with('=') {
        return Pin::Exact;
    }
    let has_lower_only = (req.contains('>') || req.contains('<')) && !(req.contains('<') && req.contains(','));
    if has_lower_only {
        return Pin::Unpinned;
    }
    if req.contains(',') {
        return Pin::Compatible; // bounded range such as ">=1.2, <2"
    }
    let bare = req.trim_start_matches(|c| c == '^' || c == '~').trim();
    let parts: Vec<&str> = bare.split('.').collect();
    if parts.len() >= 3 && parts.iter().all(|p| !p.is_empty()) {
        Pin::Compatible
    } else {
        Pin::Unpinned
    }
}

/// Evaluates a (simple) SPDX license expression against an allow list.
///
/// `A OR B` is allowed if either side is; `A AND B` only if both are; a `WITH exception`
/// suffix is ignored; parentheses are flattened. `None` and `NOASSERTION` are never allowed.
pub fn license_allowed(expr: &str, allowed: &[String]) -> bool {
    let expr = expr.replace(['(', ')'], " ").replace('/', " OR ");
    let expr = expr.trim();
    if expr.is_empty() || expr.eq_ignore_ascii_case("NOASSERTION") {
        return false;
    }
    expr.split(" OR ").any(|alternative| {
        alternative.split(" AND ").all(|term| {
            let id = term.split(" WITH ").next().unwrap_or("").trim();
            allowed.iter().any(|a| a.eq_ignore_ascii_case(id))
        })
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub license: Option<String>,
}

/// Parses `cargo metadata --format-version 1` output into package licenses.
pub fn parse_metadata(json: &str) -> Result<Vec<PackageInfo>, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("metadata: {e}"))?;
    let packages = v["packages"].as_array().ok_or("metadata: no `packages` array")?;
    Ok(packages
        .iter()
        .filter_map(|p| {
            Some(PackageInfo {
                name: p["name"].as_str()?.to_string(),
                version: p["version"].as_str().unwrap_or("").to_string(),
                license: p["license"].as_str().map(str::to_string),
            })
        })
        .collect())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vulnerability {
    pub package: String,
    pub version: String,
    pub id: String,
    pub title: String,
}

/// Parses `cargo audit --json` output.
pub fn parse_audit(json: &str) -> Result<Vec<Vulnerability>, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| format!("audit: {e}"))?;
    let list = v["vulnerabilities"]["list"].as_array().cloned().unwrap_or_default();
    Ok(list
        .iter()
        .map(|e| Vulnerability {
            package: e["package"]["name"].as_str().unwrap_or("").to_string(),
            version: e["package"]["version"].as_str().unwrap_or("").to_string(),
            id: e["advisory"]["id"].as_str().unwrap_or("").to_string(),
            title: e["advisory"]["title"].as_str().unwrap_or("").to_string(),
        })
        .collect())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Check<T> {
    NotChecked,
    Done(T),
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct DependencyReport {
    pub name: String,
    pub requirements: BTreeSet<String>,
    pub manifests: BTreeSet<String>,
    pub sections: BTreeSet<&'static str>,
    pub vulnerabilities: Vec<String>,
    pub license_issues: Vec<String>,
    pub pin_issues: Vec<String>,
    pub duplicate_versions: Vec<String>,
}

impl DependencyReport {
    pub fn failing(&self) -> bool {
        !self.vulnerabilities.is_empty() || !self.license_issues.is_empty()
    }

    fn statuses(&self) -> Vec<&'static str> {
        let mut s = Vec::new();
        if !self.vulnerabilities.is_empty() {
            s.push("vulnerable");
        }
        if !self.license_issues.is_empty() {
            s.push("license-issue");
        }
        if !self.pin_issues.is_empty() {
            s.push("unpinned");
        }
        if !self.duplicate_versions.is_empty() {
            s.push("duplicate-versions");
        }
        if s.is_empty() {
            s.push("ok");
        }
        s
    }
}

pub struct Inputs<'a> {
    pub dependencies: &'a [Dependency],
    pub vulnerabilities: Check<Vec<Vulnerability>>,
    pub packages: Check<Vec<PackageInfo>>,
    pub allowed_licenses: Vec<String>,
}

/// Combines the parsed manifests with the audit and license data.
pub fn build_report(inputs: &Inputs) -> Vec<DependencyReport> {
    let mut by_name: BTreeMap<String, DependencyReport> = BTreeMap::new();
    for d in inputs.dependencies {
        let r = by_name.entry(d.name.clone()).or_insert_with(|| DependencyReport {
            name: d.name.clone(),
            ..Default::default()
        });
        r.manifests.insert(d.manifest.clone());
        r.sections.insert(d.kind.as_str());
        match &d.source {
            Source::Registry => {
                r.requirements.insert(d.requirement.clone());
                match classify_version(&d.requirement) {
                    Pin::Wildcard => r.pin_issues.push(format!("wildcard requirement `{}` in {}", d.requirement, d.manifest)),
                    Pin::Unpinned => r.pin_issues.push(format!("unpinned requirement `{}` in {}", d.requirement, d.manifest)),
                    Pin::Exact | Pin::Compatible => {}
                }
            }
            Source::Git { pinned: false } => {
                r.pin_issues.push(format!("git dependency without `rev` or `tag` in {}", d.manifest));
            }
            Source::Git { pinned: true } | Source::Path | Source::Inherited => {}
        }
    }
    for r in by_name.values_mut() {
        if r.requirements.len() > 1 {
            r.duplicate_versions = r.requirements.iter().cloned().collect();
        }
        r.pin_issues.sort();
        r.pin_issues.dedup();
    }
    if let Check::Done(vulns) = &inputs.vulnerabilities {
        for v in vulns {
            if let Some(r) = by_name.get_mut(&v.package) {
                r.vulnerabilities.push(format!("{} ({} {}): {}", v.id, v.package, v.version, v.title));
            }
        }
    }
    if let Check::Done(packages) = &inputs.packages {
        for r in by_name.values_mut() {
            for p in packages.iter().filter(|p| p.name == r.name) {
                match &p.license {
                    None => r.license_issues.push(format!("{} {}: no license declared", p.name, p.version)),
                    Some(l) if !license_allowed(l, &inputs.allowed_licenses) => {
                        r.license_issues.push(format!("{} {}: license `{}` is not allowed", p.name, p.version, l))
                    }
                    Some(_) => {}
                }
            }
            r.license_issues.sort();
            r.license_issues.dedup();
        }
    }
    by_name.into_values().collect()
}

/// Every `Cargo.toml` below `root`, skipping build output and VCS folders.
pub fn find_manifests(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                if !matches!(name.as_str(), "target" | ".git" | "node_modules") {
                    walk(&path, out);
                }
            } else if name == "Cargo.toml" {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, &mut out);
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

fn json_list<'a>(items: impl Iterator<Item = &'a String>) -> String {
    format!("[{}]", items.map(|s| format!("\"{}\"", json_escape(s))).collect::<Vec<_>>().join(","))
}

pub fn render_json(report: &[DependencyReport], vuln_checked: bool, license_checked: bool) -> String {
    let deps = report
        .iter()
        .map(|r| {
            format!(
                "{{\"name\":\"{}\",\"status\":{},\"sections\":{},\"requirements\":{},\"manifests\":{},\"vulnerabilities\":{},\"license_issues\":{},\"pin_issues\":{},\"duplicate_versions\":{}}}",
                json_escape(&r.name),
                format!("[{}]", r.statuses().iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(",")),
                format!("[{}]", r.sections.iter().map(|s| format!("\"{s}\"")).collect::<Vec<_>>().join(",")),
                json_list(r.requirements.iter()),
                json_list(r.manifests.iter()),
                json_list(r.vulnerabilities.iter()),
                json_list(r.license_issues.iter()),
                json_list(r.pin_issues.iter()),
                json_list(r.duplicate_versions.iter()),
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"passed\":{},\"vulnerabilities_checked\":{},\"licenses_checked\":{},\"dependencies\":[{}]}}\n",
        !report.iter().any(|r| r.failing()),
        vuln_checked,
        license_checked,
        deps
    )
}

pub fn render_text(report: &[DependencyReport], vuln_checked: bool, license_checked: bool) -> String {
    let mut out = String::from("Dependency audit\n\n");
    out.push_str(&format!("{:<32} {:<34} {}\n", "dependency", "requirement(s)", "status"));
    for r in report {
        let reqs = r.requirements.iter().cloned().collect::<Vec<_>>().join(", ");
        out.push_str(&format!("{:<32} {:<34} {}\n", r.name, reqs, r.statuses().join(", ")));
        for line in r.vulnerabilities.iter().chain(&r.license_issues).chain(&r.pin_issues) {
            out.push_str(&format!("    - {line}\n"));
        }
        if !r.duplicate_versions.is_empty() {
            out.push_str(&format!("    - versions requested: {}\n", r.duplicate_versions.join(", ")));
        }
    }
    if !vuln_checked {
        out.push_str("\nNOT CHECKED: vulnerabilities (pass --audit FILE or --run-audit)\n");
    }
    if !license_checked {
        out.push_str("NOT CHECKED: licenses (pass --metadata FILE or --run-metadata)\n");
    }
    let failing = report.iter().filter(|r| r.failing()).count();
    out.push_str(&format!("\n{} dependencies, {} failing\n", report.len(), failing));
    out
}

fn run(cmd: &str, args: &[&str], dir: &Path) -> Result<String, String> {
    let out = Command::new(cmd).args(args).current_dir(dir).output().map_err(|e| format!("{cmd}: {e}"))?;
    // `cargo audit` exits non-zero when it finds vulnerabilities but still prints the report.
    if out.stdout.is_empty() {
        return Err(format!("{cmd}: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

fn usage() -> ExitCode {
    eprintln!(
        "usage: dependency-audit [--root DIR] [--audit FILE | --run-audit] [--metadata FILE | --run-metadata] [--allow-license SPDX]... [--format text|json]"
    );
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut root = PathBuf::from(".");
    let (mut audit_file, mut meta_file) = (None::<String>, None::<String>);
    let (mut run_audit, mut run_meta) = (false, false);
    let mut extra_licenses: Vec<String> = Vec::new();
    let mut format = "text".to_string();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--root" => root = PathBuf::from(args.get(i + 1).cloned().unwrap_or_default()),
            "--audit" => audit_file = args.get(i + 1).cloned(),
            "--metadata" => meta_file = args.get(i + 1).cloned(),
            "--allow-license" => extra_licenses.extend(args.get(i + 1).cloned()),
            "--format" => format = args.get(i + 1).cloned().unwrap_or_default(),
            "--run-audit" => {
                run_audit = true;
                i += 1;
                continue;
            }
            "--run-metadata" => {
                run_meta = true;
                i += 1;
                continue;
            }
            _ => return usage(),
        }
        i += 2;
    }
    if format != "text" && format != "json" {
        return usage();
    }

    let mut dependencies = Vec::new();
    for manifest in find_manifests(&root) {
        let label = manifest.display().to_string();
        let parsed = fs::read_to_string(&manifest)
            .map_err(|e| format!("{label}: {e}"))
            .and_then(|text| parse_manifest(&label, &text));
        match parsed {
            Ok(mut deps) => dependencies.append(&mut deps),
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        }
    }

    let fetch = |file: &Option<String>, run_it: bool, cmd: &str, cmd_args: &[&str]| -> Result<Option<String>, String> {
        if let Some(f) = file {
            return fs::read_to_string(f).map(Some).map_err(|e| format!("{f}: {e}"));
        }
        if run_it {
            return run(cmd, cmd_args, &root).map(Some);
        }
        Ok(None)
    };
    let audit_text = match fetch(&audit_file, run_audit, "cargo", &["audit", "--json"]) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let meta_text = match fetch(&meta_file, run_meta, "cargo", &["metadata", "--format-version", "1"]) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let vulnerabilities = match audit_text.map(|t| parse_audit(&t)).transpose() {
        Ok(Some(v)) => Check::Done(v),
        Ok(None) => Check::NotChecked,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let packages = match meta_text.map(|t| parse_metadata(&t)).transpose() {
        Ok(Some(p)) => Check::Done(p),
        Ok(None) => Check::NotChecked,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    let mut allowed: Vec<String> = DEFAULT_ALLOWED_LICENSES.iter().map(|s| s.to_string()).collect();
    allowed.extend(extra_licenses);
    let vuln_checked = matches!(vulnerabilities, Check::Done(_));
    let license_checked = matches!(packages, Check::Done(_));
    let report = build_report(&Inputs { dependencies: &dependencies, vulnerabilities, packages, allowed_licenses: allowed });

    if format == "json" {
        print!("{}", render_json(&report, vuln_checked, license_checked));
    } else {
        print!("{}", render_text(&report, vuln_checked, license_checked));
    }
    if report.iter().any(|r| r.failing()) {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r#"
[package]
name = "demo"
version = "0.1.0"

[dependencies]
syn = { version = "2.0.1", features = ["full"] }
quote = "1"
wild = "*"
open = ">=1.0"
exact = "=1.2.3"
local = { path = "../local" }
gitdep = { git = "https://example.com/x.git" }
gitpinned = { git = "https://example.com/y.git", rev = "abc123" }
shared = { workspace = true }

[dev-dependencies]
proptest = "1.5.0"

[build-dependencies]
cc = "1.0.0"

[workspace.dependencies]
serde = "1.0.100"
"#;

    fn deps() -> Vec<Dependency> {
        parse_manifest("demo/Cargo.toml", MANIFEST).unwrap()
    }

    fn find<'a>(d: &'a [Dependency], name: &str) -> &'a Dependency {
        d.iter().find(|x| x.name == name).unwrap_or_else(|| panic!("{name} missing"))
    }

    #[test]
    fn parses_every_dependency_section() {
        let d = deps();
        assert_eq!(find(&d, "syn").kind, Kind::Normal);
        assert_eq!(find(&d, "proptest").kind, Kind::Dev);
        assert_eq!(find(&d, "cc").kind, Kind::Build);
        assert_eq!(find(&d, "serde").kind, Kind::Workspace);
        assert_eq!(d.len(), 12);
    }

    #[test]
    fn reads_versions_from_strings_and_tables() {
        let d = deps();
        assert_eq!(find(&d, "syn").requirement, "2.0.1");
        assert_eq!(find(&d, "quote").requirement, "1");
    }

    #[test]
    fn recognises_source_kinds() {
        let d = deps();
        assert_eq!(find(&d, "local").source, Source::Path);
        assert_eq!(find(&d, "gitdep").source, Source::Git { pinned: false });
        assert_eq!(find(&d, "gitpinned").source, Source::Git { pinned: true });
        assert_eq!(find(&d, "shared").source, Source::Inherited);
        assert_eq!(find(&d, "syn").source, Source::Registry);
    }

    #[test]
    fn rejects_an_invalid_manifest() {
        assert!(parse_manifest("bad/Cargo.toml", "[dependencies\nx =").is_err());
    }

    #[test]
    fn classifies_version_requirements() {
        assert_eq!(classify_version("*"), Pin::Wildcard);
        assert_eq!(classify_version("1.*"), Pin::Wildcard);
        assert_eq!(classify_version(""), Pin::Wildcard);
        assert_eq!(classify_version("=1.2.3"), Pin::Exact);
        assert_eq!(classify_version("1.2.3"), Pin::Compatible);
        assert_eq!(classify_version("^1.2.3"), Pin::Compatible);
        assert_eq!(classify_version("~1.2.3"), Pin::Compatible);
        assert_eq!(classify_version("1"), Pin::Unpinned);
        assert_eq!(classify_version("1.2"), Pin::Unpinned);
        assert_eq!(classify_version(">=1.0"), Pin::Unpinned);
        assert_eq!(classify_version(">=1.0, <2.0"), Pin::Compatible);
    }

    fn allowed() -> Vec<String> {
        DEFAULT_ALLOWED_LICENSES.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn evaluates_license_expressions() {
        let a = allowed();
        assert!(license_allowed("MIT", &a));
        assert!(license_allowed("MIT OR Apache-2.0", &a));
        assert!(license_allowed("MIT/Apache-2.0", &a));
        assert!(license_allowed("GPL-3.0 OR MIT", &a));
        assert!(!license_allowed("GPL-3.0", &a));
        assert!(!license_allowed("MIT AND GPL-3.0", &a));
        assert!(license_allowed("Apache-2.0 WITH LLVM-exception", &a));
        assert!(!license_allowed("", &a));
        assert!(!license_allowed("NOASSERTION", &a));
    }

    #[test]
    fn license_check_is_case_insensitive_and_extendable() {
        let mut a = allowed();
        assert!(license_allowed("mit", &a));
        assert!(!license_allowed("MPL-2.0", &a));
        a.push("MPL-2.0".to_string());
        assert!(license_allowed("MPL-2.0", &a));
    }

    #[test]
    fn parses_cargo_metadata_licenses() {
        let json = r#"{"packages":[{"name":"a","version":"1.0.0","license":"MIT"},{"name":"b","version":"2.0.0","license":null}]}"#;
        let p = parse_metadata(json).unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].license.as_deref(), Some("MIT"));
        assert_eq!(p[1].license, None);
    }

    #[test]
    fn parses_cargo_audit_vulnerabilities() {
        let json = r#"{"vulnerabilities":{"found":true,"list":[{"advisory":{"id":"RUSTSEC-2024-0001","title":"bad"},"package":{"name":"a","version":"1.0.0"}}]}}"#;
        let v = parse_audit(json).unwrap();
        assert_eq!(v, vec![Vulnerability { package: "a".into(), version: "1.0.0".into(), id: "RUSTSEC-2024-0001".into(), title: "bad".into() }]);
        assert!(parse_audit(r#"{"vulnerabilities":{"found":false,"list":[]}}"#).unwrap().is_empty());
        assert!(parse_audit("not json").is_err());
    }

    fn report_for(deps: &[Dependency], vulns: Check<Vec<Vulnerability>>, pkgs: Check<Vec<PackageInfo>>) -> Vec<DependencyReport> {
        build_report(&Inputs { dependencies: deps, vulnerabilities: vulns, packages: pkgs, allowed_licenses: allowed() })
    }

    fn get<'a>(r: &'a [DependencyReport], name: &str) -> &'a DependencyReport {
        r.iter().find(|x| x.name == name).unwrap()
    }

    #[test]
    fn flags_wildcard_and_unpinned_dependencies() {
        let r = report_for(&deps(), Check::NotChecked, Check::NotChecked);
        assert!(get(&r, "wild").pin_issues[0].contains("wildcard"));
        assert!(get(&r, "open").pin_issues[0].contains("unpinned"));
        assert!(get(&r, "quote").pin_issues[0].contains("unpinned"));
        assert!(get(&r, "gitdep").pin_issues[0].contains("git dependency"));
        assert!(get(&r, "exact").pin_issues.is_empty());
        assert!(get(&r, "syn").pin_issues.is_empty());
        assert!(get(&r, "local").pin_issues.is_empty());
        assert!(get(&r, "shared").pin_issues.is_empty());
    }

    #[test]
    fn unchecked_data_never_fails_the_report() {
        let r = report_for(&deps(), Check::NotChecked, Check::NotChecked);
        assert!(r.iter().all(|d| !d.failing()));
    }

    #[test]
    fn reports_a_vulnerable_dependency() {
        let v = vec![Vulnerability { package: "syn".into(), version: "2.0.1".into(), id: "RUSTSEC-1".into(), title: "t".into() }];
        let r = report_for(&deps(), Check::Done(v), Check::NotChecked);
        assert!(get(&r, "syn").failing());
        assert!(get(&r, "syn").vulnerabilities[0].contains("RUSTSEC-1"));
        assert!(!get(&r, "quote").failing());
    }

    #[test]
    fn reports_license_problems_and_missing_licenses() {
        let pkgs = vec![
            PackageInfo { name: "syn".into(), version: "2.0.1".into(), license: Some("MIT OR Apache-2.0".into()) },
            PackageInfo { name: "quote".into(), version: "1.0.0".into(), license: Some("GPL-3.0".into()) },
            PackageInfo { name: "wild".into(), version: "1.0.0".into(), license: None },
        ];
        let r = report_for(&deps(), Check::NotChecked, Check::Done(pkgs));
        assert!(!get(&r, "syn").failing());
        assert!(get(&r, "quote").license_issues[0].contains("not allowed"));
        assert!(get(&r, "wild").license_issues[0].contains("no license"));
    }

    #[test]
    fn finds_duplicate_versions_across_manifests() {
        let a = parse_manifest("a/Cargo.toml", "[dependencies]\nsyn = \"2.0.1\"\n").unwrap();
        let b = parse_manifest("b/Cargo.toml", "[dependencies]\nsyn = \"1.0.109\"\n").unwrap();
        let all: Vec<Dependency> = a.into_iter().chain(b).collect();
        let r = report_for(&all, Check::NotChecked, Check::NotChecked);
        assert_eq!(get(&r, "syn").duplicate_versions, vec!["1.0.109".to_string(), "2.0.1".to_string()]);
        assert_eq!(get(&r, "syn").manifests.len(), 2);
    }

    #[test]
    fn the_same_requirement_in_two_manifests_is_not_a_duplicate() {
        let a = parse_manifest("a/Cargo.toml", "[dependencies]\nsyn = \"2.0.1\"\n").unwrap();
        let b = parse_manifest("b/Cargo.toml", "[dependencies]\nsyn = \"2.0.1\"\n").unwrap();
        let all: Vec<Dependency> = a.into_iter().chain(b).collect();
        assert!(get(&report_for(&all, Check::NotChecked, Check::NotChecked), "syn").duplicate_versions.is_empty());
    }

    #[test]
    fn renders_json_and_text() {
        let v = vec![Vulnerability { package: "syn".into(), version: "2.0.1".into(), id: "RUSTSEC-1".into(), title: "t".into() }];
        let r = report_for(&deps(), Check::Done(v), Check::NotChecked);
        let json = render_json(&r, true, false);
        assert!(json.starts_with("{\"passed\":false"));
        assert!(json.contains("\"licenses_checked\":false"));
        assert!(json.contains("vulnerable"));
        let text = render_text(&r, true, false);
        assert!(text.contains("NOT CHECKED: licenses"));
        assert!(text.contains("RUSTSEC-1"));
    }

    #[test]
    fn finds_manifests_recursively_and_skips_target() {
        let dir = std::env::temp_dir().join(format!("dep-audit-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("a/b")).unwrap();
        fs::create_dir_all(dir.join("target/debug")).unwrap();
        fs::write(dir.join("Cargo.toml"), "").unwrap();
        fs::write(dir.join("a/b/Cargo.toml"), "").unwrap();
        fs::write(dir.join("target/debug/Cargo.toml"), "").unwrap();
        let found = find_manifests(&dir);
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(found.len(), 2);
        assert!(found.iter().all(|p| !p.to_string_lossy().contains("target")));
    }
}
