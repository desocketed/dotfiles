//! The upstream sink the user last settled on, kept across daemon starts:
//! `$XDG_STATE_HOME/novamix/upstream`, one `node.name`.
//!
//! WirePlumber remembers GLOBAL as the default once `take_default` has made it so, so the
//! default at the next start is ours and says nothing about the user's real sink.

use std::path::PathBuf;

use anyhow::anyhow;
use novamix::Result;

fn path() -> Result<PathBuf> {
    let dir = dirs::state_dir().ok_or(anyhow!("No XDG_STATE_HOME"))?;
    Ok(dir.join("novamix").join("upstream"))
}

/// The remembered `node.name`, if any.
pub fn load() -> Option<String> {
    let text = std::fs::read_to_string(path().ok()?).ok()?;
    let name = text.trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// Replaces the remembered name atomically: a file beside it, then a rename.
pub fn save(name: &str) -> Result<()> {
    let path = path()?;
    let dir = path
        .parent()
        .ok_or(anyhow!("State path has no directory"))?;
    std::fs::create_dir_all(dir)?;
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, format!("{name}\n"))?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}
