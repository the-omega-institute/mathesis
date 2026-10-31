use super::*;
#[test]
fn rejects_all_escape_hatches_and_wrong_module_formats() {
    let good = "import Target\ntheorem wishpool_solution : wishpool_target_prop := by trivial\n";
    assert_eq!(validate_solution(good), Ok(Verdict::Proved));
    assert_eq!(
        validate_solution(
            "import Mathlib\nimport Target\nlemma helper : True := by trivial\ntheorem wishpool_disproof : ¬ wishpool_target_prop := by simp [wishpool_target_prop]\n"
        ),
        Ok(Verdict::Disproved)
    );
    for construct in [
        "axiom magic : False",
        "constant magic : False",
        "opaque magic : False := by sorry",
        "sorry",
        "admit",
        "unsafe def",
        "@[implemented_by evil]",
        "@[extern evil]",
        "macro x",
        "notation x",
        "syntax x",
        "elab x",
        "initialize x",
        "@[init boot] def value := 1",
        "@[builtin_init boot] def value := 1",
        "run_meta x",
        "_root_.sorryAx",
        "set_option debug.skipKernelTC true",
        "run_cmd x",
        "#eval x",
        "run_tac x",
        "import Evil",
        "import Mathlib Target",
        "namespace Fake",
    ] {
        assert!(
            validate_solution(&format!("{construct}\n{good}")).is_err(),
            "{construct}"
        );
    }
    assert!(validate_solution(&good.replace("import Target", "import Mathlib")).is_err());
    assert!(
        validate_solution(&format!(
            "{good}\ntheorem wishpool_disproof : False := by contradiction"
        ))
        .is_err()
    );
    assert!(validate_solution(&good.repeat(2)).is_err());
}
#[test]
fn target_conversion_is_exact_and_binders_require_regeneration() {
    let legacy = "import Mathlib\ndef bound (n : Nat) := n\ntheorem wishpool_target : ∀ n : Nat, bound n = n := by sorry\n";
    let converted = convert_target(legacy).unwrap();
    assert_eq!(
        converted,
        "import Mathlib\ndef bound (n : Nat) := n\ndef wishpool_target_prop : Prop := ∀ n : Nat, bound n = n\n"
    );
    assert_eq!(convert_target(&converted), Ok(converted.clone()));
    assert!(validate_target(&converted).is_ok());
    for bad in [
        "theorem wishpool_target (n : Nat) : n = n := by sorry",
        "axiom bad : False\ntheorem wishpool_target : True := by sorry",
        "theorem wishpool_target : True := by sorry\n#eval 1",
    ] {
        assert!(convert_target(bad).is_err());
    }
}
#[tokio::test]
async fn validates_digest_before_any_workspace_or_process_access() {
    let checker = Checker {
        workspace: "/nonexistent".into(),
        scratch: std::env::temp_dir(),
        timeout: Duration::from_secs(1),
    };
    let request = Request {
        modules: Default::default(),
        target: "import Mathlib\ndef wishpool_target_prop : Prop := True\n".into(),
        target_digest: "wrong".into(),
        toolchain: "test".into(),
        solution: "import Target\ntheorem wishpool_solution : wishpool_target_prop := by trivial"
            .into(),
    };
    let receipt = checker.verify(&request).await;
    assert_eq!(receipt.verdict, Verdict::Rejected);
    assert_eq!(receipt.reason, "target digest mismatch");
    assert_eq!(receipt.solution_digest, digest(&request.solution));
}
#[cfg(unix)]
#[tokio::test]
async fn executable_fakes_enforce_exit_status_output_limit_and_axiom_receipts() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fake-lean");
    for (body, ok) in [
        (
            "echo \"'wishpool_solution' depends on axioms: [propext, Classical.choice, Quot.sound]\"",
            true,
        ),
        ("echo 'error: wrong target type'", false),
        ("exit 1", false),
        ("head -c 70000 /dev/zero", false),
    ] {
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let out = capture(command(&path, dir.path())).await;
        assert_eq!(out.is_ok(), ok);
        if let Ok(text) = out {
            assert_eq!(axioms_of(&text, "wishpool_solution").unwrap().len(), 3);
        }
    }
    assert_eq!(
        axioms_of(
            "'wishpool_solution' does not depend on any axioms",
            "wishpool_solution"
        ),
        Some(vec![])
    );
    assert_eq!(
        axioms_of("'wrong' depends on axioms: [sorryAx]", "wishpool_solution"),
        None
    );
}
#[tokio::test]
#[ignore = "requires WISHPOOL_TEST_LEAN_WORKSPACE with pinned Lean and built Mathlib"]
async fn real_lean_target_proof_disproof_wrong_type_and_axiom() {
    let workspace = std::env::var("WISHPOOL_TEST_LEAN_WORKSPACE")
        .expect("real Lean tests require WISHPOOL_TEST_LEAN_WORKSPACE");
    let dir = tempfile::tempdir().unwrap();
    let checker = Checker {
        workspace: workspace.into(),
        scratch: dir.path().into(),
        timeout: Duration::from_secs(600),
    };
    let toolchain = std::fs::read_to_string(checker.workspace.join("lean-toolchain"))
        .unwrap()
        .trim()
        .to_owned();
    for (prop, solution, expected) in [
        (
            "True",
            "theorem wishpool_solution : wishpool_target_prop := by trivial",
            Verdict::Proved,
        ),
        (
            "False",
            "theorem wishpool_disproof : ¬ wishpool_target_prop := by simp [wishpool_target_prop]",
            Verdict::Disproved,
        ),
        (
            "False",
            "theorem wishpool_solution : True := by trivial",
            Verdict::Rejected,
        ),
        (
            "False",
            "axiom magic : False\ntheorem wishpool_solution : wishpool_target_prop := magic",
            Verdict::Rejected,
        ),
    ] {
        let target = format!("import Mathlib\ndef wishpool_target_prop : Prop := {prop}\n");
        let request = Request {
            modules: Default::default(),
            target_digest: digest(&target),
            target,
            toolchain: toolchain.clone(),
            solution: format!("import Target\n{solution}\n"),
        };
        let receipt = checker.verify(&request).await;
        assert_eq!(receipt.verdict, expected, "{}", receipt.reason);
    }
}

