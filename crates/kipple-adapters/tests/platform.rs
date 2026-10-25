//! Golden and action tests for the platform adapters (M5).

use std::path::Path;

use kipple_core::model::*;
use kipple_core::testing::{Fixture, TestHome};
use kipple_core::{Kipple, OpenOptions};
use serde_json::{Value, json};

fn rel(home: &TestHome, p: &Path) -> String {
    p.strip_prefix(home.path())
        .map(|r| format!("~/{}", r.display()))
        .unwrap_or_else(|_| p.display().to_string())
        .replace('\\', "/")
}

fn project(home: &TestHome, item: &Item) -> Value {
    let locator = match &item.locator {
        Locator::Docker { kind, id } => json!(format!("docker:{kind}:{id}")),
        Locator::Logical { store, key } => json!(format!("{store}:{key}")),
        other => json!(
            other
                .paths()
                .iter()
                .map(|p| rel(home, p))
                .collect::<Vec<_>>()
        ),
    };
    json!({
        "integration": item.integration,
        "kind": item.kind,
        "title": item.title,
        "locator": locator,
        "project": item.project.as_ref().map(|p| p.name.clone()),
        "safety": item.safety,
        "reasons": item.reasons.iter().map(|r| r.code).collect::<Vec<_>>(),
        "actions": item.actions.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "apparent": item.size.apparent,
        "reclaimable_is_less": item.size.reclaimable < item.size.on_disk,
        "notes": item.notes.len(),
        "restore_hint": item.restore_hint,
    })
}

fn open_with(home: &TestHome, procs: kipple_core::platform::FixedProcesses) -> Kipple {
    let mut integrations = kipple_packs::builtin().unwrap();
    integrations.extend(kipple_adapters::all());
    let mut opts = OpenOptions::new(home.env().clone(), integrations);
    opts.processes = Some(std::sync::Arc::new(procs));
    Kipple::open(opts).unwrap()
}

fn open(home: &TestHome) -> Kipple {
    open_with(home, Default::default())
}

fn only(id: &str) -> ScanRequest {
    ScanRequest {
        only: vec![id.to_owned()],
        ..Default::default()
    }
}

fn scan(home: &mut TestHome, fixture: &str, id: &str) -> (Kipple, Vec<Item>) {
    Fixture::named(fixture).unwrap().generate(home).unwrap();
    let k = open(home);
    k.scan(only(id), &NoopSink::default()).unwrap();
    let (_, mut items) = k.all_items().unwrap();
    items.sort_by_key(|i| i.locator.canonical_key());
    (k, items)
}

fn snapshot(id: &str) -> Value {
    let mut home = TestHome::new().unwrap();
    let (_, items) = scan(&mut home, id, id);
    Value::Array(items.iter().map(|i| project(&home, i)).collect())
}

fn plan_for(k: &Kipple, id: &str) -> Plan {
    k.plan(PlanRequest {
        filter: ItemFilter {
            only: vec![id.into()],
            ..Default::default()
        },
        ..Default::default()
    })
    .unwrap()
}

fn reclaim() -> Confirmation {
    Confirmation::Typed {
        word: "reclaim".into(),
    }
}

#[test]
fn playwright() {
    insta::assert_json_snapshot!("playwright", snapshot("playwright"));
}

#[test]
fn ollama() {
    insta::assert_json_snapshot!("ollama", snapshot("ollama"));
}

#[cfg(unix)]
#[test]
fn huggingface() {
    insta::assert_json_snapshot!("huggingface", snapshot("huggingface"));
}

#[test]
fn android_avd() {
    insta::assert_json_snapshot!("android-avd", snapshot("android-avd"));
}

#[test]
fn android_system_images() {
    insta::assert_json_snapshot!("android-system-images", snapshot("android-system-images"));
}

#[cfg(target_os = "macos")]
#[test]
fn xcode_derived_data() {
    insta::assert_json_snapshot!("xcode-derived-data", snapshot("xcode-derived-data"));
}

#[cfg(target_os = "macos")]
#[test]
fn ios_device_support() {
    insta::assert_json_snapshot!("ios-device-support", snapshot("ios-device-support"));
}

