//! Development tasks: `cargo xtask <task>`.

use anyhow::{Result, bail};

fn main() -> Result<()> {
    let task = std::env::args().nth(1);
    match task.as_deref() {
        None | Some("help") => {
            println!("usage: cargo xtask <task>\n\ntasks:\n  help    show this message");
            Ok(())
        }
        Some(other) => bail!("unknown task: {other}"),
    }
}
