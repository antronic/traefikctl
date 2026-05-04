use std::fs;
use std::io::{self, Write};
use std::path::Path;

use anyhow::{bail, Context, Result};
use colored::Colorize;

pub struct RemoveOptions<'a> {
    pub name: &'a str,
    pub force: bool,
    pub dry_run: bool,
}

pub fn execute(dir: &Path, opts: RemoveOptions) -> Result<()> {
    crate::validation::validate_name(opts.name)?;

    let file_path = dir.join(format!("{}.yml", opts.name));

    if !file_path.exists() {
        bail!("route {:?} not found at {}", opts.name, file_path.display());
    }

    if opts.dry_run {
        println!(
            "{} would remove route {} ({})",
            "--- dry-run:".yellow().bold(),
            opts.name.cyan(),
            file_path.display()
        );
        return Ok(());
    }

    // Confirm unless --force
    if !opts.force {
        print!("Remove route {}? [y/N] ", opts.name.cyan());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        if input != "y" && input != "yes" {
            println!("{}", "aborted".yellow());
            return Ok(());
        }
    }

    fs::remove_file(&file_path)
        .with_context(|| format!("failed to remove {}", file_path.display()))?;

    println!("{} removed route {}", "✓".green().bold(), opts.name.cyan());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn tmp_with_route(name: &str) -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        let content = format!(
            "http:\n  routers:\n    {name}:\n      rule: \"Host(`{name}.test`)\"\n      entryPoints: [web]\n      service: {name}\n  services:\n    {name}:\n      loadBalancer:\n        servers:\n          - url: http://127.0.0.1:80\n"
        );
        fs::write(dir.path().join(format!("{name}.yml")), content).unwrap();
        dir
    }

    #[test]
    fn remove_deletes_file_with_force() {
        let dir = tmp_with_route("myapp");
        assert!(dir.path().join("myapp.yml").exists());

        execute(
            dir.path(),
            RemoveOptions {
                name: "myapp",
                force: true,
                dry_run: false,
            },
        )
        .unwrap();

        assert!(!dir.path().join("myapp.yml").exists());
    }

    #[test]
    fn remove_dry_run_keeps_file() {
        let dir = tmp_with_route("keepme");

        execute(
            dir.path(),
            RemoveOptions {
                name: "keepme",
                force: true,
                dry_run: true,
            },
        )
        .unwrap();

        assert!(dir.path().join("keepme.yml").exists());
    }

    #[test]
    fn remove_missing_route_errors() {
        let dir = tempfile::tempdir().unwrap();
        let err = execute(
            dir.path(),
            RemoveOptions {
                name: "ghost",
                force: true,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn remove_validates_name() {
        let dir = tempfile::tempdir().unwrap();
        let err = execute(
            dir.path(),
            RemoveOptions {
                name: "bad name!",
                force: true,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("invalid character"));
    }
}
