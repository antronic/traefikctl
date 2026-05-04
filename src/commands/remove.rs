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
