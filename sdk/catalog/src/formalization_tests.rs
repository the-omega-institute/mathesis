//! Synthetic public API fixtures. None of these rows are live verification evidence.
use super::tests::{mock_server, temp};
use super::*;

fn fixture(bad_digest: bool) -> impl Fn(&str) -> (u16, Vec<u8>) + Send + Sync + 'static {
    let pins = formalization::pins().unwrap();
    let toolchain = pins["toolchain"].clone();
    let target = "import Mathlib\r\ndef wishpool_target_prop : Prop := True\r\n";
    let digest = sha256(target.as_bytes());
    let library = json!({"kind":"library_declarations","libraries":[{"name":"trureturing","revision":pins["lake_manifest"]["packages"].as_array().unwrap().iter().find(|p|p["name"]=="trureturing").unwrap()["rev"]}],"declarations":[{"library":"trureturing","module":"D5.Fixture","name":"fixture"}],"total":1,"complete":true});
    move |url| {
        let body = |v: Value| (200, serde_json::to_vec(&v).unwrap());
        let path = url.split('?').next().unwrap();
        let bundles:Vec<_>=(2..=4).map(|i|{
            let solution=if i==4 {"import Target\nimport Support.Helpers\ntheorem wishpool_disproof : ¬ wishpool_target_prop := by simp [wishpool_target_prop]\n"} else {"import Target\nimport Support.Helpers\ntheorem wishpool_solution : wishpool_target_prop := by trivial\n"};
            let modules:BTreeMap<String,String>=BTreeMap::from([("Support.Helpers".into(),"import Mathlib\n-- α Unicode source retained\nlemma helper : True := True.intro\n".into())]);
            let basis=if i==2 {library.clone()} else {json!({"kind":"new_lean"})};
            let receipt=json!({"verdict":if i==4 {"disproved"} else {"proved"},"target_digest":digest,"toolchain":toolchain,"solution_digest":sha256(&serde_json::to_vec(&("mathesis-project-v1",solution,&modules)).unwrap()),"checked_at":"2026-11-13T00:00:00Z","axioms":[],"reason":"synthetic fixture","duration":1});
            let attempt=json!({"id":format!("attempt{i}"),"receipt":receipt,"solution":solution,"modules":modules,"entrant":{"id":"fixture","name":"Fixture entrant"},"proof_basis":basis,"also_verified":false});
            json!({"schema_version":1,"record":"fixture-work","claim":format!("C{i}"),"version":1,"attempt":attempt,"verification_request":{"target":target,"target_digest":digest,"toolchain":toolchain,"solution":solution,"modules":modules},"instructions":"Synthetic test fixture"})
        }).collect();
        let rows:Vec<_>=(1..=5).map(|i|{
            let attempt=if (2..=4).contains(&i) {Some(&bundles[i-2]["attempt"])} else {None};
            json!({"record":"fixture-work","claim":format!("C{i}"),"version":1,"title":"Fixture","label":format!("Theorem {i}"),"statement":"Synthetic fixture","authors":[],"kind":"paper","status":if i==5 {"preparing"} else if i==1 {"open"} else if i==4 {"disproved"} else {"verified"},"target_digest":if i==5 {Value::Null} else {json!(digest)},"toolchain":if i==5 {Value::Null} else {toolchain.clone()},"receipt":attempt.map(|a|a["receipt"].clone()),"proof_basis":attempt.map(|a|a["proof_basis"].clone()),"verified_count":usize::from(attempt.is_some())})
        }).collect();
        match path {
            "/api/v1/collected-papers"=>return body(json!({"items":[{"id":"fixture-work","revision":"1","title":"Fixture","license":"CC-BY-4.0","authors":[]}],"next_before":null})),
            "/api/v1/papers"=>return body(json!({"items":[],"next_before":null})),
            "/api/v1/formalizations/opportunities"=>return body(json!({"items":rows,"next_before":null})),
            "/api/v1/collected-papers/fixture-work/proofs"=>return body(json!((1..=5).map(|i|json!({"claim":format!("C{i}"),"version":1,"state":rows[i-1]["status"],"target_digest":rows[i-1]["target_digest"],"next_step":"Exact public API reason","proofs":if (2..=4).contains(&i) {vec![bundles[i-2]["attempt"].clone()]} else {vec![]}})).collect::<Vec<_>>())),
            "/api/v1/research-problems"=>return body(json!({"items":[{"id":"curated-fixture"}],"next_before":null})),
            "/api/v1/research-problems/curated-fixture"=>return body(json!({"summary":{"id":"curated-fixture"},"curated":{"name":"Fixture question","version":1,"lean_target":{"state":"elaborated","text":target,"digest":digest,"toolchain":toolchain,"correspondence":"Synthetic fixture"}}})),
            "/api/v1/leaderboard" | "/api/v1/leaderboard/formalizations"=>return body(json!([{"rank":1,"entrant":{"name":"Fixture entrant"},"others":1,"total":2,"solved":2,"disproved":0,"last_solve":"2026-11-13T00:00:00Z"}])),
            _=>{}
        }
        for i in 1..=4 {
            let stem = format!("/api/v1/papers/fixture-work/statements/C{i}");
            if path == format!("{stem}/target") {
                return (200, target.as_bytes().to_vec());
            }
            if path == stem {
                return body(
                    json!({"target":{"lean":target,"digest":if bad_digest {"wrong"} else {&digest},"toolchain":toolchain}}),
                );
            }
            if path == format!("{stem}/attempts") {
                return body(if i == 1 {
                    json!([])
                } else {
                    json!([bundles[i - 2]["attempt"]])
                });
            }
            if i >= 2 && path == format!("{stem}/attempts/attempt{i}/bundle") {
                return body(bundles[i - 2].clone());
            }
        }
        (404, b"{}".to_vec())
    }
}

