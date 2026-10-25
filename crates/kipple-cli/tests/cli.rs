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
