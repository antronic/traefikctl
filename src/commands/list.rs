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
            e.path().is_file()
                && e.path()
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

            let protocol = config.protocol();
            let name = config.route_name().unwrap_or("?");

            match protocol {
                "http" => print_http_route(&config, name),
                "tcp" => print_tcp_route(&config, name),
                "udp" => print_udp_route(&config, name),
                _ => {
                    println!("  {} {} (unknown protocol)", "▸".dimmed(), name.cyan().bold());
                }
            }
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

fn print_http_route(config: &TraefikDynamicConfig, name: &str) {
    let host = config.host().unwrap_or_else(|| "?".to_string());
    let url = config.backend_url().unwrap_or("?");

    let eps: Vec<&str> = config
        .http
        .as_ref()
        .and_then(|h| h.routers.values().next())
        .map(|r| r.entry_points.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    let tls_badge = if config
        .http
        .as_ref()
        .and_then(|h| h.routers.values().next())
        .and_then(|r| r.tls.as_ref())
        .is_some()
    {
        " 🔒"
    } else {
        ""
    };

    println!(
        "  {} [{}] {} → {} → {} [{}]{}",
        "▸".dimmed(),
        "HTTP".cyan(),
        name.cyan().bold(),
        host.yellow(),
        url.blue(),
        eps.join(", "),
        tls_badge
    );
}

fn print_tcp_route(config: &TraefikDynamicConfig, name: &str) {
    let rule = config.tcp_rule().unwrap_or("?");
    let addr = config.backend_address().unwrap_or("?");

    let eps: Vec<&str> = config
        .tcp
        .as_ref()
        .and_then(|t| t.routers.values().next())
        .map(|r| r.entry_points.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    let tls_badge = if config
        .tcp
        .as_ref()
        .and_then(|t| t.routers.values().next())
        .and_then(|r| r.tls.as_ref())
        .is_some()
    {
        " 🔒"
    } else {
        ""
    };

    println!(
        "  {} [{}] {} → {} → {} [{}]{}",
        "▸".dimmed(),
        "TCP".magenta(),
        name.cyan().bold(),
        rule.yellow(),
        addr.blue(),
        eps.join(", "),
        tls_badge
    );
}

fn print_udp_route(config: &TraefikDynamicConfig, name: &str) {
    let addr = config.backend_address().unwrap_or("?");

    let eps: Vec<&str> = config
        .udp
        .as_ref()
        .and_then(|u| u.routers.values().next())
        .map(|r| r.entry_points.iter().map(|s| s.as_str()).collect())
        .unwrap_or_default();

    println!(
        "  {} [{}] {} → {} [{}]",
        "▸".dimmed(),
        "UDP".green(),
        name.cyan().bold(),
        addr.blue(),
        eps.join(", "),
    );
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

    fn write_tcp_route(dir: &Path, name: &str) {
        let yaml = format!(
            "tcp:\n  routers:\n    {name}:\n      rule: \"HostSNI(`*`)\"\n      entryPoints:\n      - postgres\n      service: {name}\n  services:\n    {name}:\n      loadBalancer:\n        servers:\n        - address: 10.0.0.5:5432\n"
        );
        fs::write(dir.join(format!("{name}.yml")), yaml).unwrap();
    }

    fn write_udp_route(dir: &Path, name: &str) {
        let yaml = format!(
            "udp:\n  routers:\n    {name}:\n      entryPoints:\n      - dns\n      service: {name}\n  services:\n    {name}:\n      loadBalancer:\n        servers:\n        - address: 10.0.0.2:53\n"
        );
        fs::write(dir.join(format!("{name}.yml")), yaml).unwrap();
    }

    fn write_middleware(dir: &Path, name: &str) {
        let yaml = format!("http:\n  middlewares:\n    {name}:\n      compress: {{}}\n");
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

    #[test]
    fn list_tcp_routes() {
        let dir = tempfile::tempdir().unwrap();
        write_tcp_route(dir.path(), "postgres");
        execute(dir.path()).unwrap();
    }

    #[test]
    fn list_udp_routes() {
        let dir = tempfile::tempdir().unwrap();
        write_udp_route(dir.path(), "dns");
        execute(dir.path()).unwrap();
    }

    #[test]
    fn list_mixed_protocols() {
        let dir = tempfile::tempdir().unwrap();
        write_route(dir.path(), "webapp");
        write_tcp_route(dir.path(), "postgres");
        write_udp_route(dir.path(), "dns");
        write_middleware(dir.path(), "headers");
        execute(dir.path()).unwrap();
    }
}
