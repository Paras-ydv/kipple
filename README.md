# kipple

*/ˈkɪp.əl/ — rhymes with ripple. The junk that piles up on its own.*

kipple finds the disk space your development tools quietly eat — dependency folders, build output, package caches, Docker, emulators, model stores and AI-agent history — tells you what each item is and whether it is safe to remove, and reclaims it with an undo.

**Status: pre-alpha.** Nothing here is ready for use yet.

## Integrations

<!-- integrations:start -->
| Category | Integration | What | Kind | Status | Systems |
|---|---|---|---|---|---|
| Build caches | [`cachedir-tag`](docs/integrations/cachedir-tag.md) | Tagged caches | pack | stable | macOS, Linux, Windows |
| Containers | [`docker`](docs/integrations/docker.md) | Docker | adapter | stable | macOS, Linux, Windows |
| Containers | [`docker-desktop-disk`](docs/integrations/docker-desktop-disk.md) | Docker Desktop disk | adapter | experimental | macOS, Linux, Windows |
| Mobile | [`android-avd`](docs/integrations/android-avd.md) | Android emulators | adapter | stable | macOS, Linux, Windows |
| Mobile | [`android-system-images`](docs/integrations/android-system-images.md) | Android system images | adapter | stable | macOS, Linux, Windows |
| Mobile | [`ios-device-support`](docs/integrations/ios-device-support.md) | Device support files | adapter | stable | macOS |
| Mobile | [`xcode-derived-data`](docs/integrations/xcode-derived-data.md) | Xcode DerivedData | adapter | stable | macOS |
| Models | [`huggingface`](docs/integrations/huggingface.md) | Hugging Face cache | adapter | stable | macOS, Linux, Windows |
| Models | [`ollama`](docs/integrations/ollama.md) | Ollama models | adapter | stable | macOS, Linux, Windows |
| Package caches | [`bun-cache`](docs/integrations/bun-cache.md) | Bun cache | pack | stable | macOS, Linux, Windows |
| Package caches | [`cargo-registry`](docs/integrations/cargo-registry.md) | Cargo registry | pack | stable | macOS, Linux, Windows |
| Package caches | [`go-build-cache`](docs/integrations/go-build-cache.md) | Go build cache | pack | stable | macOS, Linux, Windows |
| Package caches | [`go-mod-cache`](docs/integrations/go-mod-cache.md) | Go module cache | pack | stable | macOS, Linux, Windows |
| Package caches | [`gradle-cache`](docs/integrations/gradle-cache.md) | Gradle cache | pack | stable | macOS, Linux, Windows |
| Package caches | [`homebrew-cache`](docs/integrations/homebrew-cache.md) | Homebrew cache | pack | stable | macOS, Linux |
| Package caches | [`npm-cache`](docs/integrations/npm-cache.md) | npm cache | pack | stable | macOS, Linux, Windows |
| Package caches | [`pip-cache`](docs/integrations/pip-cache.md) | pip cache | pack | stable | macOS, Linux, Windows |
| Package caches | [`playwright`](docs/integrations/playwright.md) | Playwright browsers | adapter | stable | macOS, Linux, Windows |
| Package caches | [`pnpm-store`](docs/integrations/pnpm-store.md) | pnpm store | pack | stable | macOS, Linux, Windows |
| Package caches | [`puppeteer`](docs/integrations/puppeteer.md) | Puppeteer browsers | pack | stable | macOS, Linux, Windows |
| Package caches | [`uv-cache`](docs/integrations/uv-cache.md) | uv cache | pack | stable | macOS, Linux, Windows |
| Package caches | [`yarn-cache`](docs/integrations/yarn-cache.md) | Yarn cache | pack | stable | macOS, Linux, Windows |
| Project artifacts | [`gradle-project`](docs/integrations/gradle-project.md) | Gradle project build | pack | stable | macOS, Linux, Windows |
| Project artifacts | [`js-build`](docs/integrations/js-build.md) | JavaScript build output | pack | stable | macOS, Linux, Windows |
| Project artifacts | [`maven-project`](docs/integrations/maven-project.md) | Maven target | pack | stable | macOS, Linux, Windows |
| Project artifacts | [`node-modules`](docs/integrations/node-modules.md) | node_modules | pack | stable | macOS, Linux, Windows |
| Project artifacts | [`python-caches`](docs/integrations/python-caches.md) | Python caches | pack | stable | macOS, Linux, Windows |
| Project artifacts | [`python-venv`](docs/integrations/python-venv.md) | Python virtual environments | pack | stable | macOS, Linux, Windows |
| Project artifacts | [`rust-target`](docs/integrations/rust-target.md) | Rust target | pack | stable | macOS, Linux, Windows |
| Project artifacts | [`xcode-project`](docs/integrations/xcode-project.md) | Swift and CocoaPods project build | pack | stable | macOS, Linux, Windows |
<!-- integrations:end -->

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in kipple by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
