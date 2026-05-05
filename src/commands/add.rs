use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use colored::Colorize;

use crate::cli::{Protocol, ServicePreset};
use crate::config::{RouterTls, TraefikDynamicConfig};
use crate::validation::{validate_address, validate_host, validate_name, validate_url};

pub struct AddOptions<'a> {
    pub name: &'a str,
    pub host: Option<&'a str>,
    pub url: Option<&'a str>,
    pub address: Option<&'a str>,
    pub entrypoint: Option<&'a str>,
    pub protocol: Protocol,
    pub preset: Option<ServicePreset>,
    pub tls: bool,
    pub tls_passthrough: bool,
    pub cert_resolver: Option<&'a str>,
    pub middlewares: Option<&'a str>,
    pub force: bool,
    pub dry_run: bool,
}

pub fn execute(dir: &Path, opts: AddOptions) -> Result<()> {
    validate_name(opts.name)?;

    let file_path = dir.join(format!("{}.yml", opts.name));

    if file_path.exists() && !opts.force {
        bail!(
            "route {:?} already exists at {}. Use --force to overwrite.",
            opts.name,
            file_path.display()
        );
    }

    let (protocol, preset_defaults) = if let Some(preset) = &opts.preset {
        let defaults = preset.defaults();
        (defaults.protocol, Some(defaults))
    } else {
        (opts.protocol, None)
    };

    let config = match protocol {
        Protocol::Http => build_http_config(&opts, preset_defaults.as_ref())?,
        Protocol::Tcp => build_tcp_config(&opts, preset_defaults.as_ref())?,
        Protocol::Udp => build_udp_config(&opts, preset_defaults.as_ref())?,
    };

    let yaml = config
        .to_yaml()
        .context("failed to serialize config to YAML")?;

    if opts.dry_run {
        println!("{}", "--- dry-run: would write ---".yellow().bold());
        println!("{}: {}", "file".bold(), file_path.display());
        println!("{}", yaml);
        return Ok(());
    }

    if !dir.exists() {
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create directory {}", dir.display()))?;
    }

    fs::write(&file_path, &yaml)
        .with_context(|| format!("failed to write {}", file_path.display()))?;

    print_success(&opts, &protocol, &file_path, preset_defaults.as_ref());

    Ok(())
}

fn build_http_config(
    opts: &AddOptions,
    preset_defaults: Option<&crate::cli::PresetDefaults>,
) -> Result<TraefikDynamicConfig> {
    let host = opts
        .host
        .ok_or_else(|| anyhow::anyhow!("--host is required for HTTP routes"))?;
    let url = opts
        .url
        .ok_or_else(|| anyhow::anyhow!("--url is required for HTTP routes"))?;

    validate_host(host)?;
    validate_url(url)?;

    let entrypoint = opts
        .entrypoint
        .or(preset_defaults.map(|d| d.entrypoint))
        .unwrap_or("web");

    let mut entrypoints = vec![entrypoint.to_string()];
    if opts.tls && entrypoint == "web" {
        entrypoints.push("websecure".to_string());
    }

    let tls = if opts.tls {
        Some(RouterTls {
            cert_resolver: opts.cert_resolver.map(|s| s.to_string()),
            ..Default::default()
        })
    } else {
        None
    };

    let middlewares = opts.middlewares.map(|m| {
        m.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
    });

    Ok(TraefikDynamicConfig::new_http(
        opts.name,
        host,
        url,
        entrypoints,
        tls,
        middlewares,
    ))
}

