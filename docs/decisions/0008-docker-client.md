# 0008 — A small blocking Docker API client instead of bollard

**Status:** accepted · 2026-10-26

## Context
SPEC §3.3 suggests `bollard` for Docker. bollard is async and brings tokio, hyper and their dependencies (about 100 crates), and SPEC §3.2 would need a private runtime behind a sync facade. kipple needs five endpoints: `/_ping`, `/version`, `/system/df`, a filtered `/containers/json`, and three DELETEs plus `POST /build/prune`.

## Decision
`kipple-adapters/src/docker/client.rs` is a ~250-line blocking HTTP/1.1 client over a Unix socket, TCP or a Windows named pipe (opened as a file). It handles content-length and chunked bodies. Requests use API v1.41 paths so field meanings stay fixed on newer engines (current engines support 1.40 and up).

## Consequences
- No async runtime in the CLI; the core stays sync.
- Tests run a fake daemon on a Unix socket in the test home, so Docker logic is covered on every CI run without a real daemon.
- TLS (`DOCKER_TLS_VERIFY`) and `ssh://` hosts aren't supported yet; `doctor` says so. If more of the API is needed later, revisit bollard.
