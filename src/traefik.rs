use anyhow::{Context, Result};
use std::process::Command;

/// Reload Traefik via systemctl.
/// Attempts `reload` first; falls back to `restart` on failure.
pub fn reload_traefik() -> Result<()> {
    let reload = Command::new("systemctl")
        .args(["reload", "traefik"])
        .output()
        .context("failed to execute systemctl reload traefik")?;

    if reload.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&reload.stderr);
    eprintln!(
        "systemctl reload traefik failed ({}), trying restart...",
        stderr.trim()
    );

    let restart = Command::new("systemctl")
        .args(["restart", "traefik"])
        .output()
        .context("failed to execute systemctl restart traefik")?;

    if restart.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&restart.stderr);
        anyhow::bail!("failed to reload/restart traefik: {}", stderr.trim());
    }
}
