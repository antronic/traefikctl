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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TraefikDynamicConfig;
    use std::fs;

    fn create_route(dir: &Path, name: &str) {
        let cfg = TraefikDynamicConfig::new(
            name,
            &format!("{name}.example.com"),
            &format!("http://{name}:80"),
            vec!["web".into()],
            None,
            None,
        );
        fs::write(
            dir.join(format!("{name}.yml")),
            cfg.to_yaml().unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn update_host() {
        let dir = tempfile::tempdir().unwrap();
        create_route(dir.path(), "up");

        execute(
            dir.path(),
            UpdateOptions {
                name: "up",
                host: Some("new.example.com"),
                url: None,
                entrypoint: None,
                tls: None,
                middlewares: None,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("up.yml")).unwrap();
        assert!(content.contains("Host(`new.example.com`)"));
    }

    #[test]
    fn update_url() {
        let dir = tempfile::tempdir().unwrap();
        create_route(dir.path(), "urlup");

        execute(
            dir.path(),
            UpdateOptions {
                name: "urlup",
                host: None,
                url: Some("http://newback:9999"),
                entrypoint: None,
                tls: None,
                middlewares: None,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("urlup.yml")).unwrap();
        assert!(content.contains("http://newback:9999"));
    }

    #[test]
    fn update_enable_tls() {
        let dir = tempfile::tempdir().unwrap();
        create_route(dir.path(), "tlsup");

        execute(
            dir.path(),
            UpdateOptions {
                name: "tlsup",
                host: None,
                url: None,
                entrypoint: None,
                tls: Some(true),
                middlewares: None,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("tlsup.yml")).unwrap();
        assert!(content.contains("certResolver:"));
        assert!(content.contains("websecure"));
    }

    #[test]
    fn update_disable_tls() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = TraefikDynamicConfig::new(
            "tlsoff",
            "tlsoff.example.com",
            "http://tlsoff:80",
            vec!["web".into(), "websecure".into()],
            Some(RouterTls {
                cert_resolver: "letsencrypt".to_string(),
            }),
            None,
        );
        fs::write(
            dir.path().join("tlsoff.yml"),
            cfg.to_yaml().unwrap(),
        )
        .unwrap();

        execute(
            dir.path(),
            UpdateOptions {
                name: "tlsoff",
                host: None,
                url: None,
                entrypoint: None,
                tls: Some(false),
                middlewares: None,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("tlsoff.yml")).unwrap();
        assert!(!content.contains("certResolver:"));
        assert!(!content.contains("websecure"));
    }

    #[test]
    fn update_middlewares() {
        let dir = tempfile::tempdir().unwrap();
        create_route(dir.path(), "mwup");

        execute(
            dir.path(),
            UpdateOptions {
                name: "mwup",
                host: None,
                url: None,
                entrypoint: None,
                tls: None,
                middlewares: Some("auth,headers"),
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("mwup.yml")).unwrap();
        assert!(content.contains("auth"));
        assert!(content.contains("headers"));
    }

    #[test]
    fn update_clear_middlewares() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = TraefikDynamicConfig::new(
            "clrmw",
            "clrmw.example.com",
            "http://clrmw:80",
            vec!["web".into()],
            None,
            Some(vec!["auth".into()]),
        );
        fs::write(
            dir.path().join("clrmw.yml"),
            cfg.to_yaml().unwrap(),
        )
        .unwrap();

        execute(
            dir.path(),
            UpdateOptions {
                name: "clrmw",
                host: None,
                url: None,
                entrypoint: None,
                tls: None,
                middlewares: Some(""),
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("clrmw.yml")).unwrap();
        assert!(!content.contains("middlewares:"));
    }

    #[test]
    fn update_dry_run_no_write() {
        let dir = tempfile::tempdir().unwrap();
        create_route(dir.path(), "dryup");
        let before = fs::read_to_string(dir.path().join("dryup.yml")).unwrap();

        execute(
            dir.path(),
            UpdateOptions {
                name: "dryup",
                host: Some("changed.example.com"),
                url: None,
                entrypoint: None,
                tls: None,
                middlewares: None,
                dry_run: true,
            },
        )
        .unwrap();

        let after = fs::read_to_string(dir.path().join("dryup.yml")).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn update_missing_route_errors() {
        let dir = tempfile::tempdir().unwrap();
        let err = execute(
            dir.path(),
            UpdateOptions {
                name: "ghost",
                host: Some("x.example.com"),
                url: None,
                entrypoint: None,
                tls: None,
                middlewares: None,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn update_validates_host() {
        let dir = tempfile::tempdir().unwrap();
        create_route(dir.path(), "valhost");
        let err = execute(
            dir.path(),
            UpdateOptions {
                name: "valhost",
                host: Some("bad host!"),
                url: None,
                entrypoint: None,
                tls: None,
                middlewares: None,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("whitespace") || err.to_string().contains("invalid"));
    }

    #[test]
    fn update_validates_url() {
        let dir = tempfile::tempdir().unwrap();
        create_route(dir.path(), "valurl");
        let err = execute(
            dir.path(),
            UpdateOptions {
                name: "valurl",
                host: None,
                url: Some("ftp://bad"),
                entrypoint: None,
                tls: None,
                middlewares: None,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("http://"));
    }
}
