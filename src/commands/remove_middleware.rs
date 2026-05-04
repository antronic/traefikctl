use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{bail, Context, Result};
use colored::Colorize;

use crate::validation::validate_name;

pub struct RemoveMiddlewareOptions<'a> {
    pub name: &'a str,
    pub force: bool,
    pub dry_run: bool,
}

pub fn execute(dir: &Path, opts: RemoveMiddlewareOptions) -> Result<()> {
    validate_name(opts.name)?;

    let file_path = dir.join(format!("mw-{}.yml", opts.name));

    if !file_path.exists() {
        bail!(
            "middleware {:?} not found at {}",
            opts.name,
            file_path.display()
        );
    }

    if opts.dry_run {
        println!("{}", "--- dry-run: would remove ---".yellow().bold());
        println!("{}: {}", "file".bold(), file_path.display());
        return Ok(());
    }

    if !opts.force {
        print!("Remove middleware {}? [y/N] ", opts.name.cyan());
        io::stdout().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        if !answer.trim().eq_ignore_ascii_case("y") {
            println!("{}", "cancelled".yellow());
            return Ok(());
        }
    }

    fs::remove_file(&file_path)
        .with_context(|| format!("failed to remove {}", file_path.display()))?;

    println!(
        "{} removed middleware {}",
        "✓".green().bold(),
        opts.name.cyan()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_mw(dir: &Path, name: &str) {
        let yaml = format!("http:\n  middlewares:\n    {name}:\n      compress: {{}}\n");
        fs::write(dir.join(format!("mw-{name}.yml")), yaml).unwrap();
    }

    #[test]
    fn remove_middleware_with_force() {
        let dir = tempfile::tempdir().unwrap();
        write_mw(dir.path(), "rm-mw");
        assert!(dir.path().join("mw-rm-mw.yml").exists());

        execute(
            dir.path(),
            RemoveMiddlewareOptions {
                name: "rm-mw",
                force: true,
                dry_run: false,
            },
        )
        .unwrap();

        assert!(!dir.path().join("mw-rm-mw.yml").exists());
    }

    #[test]
    fn remove_middleware_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        write_mw(dir.path(), "dry-rm");

        execute(
            dir.path(),
            RemoveMiddlewareOptions {
                name: "dry-rm",
                force: true,
                dry_run: true,
            },
        )
        .unwrap();

        assert!(dir.path().join("mw-dry-rm.yml").exists());
    }

    #[test]
    fn remove_middleware_missing_errors() {
        let dir = tempfile::tempdir().unwrap();
        let err = execute(
            dir.path(),
            RemoveMiddlewareOptions {
                name: "ghost",
                force: true,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn remove_middleware_validates_name() {
        let dir = tempfile::tempdir().unwrap();
        let err = execute(
            dir.path(),
            RemoveMiddlewareOptions {
                name: "bad name!",
                force: true,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("invalid character"));
    }
}
