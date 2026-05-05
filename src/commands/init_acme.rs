use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use colored::Colorize;

use crate::config::{
    AcmeConfig, CertificateResolver, DnsChallenge, DnsPropagation, TraefikStaticConfig,
};

const LE_STAGING: &str = "https://acme-staging-v02.api.letsencrypt.org/directory";

pub struct InitAcmeOptions<'a> {
    pub resolver_name: &'a str,
    pub email: &'a str,
    pub provider: &'a str,
    pub staging: bool,
    pub storage: &'a str,
    pub dns_resolvers: &'a [String],
    pub key_type: Option<&'a str>,
    pub propagation_delay: Option<u64>,
    pub disable_propagation_check: bool,
    pub force: bool,
    pub dry_run: bool,
    pub traefik_config: Option<&'a Path>,
}

struct KnownProvider {
    env_vars: &'static [&'static str],
}

fn known_providers() -> BTreeMap<&'static str, KnownProvider> {
    let mut m = BTreeMap::new();
    m.insert(
        "cloudflare",
        KnownProvider {
            env_vars: &["CF_DNS_API_TOKEN"],
        },
    );
    m.insert(
        "route53",
        KnownProvider {
            env_vars: &[
                "AWS_ACCESS_KEY_ID",
                "AWS_SECRET_ACCESS_KEY",
                "AWS_REGION",
            ],
        },
    );
    m.insert(
        "digitalocean",
        KnownProvider {
            env_vars: &["DO_AUTH_TOKEN"],
        },
    );
    m.insert(
        "hetzner",
        KnownProvider {
            env_vars: &["HETZNER_API_KEY"],
        },
    );
    m.insert(
        "ovh",
        KnownProvider {
            env_vars: &[
                "OVH_ENDPOINT",
                "OVH_APPLICATION_KEY",
                "OVH_APPLICATION_SECRET",
                "OVH_CONSUMER_KEY",
            ],
        },
    );
    m.insert(
        "gandiv5",
        KnownProvider {
            env_vars: &["GANDIV5_PERSONAL_ACCESS_TOKEN"],
        },
    );
    m.insert(
        "gcloud",
        KnownProvider {
            env_vars: &["GCE_PROJECT"],
        },
    );
    m.insert(
        "azuredns",
        KnownProvider {
            env_vars: &["AZURE_CLIENT_ID", "AZURE_TENANT_ID", "AZURE_CLIENT_SECRET"],
        },
    );
    m
}

fn warn_missing_env_vars(provider: &str) {
    let providers = known_providers();
    if let Some(kp) = providers.get(provider) {
        let missing: Vec<&&str> = kp
            .env_vars
            .iter()
            .filter(|v| std::env::var(v).is_err())
            .collect();
        if !missing.is_empty() {
            eprintln!(
                "  {} provider {} expects env vars: {}",
                "⚠".yellow().bold(),
                provider.cyan(),
                missing
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
                    .red()
            );
            eprintln!(
                "    {} these must be set in Traefik's environment (systemd unit, docker-compose, etc.)",
                "→".dimmed()
            );
        }
    }
}

