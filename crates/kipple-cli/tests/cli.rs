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
