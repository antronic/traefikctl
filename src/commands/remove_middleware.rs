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