#[test]
fn public_sources_bundles_curated_and_boards_survive_offline_render() {
    let (base, stop, join) = mock_server(fixture(false));
    let input = temp("formalization-fixture");
    let output = temp("formalization-fixture-output");
    snapshot(&base, &input).unwrap();
    stop.send(()).unwrap();
    join.join().unwrap();
    // Server is stopped: rendering and all check commands must be offline.
    render(
        &input,
        &output,
        "mathesis-papers",
        "Mathesis",
        "https://site",
        None,
    )
    .unwrap();
    check(&output).unwrap();
    let root = output.join("formalization");
    let paper = read_json(&root.join("index/papers/fixture-work.json")).unwrap();
    for key in [
        "covered_by_existing_base",
        "newly_formalized",
        "open_target",
        "no_target_yet",
        "disproved",
    ] {
        assert_eq!(paper["split"][key].as_array().unwrap().len(), 1, "{key}");
    }
    assert_eq!(
        paper["split"]["no_target_yet"][0]["reason"],
        "Exact public API reason"
    );
    assert!(paper["trureturing_write_back"]["state"].is_null());
    assert_eq!(
        paper["split"]["covered_by_existing_base"][0]["proof_basis"]["declarations"][0]["name"],
        "fixture"
    );
    assert!(
        root.join("disproofs/fixture-work/v1/C4/attempt4/receipt.json")
            .exists()
    );
    assert_eq!(
        fs::read(root.join("targets/fixture-work/v1/C1/Target.lean")).unwrap(),
        b"import Mathlib\r\ndef wishpool_target_prop : Prop := True\r\n"
    );
    let request = read_json(&root.join("proofs/fixture-work/v1/C2/attempt2/bundle.json")).unwrap();
    assert_eq!(
        fs::read_to_string(root.join("proofs/fixture-work/v1/C2/attempt2/Support/Helpers.lean"))
            .unwrap(),
        request["verification_request"]["modules"]["Support.Helpers"]
    );
    assert_eq!(
        read_json(&root.join("leaderboard.json")).unwrap()["boards"]["formalizations"][0]["total"],
        2
    );
    assert_eq!(
        read_json(&root.join("index/curated.json")).unwrap()["targets"][0]["open_for_submission"],
        false
    );
    assert!(
        std::process::Command::new("python3")
            .arg(root.join("scripts/check.py"))
            .status()
            .unwrap()
            .success()
    );
    let before = fs::read(output.join(".mathesis/rendered.json")).unwrap();
    render(
        &input,
        &output,
        "mathesis-papers",
        "Mathesis",
        "https://site",
        None,
    )
    .unwrap();
    assert_eq!(
        fs::read(output.join(".mathesis/rendered.json")).unwrap(),
        before
    );
    let mut meta = read_json(&root.join("snapshot.json")).unwrap();
    meta["counts"]["open_target"] = json!(99);
    write_json(&root.join("snapshot.json"), &meta).unwrap();
    // Even if someone regenerates the outer hash manifest, semantic counts fail.
    let mut hashes = BTreeMap::new();
    collect_files(&output, &output, &mut hashes, false).unwrap();
    hashes.remove(".mathesis/rendered.json");
    write_json(
        &output.join(".mathesis/rendered.json"),
        &json!({"schema":"mathesis.rendered.v1","sha256":hashes}),
    )
    .unwrap();
    assert!(check(&output).is_err());
}

