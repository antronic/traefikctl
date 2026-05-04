use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use colored::Colorize;

use crate::config::{RouterTls, TraefikDynamicConfig};
use crate::validation::{validate_host, validate_url};

pub struct UpdateOptions<'a> {
    pub name: &'a str,
    pub host: Option<&'a str>,
    pub url: Option<&'a str>,
    pub entrypoint: Option<&'a str>,
    pub tls: Option<bool>,
    pub middlewares: Option<&'a str>,
    pub dry_run: bool,
}

pub fn execute(dir: &Path, opts: UpdateOptions) -> Result<()> {
    crate::validation::validate_name(opts.name)?;

    let file_path = dir.join(format!("{}.yml", opts.name));

    if !file_path.exists() {
        bail!(
            "route {:?} not found at {}. Use 'add' to create it.",
            opts.name,
            file_path.display()
        );
    }

    // Validate new values if provided
    if let Some(host) = opts.host {
        validate_host(host)?;
    }
    if let Some(url) = opts.url {
        validate_url(url)?;
    }

    // Read existing config
    let content = fs::read_to_string(&file_path)
        .with_context(|| format!("failed to read {}", file_path.display()))?;
    let mut config: TraefikDynamicConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("failed to parse {}", file_path.display()))?;

    // Apply updates to router
    if let Some(router) = config.http.routers.get_mut(opts.name) {
        if let Some(host) = opts.host {
            router.rule = format!("Host(`{host}`)");
        }
        if let Some(ep) = opts.entrypoint {
            router.entry_points = vec![ep.to_string()];
        }
        if let Some(enable_tls) = opts.tls {
            if enable_tls {
                router.tls = Some(RouterTls {
                    cert_resolver: "letsencrypt".to_string(),
                });
                // Add websecure entrypoint if not present
                if !router.entry_points.contains(&"websecure".to_string()) {
                    router.entry_points.push("websecure".to_string());
                }
            } else {
                router.tls = None;
                router.entry_points.retain(|ep| ep != "websecure");
            }
        }
        if let Some(mw) = opts.middlewares {
            let parsed: Vec<String> = mw
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if parsed.is_empty() {
                router.middlewares = None;
            } else {
                router.middlewares = Some(parsed);
            }
        }
    } else {
        bail!("router {:?} not found in config file", opts.name);
    }

    // Apply updates to service
    if let Some(url) = opts.url {
        if let Some(service) = config.http.services.get_mut(opts.name) {
            if let Some(server) = service.load_balancer.servers.first_mut() {
                server.url = url.to_string();
            }
        }
    }

    let yaml = config
        .to_yaml()
        .context("failed to serialize updated config")?;

    if opts.dry_run {
        println!("{}", "--- dry-run: would write ---".yellow().bold());
        println!("{}: {}", "file".bold(), file_path.display());
        println!("{yaml}");
        return Ok(());
    }

    fs::write(&file_path, &yaml)
        .with_context(|| format!("failed to write {}", file_path.display()))?;

    println!("{} updated route {}", "✓".green().bold(), opts.name.cyan());
    println!("  {}: {}", "file".dimmed(), file_path.display());

    Ok(())
}
