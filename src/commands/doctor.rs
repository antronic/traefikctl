use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use colored::Colorize;

use crate::config::{FileProvider, Providers, TraefikStaticConfig};

const TRAEFIK_CONFIG_CANDIDATES: &[&str] =
    &["/etc/traefik/traefik.yml", "/etc/traefik/traefik.yaml"];

pub fn find_traefik_config(override_path: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = override_path {
        if p.exists() {
            return Some(p.to_path_buf());
        }
        return None;
    }
    for candidate in TRAEFIK_CONFIG_CANDIDATES {
        let p = Path::new(candidate);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }
    None
}

pub fn ensure_setup(
    dir: &Path,
    traefik_config_override: Option<&Path>,
    dry_run: bool,
) -> Result<bool> {
    let mut all_ok = true;

    // 1. Check/create route directory
    if !dir.exists() {
        if dry_run {
            println!(
                "  {} would create directory {}",
                "○".yellow(),
                dir.display()
            );
            all_ok = false;
        } else {
            fs::create_dir_all(dir)
                .with_context(|| format!("failed to create directory {}", dir.display()))?;
            println!(
                "  {} created directory {}",
                "✓".green().bold(),
                dir.display()
            );
        }
    } else {
        println!(
            "  {} directory exists: {}",
            "✓".green().bold(),
            dir.display()
        );
    }

    // 2. Find or create traefik static config
    let config_path = find_traefik_config(traefik_config_override);
    let dir_str = dir.to_string_lossy().to_string();

    match config_path {
        Some(path) => {
            let content = fs::read_to_string(&path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let mut config: TraefikStaticConfig = serde_yaml::from_str(&content)
                .with_context(|| format!("failed to parse {}", path.display()))?;

            let needs_fix = match &config.providers {
                Some(providers) => match &providers.file {
                    Some(fp) => match &fp.directory {
                        Some(d) if d == &dir_str => {
                            println!(
                                "  {} file provider directory: {}",
                                "✓".green().bold(),
                                d.cyan()
                            );
                            let watch_ok = fp.watch.unwrap_or(false);
                            if !watch_ok {
                                println!("  {} watch is not enabled", "!".yellow().bold());
                            } else {
                                println!("  {} watch: true", "✓".green().bold());
                            }
                            !watch_ok
                        }
                        Some(d) => {
                            println!(
                                "  {} file provider directory mismatch: {} (expected {})",
                                "✗".red().bold(),
                                d.red(),
                                dir_str.green()
                            );
                            true
                        }
                        None => {
                            println!(
                                "  {} file provider exists but no directory set",
                                "✗".red().bold()
                            );
                            true
                        }
                    },
                    None => {
                        println!("  {} no file provider configured", "✗".red().bold());
                        true
                    }
                },
                None => {
                    println!("  {} no providers configured", "✗".red().bold());
                    true
                }
            };

            if needs_fix {
                if dry_run {
                    println!(
                        "  {} would update {} with file provider → {}",
                        "○".yellow(),
                        path.display(),
                        dir_str.cyan()
                    );
                    all_ok = false;
                } else {
                    let providers = config.providers.get_or_insert_with(Providers::default);
                    let file = providers.file.get_or_insert_with(FileProvider::default);
                    file.directory = Some(dir_str.clone());
                    file.watch = Some(true);

                    let yaml = serde_yaml::to_string(&config)
                        .context("failed to serialize traefik config")?;
                    fs::write(&path, &yaml)
                        .with_context(|| format!("failed to write {}", path.display()))?;
                    println!(
                        "  {} updated {} → providers.file.directory: {}",
                        "✓".green().bold(),
                        path.display(),
                        dir_str.cyan()
                    );
                }
            }
        }
        None => {
            let default_path = traefik_config_override
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from(TRAEFIK_CONFIG_CANDIDATES[0]));

            if dry_run {
                println!(
                    "  {} would create {} with file provider → {}",
                    "○".yellow(),
                    default_path.display(),
                    dir_str.cyan()
                );
                all_ok = false;
            } else {
                let config = TraefikStaticConfig {
                    providers: Some(Providers {
                        file: Some(FileProvider {
                            directory: Some(dir_str.clone()),
                            watch: Some(true),
                            rest: Default::default(),
                        }),
                        rest: Default::default(),
                    }),
                    rest: Default::default(),
                };

                if let Some(parent) = default_path.parent() {
                    if !parent.exists() {
                        fs::create_dir_all(parent).with_context(|| {
                            format!("failed to create directory {}", parent.display())
                        })?;
                    }
                }

                let yaml =
                    serde_yaml::to_string(&config).context("failed to serialize traefik config")?;
                fs::write(&default_path, &yaml)
                    .with_context(|| format!("failed to write {}", default_path.display()))?;
                println!(
                    "  {} created {} with file provider → {}",
                    "✓".green().bold(),
                    default_path.display(),
                    dir_str.cyan()
                );
            }
        }
    }

    Ok(all_ok)
}

pub fn execute(dir: &Path, traefik_config: Option<&Path>, dry_run: bool) -> Result<()> {
    println!("{}", "traefikctl doctor".bold());
    println!();

    let all_ok = ensure_setup(dir, traefik_config, dry_run)?;

    println!();
    if all_ok {
        println!("{}", "All checks passed.".green().bold());
    } else if dry_run {
        println!(
            "{}",
            "Issues found. Run without --dry-run to fix."
                .yellow()
                .bold()
        );
    } else {
        println!("{}", "Issues fixed.".green().bold());
    }

    Ok(())
}