#[cfg(unix)]
#[tokio::test]
async fn full_module_pipeline_with_executable_fake_checks_names_types_and_receipt() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("workspace");
    std::fs::create_dir_all(workspace.join(".lake/packages")).unwrap();
    std::fs::write(workspace.join("lean-toolchain"), "fake").unwrap();
    std::fs::write(
        workspace.join("lake-manifest.json"),
        r#"{"packages":[{"name":"mathlib","rev":"0123456789"}]}"#,
    )
    .unwrap();
    let binary = dir.path().join("fake-lean");
    std::fs::write(
        &binary,
        r##"#!/usr/bin/python3
import os, sys, json
name = sys.argv[-1]
assert sys.argv[1:3] == ['-j','2']
assert os.path.realpath(os.environ['HOME']) == os.path.realpath(os.getcwd())
setup = json.load(open(sys.argv[sys.argv.index('--setup')+1]))
if name == 'Solution.lean':
    assert 'Target' in setup['importArts']
    assert len(setup['importArts']['Target'][0]) == 3
    assert len(setup['importArts']['Target'][1]) == 2
    assert not any('TOKEN' in key or 'SECRET' in key for key in os.environ)
if name == 'Check.lean':
    assert 'Solution' in setup['importArts']
    text = open(name).read()
    solution = open('Solution.lean').read()
    assert '#check (_root_.wishpool_' in text and '_root_.wishpool_target_prop)' in text
    if 'wrong_type' in solution:
        print('error: type mismatch'); sys.exit(1)
    theorem = 'wishpool_disproof' if 'wishpool_disproof' in solution else 'wishpool_solution'
    axioms = 'customAxiom' if 'extra_axiom' in solution else 'propext, Classical.choice, Quot.sound'
    print("'%s' depends on axioms: [%s]" % (theorem, axioms))
output = sys.argv[sys.argv.index('-o')+1]
for suffix in ['', '.server', '.private']:
    open(output + suffix, 'w').close()
for suffix in ['ir.sig', 'ir']:
    open(output.removesuffix('olean') + suffix, 'w').close()
"##,
    )
    .unwrap();
    std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o700)).unwrap();
    let checker = Checker {
        workspace: workspace.clone(),
        scratch: dir.path().into(),
        timeout: Duration::from_secs(5),
    };
    for (declaration, expected) in [
        (
            "theorem wishpool_solution : wishpool_target_prop := by trivial",
            Some(Verdict::Proved),
        ),
        (
            "theorem wishpool_disproof : ¬ wishpool_target_prop := by simp [wishpool_target_prop]",
            Some(Verdict::Disproved),
        ),
        (
            "theorem wishpool_solution : True := by trivial -- wrong_type",
            None,
        ),
        (
            "theorem wishpool_solution : wishpool_target_prop := by trivial -- extra_axiom",
            None,
        ),
    ] {
        let target = "import Mathlib\ndef wishpool_target_prop : Prop := True\n".to_string();
        let request = Request {
            modules: Default::default(),
            target_digest: digest(&target),
            target,
            toolchain: "fake".into(),
            solution: format!("import Target\n{declaration}\n"),
        };
        let result = checker.check_using(&request, Some(&binary)).await;
        assert_eq!(result.as_ref().ok().map(|r| r.0), expected, "{result:?}");
    }
    assert!(!workspace.join("Target.lean").exists());
    assert!(!workspace.join("Solution.lean").exists());
}