fn build_tcp_config(
    opts: &AddOptions,
    preset_defaults: Option<&crate::cli::PresetDefaults>,
) -> Result<TraefikDynamicConfig> {
    let address = resolve_address(opts, preset_defaults)?;
    validate_address(&address)?;

    let host = opts.host;
    let entrypoint = opts
        .entrypoint
        .or(preset_defaults.map(|d| d.entrypoint))
        .ok_or_else(|| anyhow::anyhow!("--entrypoint is required for TCP routes (or use --preset)"))?;

    let rule = if let Some(h) = host {
        validate_host(h)?;
        format!("HostSNI(`{h}`)")
    } else {
        "HostSNI(`*`)".to_string()
    };

    let tls = if opts.tls || opts.tls_passthrough || host.is_some() {
        Some(crate::config::TcpRouterTls {
            passthrough: if opts.tls_passthrough {
                Some(true)
            } else {
                None
            },
            options: None,
            cert_resolver: opts.cert_resolver.map(|s| s.to_string()),
        })
    } else {
        None
    };

    Ok(TraefikDynamicConfig::new_tcp(
        opts.name,
        &rule,
        &address,
        vec![entrypoint.to_string()],
        tls,
    ))
}

fn build_udp_config(
    opts: &AddOptions,
    preset_defaults: Option<&crate::cli::PresetDefaults>,
) -> Result<TraefikDynamicConfig> {
    let address = resolve_address(opts, preset_defaults)?;
    validate_address(&address)?;

    let entrypoint = opts
        .entrypoint
        .or(preset_defaults.map(|d| d.entrypoint))
        .ok_or_else(|| anyhow::anyhow!("--entrypoint is required for UDP routes (or use --preset)"))?;

    Ok(TraefikDynamicConfig::new_udp(
        opts.name,
        &address,
        vec![entrypoint.to_string()],
    ))
}

fn resolve_address(
    opts: &AddOptions,
    preset_defaults: Option<&crate::cli::PresetDefaults>,
) -> Result<String> {
    if let Some(addr) = opts.address {
        return Ok(addr.to_string());
    }

    if let Some(url) = opts.url {
        if let Ok(parsed) = url::Url::parse(url) {
            let host = parsed.host_str().unwrap_or("127.0.0.1");
            let port = parsed.port().or(preset_defaults.map(|d| d.port));
            if let Some(p) = port {
                return Ok(format!("{host}:{p}"));
            }
        }
        return Ok(url.to_string());
    }

    if let Some(defaults) = preset_defaults {
        return Ok(format!("127.0.0.1:{}", defaults.port));
    }

    bail!("--address is required for TCP/UDP routes (or use --url or --preset)")
}

