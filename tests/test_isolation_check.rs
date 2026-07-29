/// Enforces that no test in the workspace carries a test-isolation #[ignore].
///
/// Run with: cargo test --test test_isolation_check
#[test]
fn no_ignore_test_isolation_annotations_remain() {
    use std::fs;
    use std::path::Path;

    fn walk(dir: &Path, violations: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // skip target directory
                if path.file_name().map_or(false, |n| n == "target") {
                    continue;
                }
                walk(&path, violations);
            } else if path.extension().map_or(false, |e| e == "rs") {
                let Ok(src) = fs::read_to_string(&path) else { continue };
                for (i, line) in src.lines().enumerate() {
                    if line.contains("#[ignore") && line.contains("isolation") {
                        violations.push(format!(
                            "{}:{}: {}",
                            path.display(),
                            i + 1,
                            line.trim()
                        ));
                    }
                }
            }
        }
    }

    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let mut violations = Vec::new();
    walk(workspace_root, &mut violations);

    assert!(
        violations.is_empty(),
        "Found test-isolation #[ignore] annotations that must be removed:\n{}",
        violations.join("\n")
    );
}
