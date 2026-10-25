# 0009 — `docker-desktop-disk` is opt-in

**Status:** accepted · 2026-10-26

## Context
On recent macOS, reading files inside another app's container (`~/Library/Containers/com.docker.docker/…`) can trigger a "would like to access data from other apps" prompt. A read-only scan shouldn't surprise people with permission prompts (SPEC §16: Full Disk Access is never required for MVP features).

## Decision
`docker-desktop-disk` stays report-only, is marked experimental, and only runs when listed in `[integrations] enable`. The `docker` integration (through the API) covers what can actually be reclaimed.