#[test]
fn projects_validate_every_module_and_bind_the_entire_dependency_graph() {
    let solution =
        "import Target\nimport Result\ntheorem wishpool_solution : wishpool_target_prop := helper";
    let modules = BTreeMap::from([
        (
            "Base".into(),
            "import Mathlib\ntheorem base : True := True.intro".into(),
        ),
        (
            "Result".into(),
            "import Base\ntheorem helper : True := base".into(),
        ),
    ]);
    let (verdict, order) = validate_project(solution, &modules).unwrap();
    assert_eq!(verdict, Verdict::Proved);
    assert_eq!(order, vec!["Base", "Result"]);
    assert_eq!(
        solution_digest("single", &BTreeMap::new()),
        digest("single")
    );
    let hash = solution_digest(solution, &modules);
    let mut changed = modules.clone();
    changed.get_mut("Base").unwrap().push_str("\n-- change");
    assert_ne!(hash, solution_digest(solution, &changed));
    for (name, text) in [
        ("../Escape", "import Mathlib"),
        ("Target", "import Mathlib"),
        ("Mathlib", "import Mathlib"),
        ("Extra", "axiom bad : False"),
        ("Extra", "import IO"),
        ("Extra", "import Base Result"),
        ("Extra", "#eval 1"),
        ("Extra", "theorem wishpool_solution : True := True.intro"),
        ("Extra", "import Extra"),
        ("base", "import Mathlib"),
    ] {
        let mut bad = modules.clone();
        bad.insert(name.into(), text.into());
        assert!(validate_project(solution, &bad).is_err(), "{name}: {text}");
    }
    let mut cyclic = modules.clone();
    cyclic.insert("Base".into(), "import Result".into());
    assert!(
        validate_project(solution, &cyclic)
            .unwrap_err()
            .contains("cyclic")
    );
    let mut shadow = modules.clone();
    shadow.insert("BASE".into(), "import Mathlib".into());
    assert!(validate_project(solution, &shadow).is_err());
}

