//! Rust adapters, one module per integration (SPEC §9.2).

pub mod ai;
pub mod android;
pub mod claude_code;
pub mod codex;
pub mod docker;
pub mod huggingface;
pub mod ollama;
pub mod playwright;
pub mod secrets;
pub mod util;
pub mod xcode;

use std::sync::Arc;

use kipple_core::integration::Adapter;

/// Every built-in adapter.
pub fn all() -> Vec<Arc<dyn Adapter>> {
    vec![
        Arc::new(docker::Docker),
        Arc::new(docker::DesktopDisk),
        Arc::new(android::Avds),
        Arc::new(android::SystemImages),
        Arc::new(xcode::DerivedData),
        Arc::new(xcode::DeviceSupport),
        Arc::new(playwright::Playwright),
        Arc::new(ollama::Ollama),
        Arc::new(huggingface::HuggingFace),
        Arc::new(claude_code::ClaudeCode),
        Arc::new(codex::Codex::default()),
    ]
}