fn print_success(
    opts: &AddOptions,
    protocol: &Protocol,
    file_path: &Path,
    preset_defaults: Option<&crate::cli::PresetDefaults>,
) {
    let proto_badge = match protocol {
        Protocol::Http => "HTTP",
        Protocol::Tcp => "TCP",
        Protocol::Udp => "UDP",
    };

    let preset_badge = opts
        .preset
        .as_ref()
        .map(|p| format!(" ({})", format!("{p:?}").to_lowercase()))
        .unwrap_or_default();

    let target = opts
        .url
        .or(opts.address)
        .or(preset_defaults.map(|d| d.entrypoint))
        .unwrap_or("?");

    println!(
        "{} [{}]{} route {} → {}",
        "✓".green().bold(),
        proto_badge.cyan(),
        preset_badge.dimmed(),
        opts.name.cyan(),
        target.blue()
    );
    println!("  {}: {}", "file".dimmed(), file_path.display());
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn add_creates_http_route() {
        let dir = tmp();
        execute(
            dir.path(),
            AddOptions {
                name: "myapp",
                host: Some("app.example.com"),
                url: Some("http://127.0.0.1:3000"),
                address: None,
                entrypoint: Some("web"),
                protocol: Protocol::Http,
                preset: None,
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
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
        execute(
            dir.path(),
            AddOptions {
                name: "tls-app",
                host: Some("tls.example.com"),
                url: Some("http://back:443"),
                address: None,
                entrypoint: Some("web"),
                protocol: Protocol::Http,
                preset: None,
                tls: true,
                tls_passthrough: false,
                cert_resolver: Some("myresolver"),
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
        execute(
            dir.path(),
            AddOptions {
                name: "mw-app",
                host: Some("mw.example.com"),
                url: Some("http://back:80"),
                address: None,
                entrypoint: Some("web"),
                protocol: Protocol::Http,
                preset: None,
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
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
        execute(
            dir.path(),
            AddOptions {
                name: "dry",
                host: Some("dry.example.com"),
                url: Some("http://dry:80"),
                address: None,
                entrypoint: Some("web"),
                protocol: Protocol::Http,
                preset: None,
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
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
        let make_opts = || AddOptions {
            name: "dup",
            host: Some("dup.example.com"),
            url: Some("http://dup:80"),
            address: None,
            entrypoint: Some("web"),
            protocol: Protocol::Http,
            preset: None,
            tls: false,
            tls_passthrough: false,
            cert_resolver: None,
            middlewares: None,
            force: false,
            dry_run: false,
        };

        execute(dir.path(), make_opts()).unwrap();
        let err = execute(dir.path(), make_opts()).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn add_force_overwrites() {
        let dir = tmp();
        execute(
            dir.path(),
            AddOptions {
                name: "force",
                host: Some("force.example.com"),
                url: Some("http://force:80"),
                address: None,
                entrypoint: Some("web"),
                protocol: Protocol::Http,
                preset: None,
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        execute(
            dir.path(),
            AddOptions {
                name: "force",
                host: Some("force.example.com"),
                url: Some("http://force:9999"),
                address: None,
                entrypoint: Some("web"),
                protocol: Protocol::Http,
                preset: None,
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
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
        let err = execute(
            dir.path(),
            AddOptions {
                name: "bad name!",
                host: Some("ok.example.com"),
                url: Some("http://ok:80"),
                address: None,
                entrypoint: Some("web"),
                protocol: Protocol::Http,
                preset: None,
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
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
        let err = execute(
            dir.path(),
            AddOptions {
                name: "ok",
                host: Some("not valid host"),
                url: Some("http://ok:80"),
                address: None,
                entrypoint: Some("web"),
                protocol: Protocol::Http,
                preset: None,
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
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
        let err = execute(
            dir.path(),
            AddOptions {
                name: "ok",
                host: Some("ok.example.com"),
                url: Some("ftp://bad"),
                address: None,
                entrypoint: Some("web"),
                protocol: Protocol::Http,
                preset: None,
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
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
        execute(
            &nested,
            AddOptions {
                name: "nested",
                host: Some("n.example.com"),
                url: Some("http://n:80"),
                address: None,
                entrypoint: Some("web"),
                protocol: Protocol::Http,
                preset: None,
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
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
        execute(
            dir.path(),
            AddOptions {
                name: "cep",
                host: Some("cep.example.com"),
                url: Some("http://cep:80"),
                address: None,
                entrypoint: Some("custom-ep"),
                protocol: Protocol::Http,
                preset: None,
                tls: true,
                tls_passthrough: false,
                cert_resolver: None,
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
        execute(
            dir.path(),
            AddOptions {
                name: "rt",
                host: Some("rt.example.com"),
                url: Some("http://rt:80"),
                address: None,
                entrypoint: Some("web"),
                protocol: Protocol::Http,
                preset: None,
                tls: true,
                tls_passthrough: false,
                cert_resolver: Some("letsencrypt"),
                middlewares: Some("auth"),
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("rt.yml")).unwrap();
        let parsed: crate::config::TraefikDynamicConfig = serde_yaml::from_str(&content).unwrap();
        assert_eq!(parsed.route_name(), Some("rt"));
        assert_eq!(parsed.host(), Some("rt.example.com".to_string()));
        assert_eq!(parsed.backend_url(), Some("http://rt:80"));
    }

    #[test]
    fn add_tcp_route_basic() {
        let dir = tmp();
        execute(
            dir.path(),
            AddOptions {
                name: "postgres",
                host: None,
                url: None,
                address: Some("10.0.0.5:5432"),
                entrypoint: Some("postgres"),
                protocol: Protocol::Tcp,
                preset: None,
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("postgres.yml")).unwrap();
        assert!(content.contains("tcp:"));
        assert!(content.contains("HostSNI(`*`)"));
        assert!(content.contains("10.0.0.5:5432"));
        assert!(content.contains("postgres"));
    }

    #[test]
    fn add_tcp_with_sni_host() {
        let dir = tmp();
        execute(
            dir.path(),
            AddOptions {
                name: "db",
                host: Some("db.internal.local"),
                url: None,
                address: Some("10.0.0.5:5432"),
                entrypoint: Some("postgres"),
                protocol: Protocol::Tcp,
                preset: None,
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("db.yml")).unwrap();
        assert!(content.contains("HostSNI(`db.internal.local`)"));
    }

    #[test]
    fn add_tcp_with_tls_passthrough() {
        let dir = tmp();
        execute(
            dir.path(),
            AddOptions {
                name: "secure-db",
                host: Some("db.example.com"),
                url: None,
                address: Some("10.0.0.5:5432"),
                entrypoint: Some("postgres"),
                protocol: Protocol::Tcp,
                preset: None,
                tls: false,
                tls_passthrough: true,
                cert_resolver: None,
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("secure-db.yml")).unwrap();
        assert!(content.contains("passthrough: true"));
    }

    #[test]
    fn add_udp_route_basic() {
        let dir = tmp();
        execute(
            dir.path(),
            AddOptions {
                name: "dns",
                host: None,
                url: None,
                address: Some("10.0.0.2:53"),
                entrypoint: Some("dns"),
                protocol: Protocol::Udp,
                preset: None,
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("dns.yml")).unwrap();
        assert!(content.contains("udp:"));
        assert!(content.contains("10.0.0.2:53"));
        assert!(!content.contains("rule:"));
    }

    #[test]
    fn add_preset_postgres() {
        let dir = tmp();
        execute(
            dir.path(),
            AddOptions {
                name: "mydb",
                host: None,
                url: None,
                address: Some("10.0.0.5:5432"),
                entrypoint: None,
                protocol: Protocol::Http,
                preset: Some(ServicePreset::Postgres),
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("mydb.yml")).unwrap();
        assert!(content.contains("tcp:"));
        assert!(content.contains("10.0.0.5:5432"));
        assert!(content.contains("postgres"));
    }

    #[test]
    fn add_preset_dns_udp() {
        let dir = tmp();
        execute(
            dir.path(),
            AddOptions {
                name: "mydns",
                host: None,
                url: None,
                address: Some("10.0.0.2:53"),
                entrypoint: None,
                protocol: Protocol::Http,
                preset: Some(ServicePreset::Dns),
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("mydns.yml")).unwrap();
        assert!(content.contains("udp:"));
        assert!(content.contains("10.0.0.2:53"));
    }

    #[test]
    fn add_preset_uses_default_address() {
        let dir = tmp();
        execute(
            dir.path(),
            AddOptions {
                name: "redis-cache",
                host: None,
                url: None,
                address: None,
                entrypoint: None,
                protocol: Protocol::Http,
                preset: Some(ServicePreset::Redis),
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(dir.path().join("redis-cache.yml")).unwrap();
        assert!(content.contains("tcp:"));
        assert!(content.contains("127.0.0.1:6379"));
    }

    #[test]
    fn add_http_requires_host() {
        let dir = tmp();
        let err = execute(
            dir.path(),
            AddOptions {
                name: "no-host",
                host: None,
                url: Some("http://back:80"),
                address: None,
                entrypoint: Some("web"),
                protocol: Protocol::Http,
                preset: None,
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("--host is required"));
    }

    #[test]
    fn add_tcp_requires_entrypoint_without_preset() {
        let dir = tmp();
        let err = execute(
            dir.path(),
            AddOptions {
                name: "no-ep",
                host: None,
                url: None,
                address: Some("10.0.0.1:5432"),
                entrypoint: None,
                protocol: Protocol::Tcp,
                preset: None,
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("--entrypoint is required"));
    }

    #[test]
    fn add_tcp_validates_address() {
        let dir = tmp();
        let err = execute(
            dir.path(),
            AddOptions {
                name: "bad-addr",
                host: None,
                url: None,
                address: Some("not-valid"),
                entrypoint: Some("postgres"),
                protocol: Protocol::Tcp,
                preset: None,
                tls: false,
                tls_passthrough: false,
                cert_resolver: None,
                middlewares: None,
                force: false,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("host:port"));
    }
}