#[test]
fn running_emulator_keeps_avd() {
    let mut home = TestHome::new().unwrap();
    Fixture::named("android-avd")
        .unwrap()
        .generate(&mut home)
        .unwrap();
    let procs =
        kipple_core::platform::FixedProcesses::new(vec![kipple_core::platform::ProcessInfo {
            pid: 9,
            name: "qemu-system-aarch64".into(),
            args: vec!["-avd".into(), "Pixel_8_API_35".into()],
        }]);
    let k = open_with(&home, procs);
    k.scan(only("android-avd"), &NoopSink::default()).unwrap();
    let items = k
        .items(ItemQuery {
            include_keep: true,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(items.len(), 1, "snapshots aren't offered while running");
    assert_eq!(items[0].safety, Safety::Keep);
    assert_eq!(items[0].reasons[0].code, ReasonCode::ToolRunning);
}

#[cfg(unix)]
#[test]
fn ollama_rm_uses_the_cli() {
    let mut home = TestHome::new().unwrap();
    let log = home.join("ollama.log");
    home.mock_command("ollama", &format!("echo \"$@\" >> '{}'", log.display()))
        .unwrap();
    let (k, _) = scan(&mut home, "ollama", "ollama");
    let plan = plan_for(&k, "ollama");
    let llama = plan
        .entries
        .iter()
        .find(|e| e.item.title == "llama3:8b (Ollama)")
        .unwrap()
        .clone();
    let one = Plan {
        entries: vec![llama],
        ..plan
    };
    let report = k.execute(&one, reclaim(), &NoopSink::default()).unwrap();
    assert_eq!(report.done, 1, "{report:#?}");
    assert_eq!(std::fs::read_to_string(log).unwrap().trim(), "rm llama3:8b");
}

#[cfg(unix)]
#[test]
fn avdmanager_deletes_by_name() {
    use std::os::unix::fs::PermissionsExt;
    let mut home = TestHome::new().unwrap();
    Fixture::named("android-avd")
        .unwrap()
        .generate(&mut home)
        .unwrap();
    let log = home.join("avd.log");
    let bin = home.join("Android/Sdk/cmdline-tools/latest/bin");
    std::fs::create_dir_all(&bin).unwrap();
    let script = bin.join("avdmanager");
    std::fs::write(
        &script,
        format!("#!/bin/sh\necho \"$@\" >> '{}'\n", log.display()),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let k = open(&home);
    k.scan(only("android-avd"), &NoopSink::default()).unwrap();
    let plan = plan_for(&k, "android-avd");
    let avd = plan
        .entries
        .iter()
        .find(|e| e.item.kind == "android_avd")
        .unwrap()
        .clone();
    assert_eq!(avd.action, ActionKind::native("avdmanager.delete"));
    let one = Plan {
        entries: vec![avd],
        ..plan
    };
    let report = k.execute(&one, reclaim(), &NoopSink::default()).unwrap();
    assert_eq!(report.done, 1, "{report:#?}");
    assert_eq!(
        std::fs::read_to_string(log).unwrap().trim(),
        "delete avd -n Pixel_8_API_35"
    );
}

#[cfg(unix)]
mod docker {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixListener;
    use std::sync::{Arc, Mutex};

    /// A fake Docker daemon serving canned `/system/df` data.
    fn serve(sock: &Path, calls: Arc<Mutex<Vec<String>>>) {
        let listener = UnixListener::bind(sock).unwrap();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                loop {
                    let mut h = String::new();
                    reader.read_line(&mut h).unwrap();
                    if h.trim().is_empty() {
                        break;
                    }
                }
                let req = line
                    .split_whitespace()
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(" ");
                calls.lock().unwrap().push(req.clone());
                let body = match req.as_str() {
                    "GET /_ping" => "OK".to_owned(),
                    "GET /v1.41/version" => r#"{"Version":"29.8.0"}"#.to_owned(),
                    "GET /v1.41/system/df" => json!({
                        "Images": [{"Id":"sha256:abc","RepoTags":["postgres:16"],"Size":4000,"SharedSize":0,"Containers":0}],
                        "Containers": [],
                        "Volumes": [{"Name":"orphan","UsageData":{"Size":7000,"RefCount":0}}],
                        "BuildCache": [{"ID":"x","InUse":false,"Shared":false,"Size":9000}]
                    })
                    .to_string(),
                    r if r.starts_with("GET /v1.41/containers/json") => "[]".to_owned(),
                    "POST /v1.41/build/prune" => r#"{"SpaceReclaimed":9000}"#.to_owned(),
                    _ => String::new(),
                };
                let mut s = stream;
                let _ = write!(
                    s,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{body}",
                    body.len()
                );
            }
        });
    }

    #[test]
    fn scans_and_acts_through_the_api() {
        let home = TestHome::new().unwrap();
        let calls = Arc::new(Mutex::new(Vec::new()));
        serve(&home.join("docker.sock"), calls.clone());
        let k = open(&home);
        k.scan(only("docker"), &NoopSink::default()).unwrap();
        let items = k.items(ItemQuery::default()).unwrap();
        let mut titles: Vec<&str> = items.iter().map(|i| i.title.as_str()).collect();
        titles.sort();
        assert_eq!(
            titles,
            vec!["Docker build cache", "Volume orphan", "postgres:16 image"]
        );

        let plan = plan_for(&k, "docker");
        let report = k.execute(&plan, reclaim(), &NoopSink::default()).unwrap();
        assert_eq!(report.done, 3, "{report:#?}");
        assert_eq!(report.freed_now, 9000 + 7000 + 4000);
        let calls = calls.lock().unwrap();
        assert!(calls.contains(&"POST /v1.41/build/prune".to_owned()));
        assert!(calls.contains(&"DELETE /v1.41/volumes/orphan".to_owned()));
        assert!(calls.contains(&"DELETE /v1.41/images/sha256:abc".to_owned()));
    }

    #[test]
    fn unreachable_docker_is_quiet_without_the_cli() {
        let home = TestHome::new().unwrap();
        let k = open(&home);
        let r = k.scan(only("docker"), &NoopSink::default()).unwrap();
        assert!(r.warnings.is_empty(), "{:?}", r.warnings);
        assert_eq!(r.summary.items, 0);
    }
}

/// Runs only in CI's `docker` job against a throwaway daemon:
/// `KIPPLE_REAL_DOCKER=unix:///var/run/docker.sock cargo test -- --ignored real_docker`.
/// Never run it on a machine whose Docker data matters; it only reads.
#[test]
#[ignore = "needs a disposable Docker daemon (CI only)"]
fn real_docker_lists_unused_images() {
    let Ok(host) = std::env::var("KIPPLE_REAL_DOCKER") else {
        return;
    };
    let mut home = TestHome::new().unwrap();
    home.env_mut().set("DOCKER_HOST", host);
    let k = open(&home);
    k.scan(only("docker"), &NoopSink::default()).unwrap();
    let items = k
        .items(ItemQuery {
            include_keep: true,
            ..Default::default()
        })
        .unwrap();
    assert!(
        items.iter().any(|i| i.title == "hello-world:latest image"),
        "{:#?}",
        items.iter().map(|i| &i.title).collect::<Vec<_>>()
    );
}
