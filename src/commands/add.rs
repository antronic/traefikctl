use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use colored::Colorize;

use crate::config::{RouterTls, TraefikDynamicConfig};
use crate::validation::{validate_host, validate_name, validate_url};

pub struct AddOptions<'a> {
    pub name: &'a str,
    pub host: &'a str,
    pub url: &'a str,
    pub entrypoint: &'a str,
    pub tls: bool,
    pub cert_resolver: &'a str,
    pub middlewares: Option<&'a str>,
    pub force: bool,
    pub dry_run: bool,
}

pub fn execute(dir: &Path, opts: AddOptions) -> Result<()> {
    // Validate inputs
    validate_name(opts.name)?;
    validate_host(opts.host)?;
    validate_url(opts.url)?;

    let file_path = dir.join(format!("{}.yml", opts.name));

    // Check for existing route
    if file_path.exists() && !opts.force {
        bail!(
            "route {:?} already exists at {}. Use --force to overwrite.",
            opts.name,
            file_path.display()
        );
    }

    // Build entrypoints list
    let mut entrypoints = vec![opts.entrypoint.to_string()];
    if opts.tls && opts.entrypoint == "web" {
        entrypoints.push("websecure".to_string());
    }

    // Build TLS config
    let tls = if opts.tls {
        Some(RouterTls {
            cert_resolver: opts.cert_resolver.to_string(),
        })
    } else {
        None
    };

    // Parse middlewares
    let middlewares = opts.middlewares.map(|m| {
        m.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    });

    // Build config
    let config = TraefikDynamicConfig::new(
        opts.name,
        opts.host,
        opts.url,
        entrypoints,
        tls,
        middlewares,
    );

    let yaml = config
        .to_yaml()
        .context("failed to serialize config to YAML")?;

    if opts.dry_run {
        println!("{}", "--- dry-run: would write ---".yellow().bold());
        println!("{}: {}", "file".bold(), file_path.display());
        println!("{}", yaml);
        return Ok(());
    }

    // Ensure directory exists
    if !dir.exists() {
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create directory {}", dir.display()))?;
    }

    fs::write(&file_path, &yaml)
        .with_context(|| format!("failed to write {}", file_path.display()))?;

    println!(
        "{} route {} → {} → {}",
        "✓".green().bold(),
        opts.name.cyan(),
        opts.host.yellow(),
        opts.url.blue()
    );
    println!("  {}: {}", "file".dimmed(), file_path.display());

    Ok(())
}