pub fn execute(opts: InitAcmeOptions) -> Result<()> {
    let config_path = crate::commands::doctor::find_traefik_config(opts.traefik_config);
    let config_path = match config_path {
        Some(p) => p,
        None => {
            let default = opts
                .traefik_config
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::path::PathBuf::from("/etc/traefik/traefik.yml"));
            if opts.dry_run {
                println!(
                    "{} traefik config not found at {}",
                    "⚠".yellow().bold(),
                    default.display()
                );
                println!("  would create it with ACME resolver");
            }
            default
        }
    };

    let mut static_config = if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("failed to read {}", config_path.display()))?;
        serde_yaml::from_str::<TraefikStaticConfig>(&content)
            .with_context(|| format!("failed to parse {}", config_path.display()))?
    } else {
        TraefikStaticConfig::default()
    };

    if let Some(ref resolvers) = static_config.certificates_resolvers {
        if resolvers.contains_key(opts.resolver_name) && !opts.force {
            bail!(
                "resolver {:?} already exists. Use --force to overwrite.",
                opts.resolver_name
            );
        }
    }

    let ca_server = if opts.staging {
        Some(LE_STAGING.to_string())
    } else {
        None
    };

    let resolvers_list = if opts.dns_resolvers.is_empty() {
        None
    } else {
        Some(opts.dns_resolvers.to_vec())
    };

    let propagation = if opts.propagation_delay.is_some() || opts.disable_propagation_check {
        Some(DnsPropagation {
            delay_before_checks: opts.propagation_delay,
            disable_checks: if opts.disable_propagation_check {
                Some(true)
            } else {
                None
            },
        })
    } else {
        None
    };

    let resolver = CertificateResolver {
        acme: AcmeConfig {
            email: opts.email.to_string(),
            storage: Some(opts.storage.to_string()),
            ca_server,
            key_type: opts.key_type.map(|s| s.to_string()),
            dns_challenge: Some(DnsChallenge {
                provider: opts.provider.to_string(),
                resolvers: resolvers_list,
                propagation,
            }),
        },
    };

    let resolvers = static_config
        .certificates_resolvers
        .get_or_insert_with(BTreeMap::new);
    resolvers.insert(opts.resolver_name.to_string(), resolver);

    let yaml =
        serde_yaml::to_string(&static_config).context("failed to serialize traefik config")?;

    if opts.dry_run {
        println!("{}", "--- dry-run: would write ---".yellow().bold());
        println!("{}: {}", "file".bold(), config_path.display());
        println!("{yaml}");
        warn_missing_env_vars(opts.provider);
        return Ok(());
    }

    if let Some(parent) = config_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
    }

    fs::write(&config_path, &yaml)
        .with_context(|| format!("failed to write {}", config_path.display()))?;

    let mode = if opts.staging { "staging" } else { "production" };
    println!(
        "{} ACME resolver {} configured ({}, provider: {}, DNS-01)",
        "✓".green().bold(),
        opts.resolver_name.cyan(),
        mode.yellow(),
        opts.provider.blue()
    );
    println!("  {}: {}", "config".dimmed(), config_path.display());
    println!("  {}: {}", "storage".dimmed(), opts.storage);

    warn_missing_env_vars(opts.provider);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn init_acme_creates_resolver() {
        let dir = tmp();
        let cfg_path = dir.path().join("traefik.yml");
        fs::write(&cfg_path, "providers: {}\n").unwrap();

        execute(InitAcmeOptions {
            resolver_name: "letsencrypt",
            email: "admin@example.com",
            provider: "cloudflare",
            staging: false,
            storage: "/etc/traefik/acme.json",
            dns_resolvers: &[],
            key_type: None,
            propagation_delay: None,
            disable_propagation_check: false,
            force: false,
            dry_run: false,
            traefik_config: Some(cfg_path.as_path()),
        })
        .unwrap();

        let content = fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains("certificatesResolvers:"));
        assert!(content.contains("letsencrypt:"));
        assert!(content.contains("admin@example.com"));
        assert!(content.contains("cloudflare"));
        assert!(content.contains("dnsChallenge:"));
        assert!(!content.contains("caServer:"));
    }

    #[test]
    fn init_acme_staging_sets_ca_server() {
        let dir = tmp();
        let cfg_path = dir.path().join("traefik.yml");
        fs::write(&cfg_path, "{}\n").unwrap();

        execute(InitAcmeOptions {
            resolver_name: "le-staging",
            email: "test@test.com",
            provider: "digitalocean",
            staging: true,
            storage: "acme.json",
            dns_resolvers: &[],
            key_type: None,
            propagation_delay: None,
            disable_propagation_check: false,
            force: false,
            dry_run: false,
            traefik_config: Some(cfg_path.as_path()),
        })
        .unwrap();

        let content = fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains("caServer:"));
        assert!(content.contains("acme-staging-v02"));
    }

    #[test]
    fn init_acme_with_dns_resolvers() {
        let dir = tmp();
        let cfg_path = dir.path().join("traefik.yml");
        fs::write(&cfg_path, "{}\n").unwrap();

        execute(InitAcmeOptions {
            resolver_name: "myresolver",
            email: "a@b.com",
            provider: "hetzner",
            staging: false,
            storage: "acme.json",
            dns_resolvers: &["1.1.1.1:53".to_string(), "8.8.8.8:53".to_string()],
            key_type: Some("EC256"),
            propagation_delay: Some(30),
            disable_propagation_check: false,
            force: false,
            dry_run: false,
            traefik_config: Some(cfg_path.as_path()),
        })
        .unwrap();

        let content = fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains("1.1.1.1:53"));
        assert!(content.contains("8.8.8.8:53"));
        assert!(content.contains("keyType: EC256"));
        assert!(content.contains("delayBeforeChecks: 30"));
    }

    #[test]
    fn init_acme_rejects_duplicate_without_force() {
        let dir = tmp();
        let cfg_path = dir.path().join("traefik.yml");
        fs::write(&cfg_path, "{}\n").unwrap();

        execute(InitAcmeOptions {
            resolver_name: "dup",
            email: "a@b.com",
            provider: "cloudflare",
            staging: false,
            storage: "acme.json",
            dns_resolvers: &[],
            key_type: None,
            propagation_delay: None,
            disable_propagation_check: false,
            force: false,
            dry_run: false,
            traefik_config: Some(cfg_path.as_path()),
        })
        .unwrap();

        let err = execute(InitAcmeOptions {
            resolver_name: "dup",
            email: "a@b.com",
            provider: "cloudflare",
            staging: false,
            storage: "acme.json",
            dns_resolvers: &[],
            key_type: None,
            propagation_delay: None,
            disable_propagation_check: false,
            force: false,
            dry_run: false,
            traefik_config: Some(cfg_path.as_path()),
        })
        .unwrap_err();

        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn init_acme_force_overwrites() {
        let dir = tmp();
        let cfg_path = dir.path().join("traefik.yml");
        fs::write(&cfg_path, "{}\n").unwrap();

        execute(InitAcmeOptions {
            resolver_name: "le",
            email: "old@test.com",
            provider: "cloudflare",
            staging: false,
            storage: "acme.json",
            dns_resolvers: &[],
            key_type: None,
            propagation_delay: None,
            disable_propagation_check: false,
            force: false,
            dry_run: false,
            traefik_config: Some(cfg_path.as_path()),
        })
        .unwrap();

        execute(InitAcmeOptions {
            resolver_name: "le",
            email: "new@test.com",
            provider: "route53",
            staging: true,
            storage: "acme.json",
            dns_resolvers: &[],
            key_type: None,
            propagation_delay: None,
            disable_propagation_check: false,
            force: true,
            dry_run: false,
            traefik_config: Some(cfg_path.as_path()),
        })
        .unwrap();

        let content = fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains("new@test.com"));
        assert!(content.contains("route53"));
        assert!(!content.contains("old@test.com"));
    }

    #[test]
    fn init_acme_dry_run_no_writes() {
        let dir = tmp();
        let cfg_path = dir.path().join("traefik.yml");
        fs::write(&cfg_path, "{}\n").unwrap();
        let original = fs::read_to_string(&cfg_path).unwrap();

        execute(InitAcmeOptions {
            resolver_name: "le",
            email: "a@b.com",
            provider: "cloudflare",
            staging: false,
            storage: "acme.json",
            dns_resolvers: &[],
            key_type: None,
            propagation_delay: None,
            disable_propagation_check: false,
            force: false,
            dry_run: true,
            traefik_config: Some(cfg_path.as_path()),
        })
        .unwrap();

        let after = fs::read_to_string(&cfg_path).unwrap();
        assert_eq!(original, after);
    }

    #[test]
    fn init_acme_preserves_existing_config() {
        let dir = tmp();
        let cfg_path = dir.path().join("traefik.yml");
        fs::write(
            &cfg_path,
            "entryPoints:\n  web:\n    address: ':80'\nproviders:\n  file:\n    directory: /etc/traefik/conf.d\n",
        )
        .unwrap();

        execute(InitAcmeOptions {
            resolver_name: "le",
            email: "a@b.com",
            provider: "cloudflare",
            staging: false,
            storage: "acme.json",
            dns_resolvers: &[],
            key_type: None,
            propagation_delay: None,
            disable_propagation_check: false,
            force: false,
            dry_run: false,
            traefik_config: Some(cfg_path.as_path()),
        })
        .unwrap();

        let content = fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains("entryPoints:"));
        assert!(content.contains("providers:"));
        assert!(content.contains("certificatesResolvers:"));
    }

    #[test]
    fn init_acme_creates_config_from_scratch() {
        let dir = tmp();
        let cfg_path = dir.path().join("traefik.yml");

        execute(InitAcmeOptions {
            resolver_name: "le",
            email: "a@b.com",
            provider: "cloudflare",
            staging: false,
            storage: "acme.json",
            dns_resolvers: &[],
            key_type: None,
            propagation_delay: None,
            disable_propagation_check: false,
            force: false,
            dry_run: false,
            traefik_config: Some(cfg_path.as_path()),
        })
        .unwrap();

        assert!(cfg_path.exists());
        let content = fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains("certificatesResolvers:"));
    }

    #[test]
    fn init_acme_disable_propagation_check() {
        let dir = tmp();
        let cfg_path = dir.path().join("traefik.yml");
        fs::write(&cfg_path, "{}\n").unwrap();

        execute(InitAcmeOptions {
            resolver_name: "le",
            email: "a@b.com",
            provider: "cloudflare",
            staging: false,
            storage: "acme.json",
            dns_resolvers: &[],
            key_type: None,
            propagation_delay: None,
            disable_propagation_check: true,
            force: false,
            dry_run: false,
            traefik_config: Some(cfg_path.as_path()),
        })
        .unwrap();

        let content = fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains("disableChecks: true"));
    }

    #[test]
    fn init_acme_yaml_roundtrip() {
        let dir = tmp();
        let cfg_path = dir.path().join("traefik.yml");
        fs::write(&cfg_path, "{}\n").unwrap();

        execute(InitAcmeOptions {
            resolver_name: "letsencrypt",
            email: "admin@example.com",
            provider: "cloudflare",
            staging: true,
            storage: "/etc/traefik/acme.json",
            dns_resolvers: &["1.1.1.1:53".to_string()],
            key_type: Some("EC384"),
            propagation_delay: Some(15),
            disable_propagation_check: false,
            force: false,
            dry_run: false,
            traefik_config: Some(cfg_path.as_path()),
        })
        .unwrap();

        let content = fs::read_to_string(&cfg_path).unwrap();
        let parsed: TraefikStaticConfig = serde_yaml::from_str(&content).unwrap();
        let resolvers = parsed.certificates_resolvers.unwrap();
        let le = resolvers.get("letsencrypt").unwrap();
        assert_eq!(le.acme.email, "admin@example.com");
        assert_eq!(
            le.acme.ca_server.as_deref(),
            Some("https://acme-staging-v02.api.letsencrypt.org/directory")
        );
        assert_eq!(le.acme.key_type.as_deref(), Some("EC384"));
        let dns = le.acme.dns_challenge.as_ref().unwrap();
        assert_eq!(dns.provider, "cloudflare");
        assert_eq!(dns.resolvers.as_ref().unwrap(), &["1.1.1.1:53"]);
        let prop = dns.propagation.as_ref().unwrap();
        assert_eq!(prop.delay_before_checks, Some(15));
    }
}
