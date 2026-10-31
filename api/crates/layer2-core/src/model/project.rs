//! Compatibility diagnostics; acceptance of a source archive is never a proof receipt.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeanPreflight {
    pub modules: usize,
    pub source_bytes: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub dependencies: Vec<String>,
    pub requires_adapter: bool,
}

/// Ignore inert comments only when locating hash commands in supporting modules.
/// Strings remain visible; nested Lean block comments and line comments are handled.
fn has_hash_command(text: &str) -> bool {
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut depth = 0usize;
    let mut string = false;
    while i < bytes.len() {
        if depth > 0 {
            if bytes[i..].starts_with(b"/-") {
                depth += 1;
                i += 2;
            } else if bytes[i..].starts_with(b"-/") {
                depth -= 1;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if string {
            if bytes[i] == b'#' {
                return true;
            }
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                string = false;
            }
        } else if bytes[i..].starts_with(b"--") {
            i += bytes[i..]
                .iter()
                .position(|b| *b == b'\n')
                .unwrap_or(bytes.len() - i);
            continue;
        } else if bytes[i..].starts_with(b"/-") {
            depth = 1;
            i += 2;
            continue;
        } else if bytes[i] == b'"' {
            string = true;
        } else if bytes[i] == b'#' {
            return true;
        }
        i += 1;
    }
    false
}

pub fn lean_project_preflight(
    solution: &str,
    modules: &BTreeMap<String, String>,
    files: &BTreeMap<String, String>,
    toolchain: &str,
) -> LeanPreflight {
    let mut report = LeanPreflight {
        modules: modules.len(),
        source_bytes: solution.len() + modules.values().map(String::len).sum::<usize>(),
        errors: vec![],
        warnings: vec![],
        dependencies: vec![],
        requires_adapter: !matches!(
            (
                solution.matches("theorem wishpool_solution").count(),
                solution.matches("theorem wishpool_disproof").count()
            ),
            (1, 0) | (0, 1)
        ) || !solution.lines().any(|line| line.trim() == "import Target"),
    };
    if let Err(error) = super::validate_lean_modules(solution, modules) {
        report.errors.push(error.to_string());
    }
    if report.requires_adapter {
        report.errors.push("Provide a Solution.lean adapter importing Target and declaring wishpool_solution or wishpool_disproof before requesting verification.".into());
    }
    if let Some(pin) = files.get("lean-toolchain") {
        if pin.trim() != toolchain.split(',').next().unwrap_or_default().trim() {
            report
                .errors
                .push("lean-toolchain disagrees with the requested target/toolchain pin".into());
        }
        if !pin.trim().starts_with("leanprover/lean4:v") || pin.contains("nightly") {
            report.warnings.push("Use an exact supported Lean release; compatibility is checked again by the verifier.".into());
        }
    } else if !modules.is_empty() {
        report.warnings.push(
            "No lean-toolchain file supplied; only the reported toolchain pin is available.".into(),
        );
    }
    if let Some(manifest) = files.get("lake-manifest.json") {
        match serde_json::from_str::<serde_json::Value>(manifest) {
            Ok(value) if value["packages"].is_array() => {
                for package in value["packages"].as_array().unwrap() {
                    let name = package["name"].as_str().unwrap_or("<unnamed>");
                    let rev = package["rev"].as_str().unwrap_or_default();
                    report.dependencies.push(format!("{name}@{rev}"));
                    if ![40, 64].contains(&rev.len()) || !rev.bytes().all(|b| b.is_ascii_hexdigit())
                    {
                        report.errors.push(format!("Dependency {name} needs a full Git revision; path and floating dependencies are not reproducible."));
                    }
                    if name == "mathlib" {
                        if let Some((_, target_rev)) = toolchain.split_once(", Mathlib ") {
                            if target_rev != rev {
                                report.errors.push(
                                    "Mathlib revision does not match the confirmed target".into(),
                                );
                            }
                        } else {
                            report.errors.push("Include the manifest Mathlib revision in the reported toolchain pin (Lean pin, Mathlib <revision>).".into());
                        }
                    }
                }
            }
            _ => report
                .errors
                .push("lake-manifest.json must contain a valid packages array".into()),
        }
    } else if files.contains_key("lakefile.lean") || files.contains_key("lakefile.toml") {
        report
            .errors
            .push("Missing lake-manifest.json: dependency revisions cannot be checked.".into());
    }
    if files.contains_key("lakefile.lean") || files.contains_key("lakefile.toml") {
        report.warnings.push("Lake source roots and build options are preserved as data. Supply module paths relative to the Lean import root; custom build steps are not executed. Non-Mathlib dependency hashes are checked for fixed revisions, but only the prepared workspace supplies their actual versions.".into());
    }
    let mut graph = BTreeMap::<String, BTreeSet<String>>::new();
    for (name, text) in modules
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .chain([("Solution", solution)])
    {
        let mut deps = BTreeSet::new();
        if name != "Solution"
            && [
                "wishpool_solution",
                "wishpool_disproof",
                "wishpool_target_prop",
            ]
            .iter()
            .any(|reserved| text.contains(reserved))
        {
            report.errors.push(format!("{name}: supporting modules cannot declare or shadow the reserved target/solution names."));
        }
        for line in text.lines() {
            let words: Vec<_> = line.split_whitespace().collect();
            if line.contains("import") && (words.len() != 2 || words.first() != Some(&"import")) {
                report.errors.push(format!("{name}: imports must use one module per line with no inline comments in the checker profile."));
            }
            if words.first() != Some(&"import") {
                continue;
            }
            for dep in words.iter().skip(1).take_while(|w| !w.starts_with("--")) {
                if modules.contains_key(*dep) {
                    deps.insert((*dep).to_owned());
                } else if !["Mathlib", "Target"].contains(dep) && !dep.starts_with("Mathlib.") {
                    report.errors.push(format!("{name}: missing import {dep}; include its source module. Custom dependencies are not fetched."));
                }
            }
        }
        // Match the checker's conservative profile, including comments. Namespace blocks
        // are accepted in library modules, but the root target adapter stays unambiguous.
        if let Some(token) = text
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .find(|token| {
                [
                    "axiom",
                    "constant",
                    "opaque",
                    "sorry",
                    "sorryAx",
                    "admit",
                    "implemented_by",
                    "extern",
                    "unsafe",
                    "macro",
                    "macro_rules",
                    "notation",
                    "syntax",
                    "elab",
                    "elab_rules",
                    "initialize",
                    "init",
                    "builtin_init",
                    "builtin_initialize",
                    "set_option",
                    "run_cmd",
                    "run_tac",
                    "run_elab",
                    "run_meta",
                    "attribute",
                    "declare_syntax_cat",
                    "export",
                ]
                .contains(token)
                    || token.contains("skipKernelTC")
                    || (name == "Solution" && ["namespace", "section", "end"].contains(token))
            })
        {
            report.errors.push(format!("{name}: construct {token} is outside the independent checker's restricted profile (comments are scanned too). Source can still be deposited."));
        }
        if (if name == "Solution" {
            text.contains('#')
        } else {
            has_hash_command(text)
        }) || text.contains('«')
            || text.contains('»')
            || text.contains('\u{202e}')
        {
            report.errors.push(format!(
                "{name}: commands or quoted identifiers are outside the checker profile."
            ));
        }
        graph.insert(name.to_owned(), deps);
    }
    while let Some(name) = graph
        .iter()
        .find(|(_, deps)| deps.is_empty())
        .map(|(n, _)| n.clone())
    {
        graph.remove(&name);
        for deps in graph.values_mut() {
            deps.remove(&name);
        }
    }
    if !graph.is_empty() {
        report.errors.push(format!(
            "Cyclic imports: {}",
            graph.keys().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    report.warnings.push("Static preflight only. The independent verifier resolves imports in its prepared pinned workspace, checks the exact target and allowed axioms. Lake scripts are preserved as data and never executed; no dependencies are downloaded.".into());
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preflight_accepts_issue_links_in_library_comments_but_not_commands() {
        let solution = "import Target\nimport Research.Helper\ntheorem wishpool_solution : wishpool_target_prop := by trivial\n";
        let module = "/- Historical (#15580; Refuted). /- Nested #42. -/ -/\n-- Source #123\ntheorem lemma : True := True.intro\n";
        let modules = BTreeMap::from([("Research.Helper".into(), module.into())]);
        let report = lean_project_preflight(solution, &modules, &BTreeMap::new(), "test");
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        for suffix in [
            "#check True",
            "def text := \"/-\"\n#check True",
            "-- comment\n#check True",
        ] {
            let modules =
                BTreeMap::from([("Research.Helper".into(), format!("{module}\n{suffix}"))]);
            let report = lean_project_preflight(solution, &modules, &BTreeMap::new(), "test");
            assert!(
                report.errors.iter().any(|error| error.contains("commands")),
                "{suffix}"
            );
        }
        let report = lean_project_preflight(
            &format!("-- #123\n{solution}"),
            &modules,
            &BTreeMap::new(),
            "test",
        );
        assert!(report.errors.iter().any(|error| error.contains("commands")));
    }

    #[test]
    fn project_preflight_finds_missing_imports_versions_cycles_and_adapter() {
        let modules = BTreeMap::from([
            (
                "Research.Main".into(),
                "import Research.Helper\nimport Missing.Certificate".into(),
            ),
            ("Research.Helper".into(), "import Research.Main".into()),
        ]);
        let files = BTreeMap::from([
            ("lean-toolchain".into(), "leanprover/lean4:v4.32.0".into()),
            (
                "lake-manifest.json".into(),
                r#"{"packages":[{"name":"mathlib","rev":"main"}]}"#.into(),
            ),
        ]);
        let report = lean_project_preflight(
            "",
            &modules,
            &files,
            "leanprover/lean4:v4.33.0, Mathlib abc",
        );
        assert!(report.requires_adapter);
        for expected in [
            "Missing.Certificate",
            "Cyclic",
            "lean-toolchain",
            "full Git",
            "Mathlib revision",
        ] {
            assert!(
                report.errors.iter().any(|e| e.contains(expected)),
                "{expected}: {:?}",
                report.errors
            );
        }
    }
    #[test]
    fn large_nested_project_is_accepted_but_traversal_and_shadowing_are_not() {
        let modules: BTreeMap<_, _> = (0..80)
            .map(|i| {
                (
                    format!("Research.Part{i}.Lemma"),
                    "theorem lemma : True := True.intro".into(),
                )
            })
            .collect();
        assert!(super::super::validate_lean_modules("", &modules).is_ok());
        for name in [
            "../Escape",
            "A..B",
            "Mathlib.User",
            "Research/Part",
            "target.X",
        ] {
            assert!(
                super::super::validate_lean_modules(
                    "",
                    &BTreeMap::from([(name.into(), "source".into())])
                )
                .is_err(),
                "{name}"
            );
        }
    }
}
