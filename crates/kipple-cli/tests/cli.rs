use assert_cmd::Command;
use kipple_core::testing::TestHome;
use predicates::prelude::*;

fn kipple(home: &TestHome) -> Command {
    let mut cmd = Command::cargo_bin("kipple").unwrap();
    cmd.env_clear().envs(home.env_pairs());
    cmd
}

#[test]
fn prints_version() {
    let home = TestHome::new().unwrap();
    kipple(&home)
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("kipple "));
}

#[test]
fn config_path_is_inside_sandbox() {
    let home = TestHome::new().unwrap();
    let out = kipple(&home).args(["config", "path"]).assert().success();
    let path = String::from_utf8(out.get_output().stdout.clone()).unwrap();
    assert!(path.trim().starts_with(home.path().to_str().unwrap()));
}

#[test]
fn config_validate_reports_unknown_keys() {
    let home = TestHome::new().unwrap();
    home.write(".config/kipple/config.toml", "[scan]\nrots = []\n")
        .unwrap();
    kipple(&home)
        .args(["config", "validate"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("did you mean `scan.roots`?"));
}

#[test]
fn config_show_json_has_schema_version() {
    let home = TestHome::new().unwrap();
    let out = kipple(&home)
        .args(["config", "show", "--json"])
        .assert()
        .success();
    let v: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["data"]["quarantine"]["keep_days"], 7);
}

mod clean {
    use super::*;
    use kipple_core::testing::Fixture;

    fn home(fixtures: &[&str]) -> TestHome {
        let mut home = TestHome::new().unwrap();
        for f in fixtures {
            Fixture::named(f).unwrap().generate(&mut home).unwrap();
        }
        kipple(&home).arg("scan").assert().success();
        home
    }

    #[test]
    fn needs_a_selection() {
        let home = home(&["rust-target"]);
        kipple(&home)
            .arg("clean")
            .assert()
            .code(2)
            .stderr(predicate::str::contains("Say what to clean"));
    }

    #[test]
    fn yes_requires_safe_only() {
        let home = home(&["rust-target"]);
        kipple(&home)
            .args(["clean", "--only", "rust-target", "--yes"])
            .assert()
            .code(2)
            .stderr(predicate::str::contains(
                "--yes only works together with --safe-only",
            ));
        assert!(home.join("code/tool/target").exists());
    }

    #[test]
    fn safe_only_yes_then_undo() {
        let home = home(&["rust-target", "maven-project"]);
        kipple(&home)
            .args(["clean", "--safe-only", "--yes"])
            .assert()
            .success()
            .stdout(predicate::str::contains("in quarantine for 7 days"));
        assert!(!home.join("code/tool/target").exists());
        assert!(!home.join("code/java-svc/target").exists());
        kipple(&home)
            .args(["quarantine", "list"])
            .assert()
            .success()
            .stdout(predicate::str::contains("2 items"));
        kipple(&home)
            .arg("history")
            .assert()
            .success()
            .stdout(predicate::str::contains("quarantined"));
        kipple(&home).arg("undo").assert().success();
        assert!(home.join("code/tool/target/debug/tool").exists());
        assert!(home.join("code/java-svc/target/classes/App.class").exists());
    }

    #[test]
    fn declining_changes_nothing() {
        let home = home(&["rust-target"]);
        kipple(&home)
            .args(["clean", "--safe-only"])
            .write_stdin("n\n")
            .assert()
            .code(4)
            .stderr(predicate::str::contains("Nothing was changed"));
        assert!(home.join("code/tool/target").exists());
    }

    #[test]
    fn review_items_need_reclaim() {
        let home = home(&["xcode-project"]);
        // Pods is a review item.
        kipple(&home)
            .args(["clean", "--only", "xcode-project"])
            .write_stdin("y\n")
            .assert()
            .code(4);
        assert!(home.join("code/ios-app/Pods").exists());
        kipple(&home)
            .args(["clean", "--only", "xcode-project"])
            .write_stdin("reclaim\n")
            .assert()
            .success();
        assert!(!home.join("code/ios-app/Pods").exists());
    }

    #[test]
    fn plan_only_then_run_saved_plan() {
        let home = home(&["rust-target"]);
        let out = kipple(&home)
            .args(["clean", "--safe-only", "--plan-only", "--json"])
            .assert()
            .success();
        let v: serde_json::Value = serde_json::from_slice(&out.get_output().stdout).unwrap();
        let id = v["data"]["id"].as_str().unwrap().to_owned();
        assert!(home.join("code/tool/target").exists());
        kipple(&home)
            .args(["clean", "--plan", &id, "--safe-only", "--yes", "--json"])
            .assert()
            .success();
        assert!(!home.join("code/tool/target").exists());
    }

    #[test]
    fn purge_now_frees_immediately() {
        let home = home(&["rust-target"]);
        kipple(&home)
            .args(["clean", "--safe-only", "--purge-now"])
            .write_stdin("y\npurge\n")
            .assert()
            .success()
            .stdout(predicate::str::contains("Freed"));
        kipple(&home)
            .args(["quarantine", "list"])
            .assert()
            .stdout(predicate::str::contains("Quarantine is empty"));
    }
}

#[test]
fn no_arguments_outside_a_terminal_prints_help() {
    let home = TestHome::new().unwrap();
    kipple(&home)
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: kipple"));
}

#[test]
fn share_writes_a_png() {
    let mut home = TestHome::new().unwrap();
    kipple_core::testing::Fixture::named("rust-target")
        .unwrap()
        .generate(&mut home)
        .unwrap();
    kipple(&home).arg("scan").assert().success();
    let out = home.join("card.png");
    kipple(&home)
        .args(["share", "--out", out.to_str().unwrap()])
        .assert()
        .success();
    assert_eq!(&std::fs::read(out).unwrap()[1..4], b"PNG");
}

/// A development build without a test sandbox never changes anything, even
/// when the user confirms (the user's standing rule: no real data, ever).
#[test]
fn debug_build_refuses_real_data_without_sandbox() {
    let mut home = TestHome::new().unwrap();
    kipple_core::testing::Fixture::named("rust-target")
        .unwrap()
        .generate(&mut home)
        .unwrap();
    home.env_mut().remove("KIPPLE_TEST_SANDBOX");
    kipple(&home).arg("scan").assert().success();
    kipple(&home)
        .args(["clean", "--safe-only", "--yes"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "only change files inside a test sandbox",
        ));
    assert!(home.join("code/tool/target/debug/tool").exists());
    assert!(
        !home.join(".local/share/kipple/quarantine").exists()
            || std::fs::read_dir(home.join(".local/share/kipple/quarantine"))
                .unwrap()
                .next()
                .is_none()
    );
}
