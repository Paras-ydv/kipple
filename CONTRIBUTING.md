# Contributing to kipple

Thanks for helping. kipple is pre-alpha; this guide will grow.

## Setup

1. Install Rust with [rustup](https://rustup.rs). The minimum supported version is in `Cargo.toml` (`rust-version`).
2. Clone the repository and run:

   ```
   cargo build
   cargo test --workspace
   ```

## Before you open a pull request

```
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

## Ground rules

- Read `docs/SPEC.md` first. It is the source of truth; `docs/ROADMAP.md` orders the work.
- Never run kipple, its tests or scripts against your real home directory. Tests use the `TestHome` helper, which points every relevant environment variable into a temp directory.
- Every destructive code path goes through the executor. Changes to safety rules or the executor need tests in the same change.
- Commits follow [Conventional Commits](https://www.conventionalcommits.org): `feat(adapters): add android-avd`.

## Reporting a tool's storage path

If kipple misses data because your tool stores it somewhere else on your OS, open a "path report" issue with the tool, its version, your OS and the path.
