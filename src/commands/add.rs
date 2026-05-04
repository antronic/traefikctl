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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    fn default_opts(name: &str, host: &str, url: &str) -> (String, String, String) {
        (name.to_string(), host.to_string(), url.to_string())
    }

    #[test]
    fn add_creates_route_file() {
        let dir = tmp();
        let (n, h, u) = default_opts("myapp", "app.example.com", "http://127.0.0.1:3000");
        execute(
            dir.path(),
            AddOptions {
                name: &n,
                host: &h,
                url: &u,
                entrypoint: "web",
                tls: false,
                cert_resolver: "letsencrypt",
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        let file = dir.path().join("myapp.yml");
        assert!(file.exists());
        let content = fs::read_to_string(&file).unwrap();
        assert!(content.contains("Host(`app.example.com`)"));
        assert!(content.contains("http://127.0.0.1:3000"));
    }

    #[test]
    fn add_with_tls_adds_websecure() {
        let dir = tmp();
        let (n, h, u) = default_opts("tls-app", "tls.example.com", "http://back:443");
        execute(
            dir.path(),
            AddOptions {
                name: &n,
                host: &h,
                url: &u,
                entrypoint: "web",
                tls: true,
                cert_resolver: "myresolver",
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("tls-app.yml")).unwrap();
        assert!(content.contains("websecure"));
        assert!(content.contains("certResolver: myresolver"));
    }

    #[test]
    fn add_with_middlewares() {
        let dir = tmp();
        let (n, h, u) = default_opts("mw-app", "mw.example.com", "http://back:80");
        execute(
            dir.path(),
            AddOptions {
                name: &n,
                host: &h,
                url: &u,
                entrypoint: "web",
                tls: false,
                cert_resolver: "letsencrypt",
                middlewares: Some("headers,rate-limit"),
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("mw-app.yml")).unwrap();
        assert!(content.contains("headers"));
        assert!(content.contains("rate-limit"));
    }

    #[test]
    fn add_dry_run_does_not_write() {
        let dir = tmp();
        let (n, h, u) = default_opts("dry", "dry.example.com", "http://dry:80");
        execute(
            dir.path(),
            AddOptions {
                name: &n,
                host: &h,
                url: &u,
                entrypoint: "web",
                tls: false,
                cert_resolver: "letsencrypt",
                middlewares: None,
                force: false,
                dry_run: true,
            },
        )
        .unwrap();

        assert!(!dir.path().join("dry.yml").exists());
    }

    #[test]
    fn add_rejects_duplicate_without_force() {
        let dir = tmp();
        let (n, h, u) = default_opts("dup", "dup.example.com", "http://dup:80");

        let opts = || AddOptions {
            name: &n,
            host: &h,
            url: &u,
            entrypoint: "web",
            tls: false,
            cert_resolver: "letsencrypt",
            middlewares: None,
            force: false,
            dry_run: false,
        };

        execute(dir.path(), opts()).unwrap();
        let err = execute(dir.path(), opts()).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn add_force_overwrites() {
        let dir = tmp();
        let (n, h, u) = default_opts("force", "force.example.com", "http://force:80");

        execute(
            dir.path(),
            AddOptions {
                name: &n,
                host: &h,
                url: &u,
                entrypoint: "web",
                tls: false,
                cert_resolver: "letsencrypt",
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        let new_url = "http://force:9999".to_string();
        execute(
            dir.path(),
            AddOptions {
                name: &n,
                host: &h,
                url: &new_url,
                entrypoint: "web",
                tls: false,
                cert_resolver: "letsencrypt",
                middlewares: None,
                force: true,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("force.yml")).unwrap();
        assert!(content.contains("http://force:9999"));
    }

    #[test]
    fn add_validates_name() {
        let dir = tmp();
        let (n, h, u) = default_opts("bad name!", "ok.example.com", "http://ok:80");
        let err = execute(
            dir.path(),
            AddOptions {
                name: &n,
                host: &h,
                url: &u,
                entrypoint: "web",
                tls: false,
                cert_resolver: "letsencrypt",
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("invalid character"));
    }

    #[test]
    fn add_validates_host() {
        let dir = tmp();
        let (n, h, u) = default_opts("ok", "not valid host", "http://ok:80");
        let err = execute(
            dir.path(),
            AddOptions {
                name: &n,
                host: &h,
                url: &u,
                entrypoint: "web",
                tls: false,
                cert_resolver: "letsencrypt",
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("whitespace"));
    }

    #[test]
    fn add_validates_url() {
        let dir = tmp();
        let (n, h, u) = default_opts("ok", "ok.example.com", "ftp://bad");
        let err = execute(
            dir.path(),
            AddOptions {
                name: &n,
                host: &h,
                url: &u,
                entrypoint: "web",
                tls: false,
                cert_resolver: "letsencrypt",
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("http://"));
    }

    #[test]
    fn add_creates_missing_directory() {
        let dir = tmp();
        let nested = dir.path().join("sub").join("dir");
        let (n, h, u) = default_opts("nested", "n.example.com", "http://n:80");
        execute(
            &nested,
            AddOptions {
                name: &n,
                host: &h,
                url: &u,
                entrypoint: "web",
                tls: false,
                cert_resolver: "letsencrypt",
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap();
        assert!(nested.join("nested.yml").exists());
    }

    #[test]
    fn add_tls_custom_entrypoint_no_websecure() {
        let dir = tmp();
        let (n, h, u) = default_opts("cep", "cep.example.com", "http://cep:80");
        execute(
            dir.path(),
            AddOptions {
                name: &n,
                host: &h,
                url: &u,
                entrypoint: "custom-ep",
                tls: true,
                cert_resolver: "letsencrypt",
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("cep.yml")).unwrap();
        assert!(content.contains("custom-ep"));
        assert!(!content.contains("websecure"));
    }

    #[test]
    fn add_yaml_roundtrips_correctly() {
        let dir = tmp();
        let (n, h, u) = default_opts("rt", "rt.example.com", "http://rt:80");
        execute(
            dir.path(),
            AddOptions {
                name: &n,
                host: &h,
                url: &u,
                entrypoint: "web",
                tls: true,
                cert_resolver: "letsencrypt",
                middlewares: Some("auth"),
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("rt.yml")).unwrap();
        let parsed: crate::config::TraefikDynamicConfig =
            serde_yaml::from_str(&content).unwrap();
        assert_eq!(parsed.route_name(), Some("rt"));
        assert_eq!(parsed.host(), Some("rt.example.com".to_string()));
        assert_eq!(parsed.backend_url(), Some("http://rt:80"));
    }
}
