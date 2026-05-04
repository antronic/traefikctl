use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;

use crate::config::{MiddlewareDynamicConfig, TraefikDynamicConfig};

pub fn execute(dir: &Path) -> Result<()> {
    if !dir.exists() {
        println!(
            "{} directory {} does not exist — no routes configured",
            "!".yellow().bold(),
            dir.display()
        );
        return Ok(());
    }

    let mut all_entries: Vec<_> = fs::read_dir(dir)
        .with_context(|| format!("failed to read directory {}", dir.display()))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .extension()
                .map(|ext| ext == "yml" || ext == "yaml")
                .unwrap_or(false)
        })
        .collect();

    all_entries.sort_by_key(|e| e.file_name());

    let (mw_entries, route_entries): (Vec<_>, Vec<_>) = all_entries.into_iter().partition(|e| {
        e.file_name()
            .to_str()
            .map(|n| n.starts_with("mw-"))
            .unwrap_or(false)
    });

    if route_entries.is_empty() && mw_entries.is_empty() {
        println!(
            "{} no routes or middlewares configured in {}",
            "!".yellow().bold(),
            dir.display()
        );
        return Ok(());
    }

    if !route_entries.is_empty() {
        println!(
            "{} {} route(s) in {}:\n",
            "●".blue().bold(),
            route_entries.len(),
            dir.display()
        );

        for entry in &route_entries {
            let path = entry.path();
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "  {} failed to read {}: {}",
                        "✗".red().bold(),
                        path.display(),
                        e
                    );
                    continue;
                }
            };

            let config: TraefikDynamicConfig = match serde_yaml::from_str(&content) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "  {} failed to parse {}: {}",
                        "✗".red().bold(),
                        path.display(),
                        e
                    );
                    continue;
                }
            };

            let name = config.route_name().unwrap_or("?");
            let host = config.host().unwrap_or_else(|| "?".to_string());
            let url = config.backend_url().unwrap_or("?");

            let eps: Vec<&str> = config
                .http
                .routers
                .values()
                .next()
                .map(|r| r.entry_points.iter().map(|s| s.as_str()).collect())
                .unwrap_or_default();

            let tls_badge = if config
                .http
                .routers
                .values()
                .next()
                .and_then(|r| r.tls.as_ref())
                .is_some()
            {
                " 🔒"
            } else {
                ""
            };

            println!(
                "  {} {} → {} → {} [{}]{}",
                "▸".dimmed(),
                name.cyan().bold(),
                host.yellow(),
                url.blue(),
                eps.join(", "),
                tls_badge
            );
        }
        println!();
    }

    if !mw_entries.is_empty() {
        println!(
            "{} {} middleware(s):\n",
            "●".blue().bold(),
            mw_entries.len(),
        );

        for entry in &mw_entries {
            let path = entry.path();
            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "  {} failed to read {}: {}",
                        "✗".red().bold(),
                        path.display(),
                        e
                    );
                    continue;
                }
            };

            let config: MiddlewareDynamicConfig = match serde_yaml::from_str(&content) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "  {} failed to parse {}: {}",
                        "✗".red().bold(),
                        path.display(),
                        e
                    );
                    continue;
                }
            };

            let name = config.middleware_name().unwrap_or("?");
            let mw_type = config.middleware_type().unwrap_or("unknown");

            println!(
                "  {} {} ({})",
                "▸".dimmed(),
                name.cyan().bold(),
                mw_type.yellow(),
            );
        }
        println!();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_route(dir: &Path, name: &str) {
        let yaml = format!(
            "http:\n  routers:\n    {name}:\n      rule: \"Host(`{name}.test`)\"\n      entryPoints:\n      - web\n      service: {name}\n  services:\n    {name}:\n      loadBalancer:\n        servers:\n        - url: http://127.0.0.1:80\n"
        );
        fs::write(dir.join(format!("{name}.yml")), yaml).unwrap();
    }

    fn write_middleware(dir: &Path, name: &str) {
        let yaml = format!(
            "http:\n  middlewares:\n    {name}:\n      compress: {{}}\n"
        );
        fs::write(dir.join(format!("mw-{name}.yml")), yaml).unwrap();
    }

    #[test]
    fn list_nonexistent_dir_ok() {
        let dir = tempfile::tempdir().unwrap();
        let gone = dir.path().join("nope");
        execute(&gone).unwrap();
    }

    #[test]
    fn list_empty_dir_ok() {
        let dir = tempfile::tempdir().unwrap();
        execute(dir.path()).unwrap();
    }

    #[test]
    fn list_routes_only() {
        let dir = tempfile::tempdir().unwrap();
        write_route(dir.path(), "app1");
        write_route(dir.path(), "app2");
        execute(dir.path()).unwrap();
    }

    #[test]
    fn list_middlewares_only() {
        let dir = tempfile::tempdir().unwrap();
        write_middleware(dir.path(), "compress");
        execute(dir.path()).unwrap();
    }

    #[test]
    fn list_mixed_routes_and_middlewares() {
        let dir = tempfile::tempdir().unwrap();
        write_route(dir.path(), "myapp");
        write_middleware(dir.path(), "headers");
        execute(dir.path()).unwrap();
    }

    #[test]
    fn list_ignores_non_yaml_files() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("notes.txt"), "not yaml").unwrap();
        execute(dir.path()).unwrap();
    }

    #[test]
    fn list_handles_malformed_yaml_gracefully() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("bad.yml"), "not: [valid: yaml: config").unwrap();
        execute(dir.path()).unwrap();
    }
}