#[tokio::test]
#[ignore = "requires WISHPOOL_TEST_LEAN_WORKSPACE with pinned Lean and built Mathlib"]
async fn real_lean_multimodule_proof_checks_helpers_and_wrong_type() {
    let workspace = std::env::var("WISHPOOL_TEST_LEAN_WORKSPACE")
        .expect("real Lean tests require WISHPOOL_TEST_LEAN_WORKSPACE");
    let dir = tempfile::tempdir().unwrap();
    let checker = Checker {
        workspace: workspace.into(),
        scratch: dir.path().into(),
        timeout: Duration::from_secs(600),
    };
    let toolchain = std::fs::read_to_string(checker.workspace.join("lean-toolchain"))
        .unwrap()
        .trim()
        .to_owned();
    for (prop, expected) in [("True", Verdict::Proved), ("False", Verdict::Rejected)] {
        let target = format!("import Mathlib\ndef wishpool_target_prop : Prop := {prop}\n");
        let request = Request {
            target_digest: digest(&target), target, toolchain: toolchain.clone(),
            solution: "import Target\nimport Research.Helpers\ntheorem wishpool_solution : wishpool_target_prop := Research.helper\n".into(),
            modules: BTreeMap::from([("Research.Helpers".into(), "import Mathlib.Data.Nat.Basic\n/- Source issue #15580. /- Nested #42. -/ -/\nnamespace Research\ntheorem helper : True := True.intro\nend Research\n".into())]),
        };
        let receipt = checker.verify(&request).await;
        assert_eq!(receipt.verdict, expected, "{}", receipt.reason);
        assert_eq!(
            receipt.solution_digest,
            solution_digest(&request.solution, &request.modules)
        );
    }
}

#[test]
fn nested_modules_support_namespaces_but_cannot_shadow_root_targets() {
    let source = "import Target\nimport Research.Helpers\ntheorem wishpool_solution : wishpool_target_prop := Research.helper";
    let mut modules = BTreeMap::from([("Research.Helpers".into(), "import Mathlib.Data.Nat.Basic\nnamespace Research\ntheorem helper : True := True.intro\nend Research".into())]);
    assert!(validate_project(source, &modules).is_ok());
    modules.insert(
        "Research.Unused".into(),
        "namespace Hidden\naxiom unsound : False\nend Hidden".into(),
    );
    assert!(validate_project(source, &modules).is_err());
    modules.remove("Research.Unused");
    modules.insert(
        "Target.Fake".into(),
        "theorem helper : True := True.intro".into(),
    );
    assert!(validate_project(source, &modules).is_err());
}

#[cfg(unix)]
#[test]
fn nested_outputs_cannot_follow_prepared_workspace_links() {
    let work = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let victim = outside.path().join("Proof.lean");
    std::fs::write(&victim, "original source").unwrap();
    std::os::unix::fs::symlink(outside.path(), work.path().join("Linked")).unwrap();
    assert!(prepare_module_outputs(work.path(), &["Linked.Proof"]).is_err());
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), "original source");
    assert!(ensure_module_parent(&work.path().join("Linked"), "Proof").is_err());
    std::os::unix::fs::symlink(&victim, work.path().join("Research.Part.Proof.setup.json"))
        .unwrap();
    prepare_module_outputs(work.path(), &["Research.Part.Proof"]).unwrap();
    assert!(!work.path().join("Research.Part.Proof.setup.json").exists());
    assert!(work.path().join("Research/Part").is_dir());
    std::fs::write(work.path().join("Research/Part/Proof.lean"), "new source").unwrap();
    assert_eq!(std::fs::read_to_string(&victim).unwrap(), "original source");
}

#[test]
fn supporting_module_issue_comments_are_inert_but_hash_commands_remain_forbidden() {
    for source in [
        "def quote : Char := '\"'\ndef text := \"/-\"\n#check True",
        "def text := s!\"{\"/-\"}\"\n#check True",
    ] {
        assert!(forbidden_in(source, true).is_some());
    }
    let module = "/- Historical (#15580; Refuted). /- Nested #42. -/ -/\n-- Source #123\ntheorem lemma : True := True.intro\n";
    assert!(forbidden_in(module, true).is_none());
    assert!(
        forbidden(module).is_some(),
        "root Target/Solution profiles stay strict"
    );
    for suffix in [
        "#check True",
        "def text := \"/-\"\n#check True",
        "def text := \"--\"\n#check True",
        "/- outer /- inner -/ -/ #check True",
        "-- source comment\n#check True",
    ] {
        assert!(
            forbidden_in(&format!("{module}\n{suffix}"), true).is_some(),
            "{suffix}"
        );
    }
    assert!(forbidden_in("/- sorry -/", true).is_some());
}