#[test]
fn exact_target_failure_preserves_previous_snapshot() {
    let (base, stop, join) = mock_server(fixture(true));
    let root = temp("formalization-mismatch");
    fs::write(root.join("marker"), "keep").unwrap();
    assert!(snapshot(&base, &root).is_err());
    assert_eq!(fs::read_to_string(root.join("marker")).unwrap(), "keep");
    stop.send(()).unwrap();
    join.join().unwrap();
}

#[test]
fn bundle_rejects_changed_sources_digests_and_unsafe_paths() {
    let handler = fixture(false);
    let (_, raw) = handler("/api/v1/papers/fixture-work/statements/C2/attempts/attempt2/bundle");
    let bundle: Value = serde_json::from_slice(&raw).unwrap();
    formalization::validate_bundle(&bundle).unwrap();
    for field in ["target", "solution"] {
        let mut changed = bundle.clone();
        changed["verification_request"][field] = json!("changed");
        assert!(formalization::validate_bundle(&changed).is_err());
    }
    let mut changed = bundle;
    changed["verification_request"]["modules"] = json!({"../escape":"text"});
    changed["attempt"]["modules"] = changed["verification_request"]["modules"].clone();
    let modules: BTreeMap<String, String> =
        serde_json::from_value(changed["verification_request"]["modules"].clone()).unwrap();
    changed["attempt"]["receipt"]["solution_digest"] = json!(sha256(
        &serde_json::to_vec(&(
            "mathesis-project-v1",
            changed["verification_request"]["solution"]
                .as_str()
                .unwrap(),
            modules
        ))
        .unwrap()
    ));
    assert!(formalization::validate_bundle(&changed).is_err());
}

#[test]
fn directory_winner_is_preserved_when_another_verdict_is_also_verified() {
    let (base, stop, join) = mock_server(fixture(false));
    let root = temp("formalization-mixed-verdict");
    snapshot(&base, &root).unwrap();
    stop.send(()).unwrap();
    join.join().unwrap();
    let mut snapshot = load_snapshot(&root).unwrap();
    // A collected proof list can report that some proof exists while the
    // directory's earliest winning receipt is a disproof. Keep that winner.
    snapshot.proofs[0].1[3]["state"] = json!("verified");
    let nodes = make_nodes(&snapshot).unwrap();
    let disproof = nodes.iter().find(|n| n.claim == "C4").unwrap();
    assert_eq!(disproof.status, "disproved");
    assert_eq!(disproof.receipt.as_ref().unwrap()["verdict"], "disproved");
}
