use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use colored::Colorize;

use crate::config::{RouterTls, TlsCertificate, TlsConfig, TraefikDynamicConfig};
use crate::validation::validate_name;

pub struct AddCertOptions<'a> {
    pub name: &'a str,
    pub cert: &'a str,
    pub key: &'a str,
    pub certs_dir: &'a str,
    pub force: bool,
    pub dry_run: bool,
}

pub fn execute(dir: &Path, opts: AddCertOptions) -> Result<()> {
    validate_name(opts.name)?;

    let route_file = dir.join(format!("{}.yml", opts.name));
    if !route_file.exists() {
        bail!(
            "route {:?} not found at {}. Create it first with 'add'.",
            opts.name,
            route_file.display()
        );
    }

    let cert_src = Path::new(opts.cert);
    let key_src = Path::new(opts.key);

    if !cert_src.exists() {
        bail!("certificate file not found: {}", opts.cert);
    }
    if !key_src.exists() {
        bail!("private key file not found: {}", opts.key);
    }

    let certs_dir = PathBuf::from(opts.certs_dir);
    let service_dir = certs_dir.join("services").join(opts.name);
    let dest_cert = service_dir.join(format!("{}.crt", opts.name));
    let dest_key = service_dir.join(format!("{}.key", opts.name));

    let content = fs::read_to_string(&route_file)
        .with_context(|| format!("failed to read {}", route_file.display()))?;
    let mut config: TraefikDynamicConfig = serde_yaml::from_str(&content)
        .with_context(|| format!("failed to parse {}", route_file.display()))?;

    if config.tls.is_some() && !opts.force {
        bail!(
            "route {:?} already has TLS certificates configured. Use --force to overwrite.",
            opts.name
        );
    }

    if opts.dry_run {
        println!("{}", "--- dry-run: would do ---".yellow().bold());
        println!("  copy cert: {} → {}", opts.cert, dest_cert.display());
        println!("  copy key:  {} → {}", opts.key, dest_key.display());
        println!("  update route: {} (add tls.certificates)", route_file.display());
        return Ok(());
    }

    fs::create_dir_all(&service_dir)
        .with_context(|| format!("failed to create {}", service_dir.display()))?;

    fs::copy(cert_src, &dest_cert)
        .with_context(|| format!("failed to copy cert to {}", dest_cert.display()))?;
    println!("  {} cert → {}", "✓".green(), dest_cert.display());

    fs::copy(key_src, &dest_key)
        .with_context(|| format!("failed to copy key to {}", dest_key.display()))?;
    println!("  {} key → {}", "✓".green(), dest_key.display());

    config.tls = Some(TlsConfig {
        certificates: Some(vec![TlsCertificate {
            cert_file: dest_cert.to_string_lossy().to_string(),
            key_file: dest_key.to_string_lossy().to_string(),
        }]),
        stores: None,
        options: None,
    });

    if let Some(router) = config
        .http
        .as_mut()
        .and_then(|h| h.routers.get_mut(opts.name))
    {
        if router.tls.is_none() {
            router.tls = Some(RouterTls::default());
            if !router.entry_points.contains(&"websecure".to_string()) {
                router.entry_points.push("websecure".to_string());
            }
        }
    }

    let yaml = serde_yaml::to_string(&config)?;
    fs::write(&route_file, &yaml)
        .with_context(|| format!("failed to write {}", route_file.display()))?;

    println!(
        "{} TLS certificate attached to route {}",
        "✓".green().bold(),
        opts.name.cyan()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{HttpConfig, LoadBalancer, Router, Server, Service};
    use std::collections::BTreeMap;

    fn create_route(dir: &Path, name: &str) {
        let config = TraefikDynamicConfig {
            http: Some(HttpConfig {
                routers: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        name.to_string(),
                        Router {
                            rule: format!("Host(`{name}.example.com`)"),
                            entry_points: vec!["web".to_string()],
                            service: name.to_string(),
                            tls: None,
                            middlewares: None,
                        },
                    );
                    m
                },
                services: {
                    let mut m = BTreeMap::new();
                    m.insert(
                        name.to_string(),
                        Service {
                            load_balancer: LoadBalancer {
                                servers: vec![Server {
                                    url: "http://127.0.0.1:3000".to_string(),
                                }],
                            },
                        },
                    );
                    m
                },
            }),
            tcp: None,
            udp: None,
            tls: None,
        };
        let yaml = serde_yaml::to_string(&config).unwrap();
        fs::write(dir.join(format!("{name}.yml")), yaml).unwrap();
    }

    fn create_test_cert(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, "-----BEGIN CERTIFICATE-----\ntest\n-----END CERTIFICATE-----\n").unwrap();
        path
    }

    fn create_test_key(dir: &Path, name: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, "-----BEGIN PRIVATE KEY-----\ntest\n-----END PRIVATE KEY-----\n").unwrap();
        path
    }

    #[test]
    fn add_cert_basic() {
        let conf_dir = tempfile::tempdir().unwrap();
        let src_dir = tempfile::tempdir().unwrap();
        let certs_dir = tempfile::tempdir().unwrap();

        create_route(conf_dir.path(), "myapp");
        let cert = create_test_cert(src_dir.path(), "myapp.crt");
        let key = create_test_key(src_dir.path(), "myapp.key");

        execute(
            conf_dir.path(),
            AddCertOptions {
                name: "myapp",
                cert: cert.to_str().unwrap(),
                key: key.to_str().unwrap(),
                certs_dir: certs_dir.path().to_str().unwrap(),
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        assert!(certs_dir.path().join("services/myapp/myapp.crt").exists());
        assert!(certs_dir.path().join("services/myapp/myapp.key").exists());

        let content = fs::read_to_string(conf_dir.path().join("myapp.yml")).unwrap();
        assert!(content.contains("certFile:"));
        assert!(content.contains("keyFile:"));
        assert!(content.contains("websecure"));
    }

    #[test]
    fn add_cert_route_not_found() {
        let conf_dir = tempfile::tempdir().unwrap();
        let err = execute(
            conf_dir.path(),
            AddCertOptions {
                name: "nonexistent",
                cert: "/fake/cert.crt",
                key: "/fake/key.key",
                certs_dir: "/fake/certs",
                force: false,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn add_cert_missing_cert_file() {
        let conf_dir = tempfile::tempdir().unwrap();
        create_route(conf_dir.path(), "app");

        let err = execute(
            conf_dir.path(),
            AddCertOptions {
                name: "app",
                cert: "/nonexistent/cert.crt",
                key: "/nonexistent/key.key",
                certs_dir: "/tmp/certs",
                force: false,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn add_cert_dry_run_no_writes() {
        let conf_dir = tempfile::tempdir().unwrap();
        let src_dir = tempfile::tempdir().unwrap();
        let certs_dir = tempfile::tempdir().unwrap();

        create_route(conf_dir.path(), "dryapp");
        let cert = create_test_cert(src_dir.path(), "dryapp.crt");
        let key = create_test_key(src_dir.path(), "dryapp.key");

        execute(
            conf_dir.path(),
            AddCertOptions {
                name: "dryapp",
                cert: cert.to_str().unwrap(),
                key: key.to_str().unwrap(),
                certs_dir: certs_dir.path().to_str().unwrap(),
                force: false,
                dry_run: true,
            },
        )
        .unwrap();

        assert!(!certs_dir.path().join("services/dryapp/dryapp.crt").exists());
        let content = fs::read_to_string(conf_dir.path().join("dryapp.yml")).unwrap();
        assert!(!content.contains("certFile:"));
    }

    #[test]
    fn add_cert_rejects_duplicate_without_force() {
        let conf_dir = tempfile::tempdir().unwrap();
        let src_dir = tempfile::tempdir().unwrap();
        let certs_dir = tempfile::tempdir().unwrap();

        create_route(conf_dir.path(), "dupapp");
        let cert = create_test_cert(src_dir.path(), "dupapp.crt");
        let key = create_test_key(src_dir.path(), "dupapp.key");

        execute(
            conf_dir.path(),
            AddCertOptions {
                name: "dupapp",
                cert: cert.to_str().unwrap(),
                key: key.to_str().unwrap(),
                certs_dir: certs_dir.path().to_str().unwrap(),
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        let err = execute(
            conf_dir.path(),
            AddCertOptions {
                name: "dupapp",
                cert: cert.to_str().unwrap(),
                key: key.to_str().unwrap(),
                certs_dir: certs_dir.path().to_str().unwrap(),
                force: false,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("already has TLS"));
    }

    #[test]
    fn add_cert_force_overwrites() {
        let conf_dir = tempfile::tempdir().unwrap();
        let src_dir = tempfile::tempdir().unwrap();
        let certs_dir = tempfile::tempdir().unwrap();

        create_route(conf_dir.path(), "forceapp");
        let cert = create_test_cert(src_dir.path(), "forceapp.crt");
        let key = create_test_key(src_dir.path(), "forceapp.key");

        execute(
            conf_dir.path(),
            AddCertOptions {
                name: "forceapp",
                cert: cert.to_str().unwrap(),
                key: key.to_str().unwrap(),
                certs_dir: certs_dir.path().to_str().unwrap(),
                force: false,
                dry_run: false,
            },
        )
        .unwrap();

        execute(
            conf_dir.path(),
            AddCertOptions {
                name: "forceapp",
                cert: cert.to_str().unwrap(),
                key: key.to_str().unwrap(),
                certs_dir: certs_dir.path().to_str().unwrap(),
                force: true,
                dry_run: false,
            },
        )
        .unwrap();

        let content = fs::read_to_string(conf_dir.path().join("forceapp.yml")).unwrap();
        assert!(content.contains("certFile:"));
    }

    #[test]
    fn add_cert_validates_name() {
        let conf_dir = tempfile::tempdir().unwrap();
        let err = execute(
            conf_dir.path(),
            AddCertOptions {
                name: "bad name!",
                cert: "/fake",
                key: "/fake",
                certs_dir: "/fake",
                force: false,
                dry_run: false,
            },
        )
        .unwrap_err();
        assert!(err.to_string().contains("invalid character"));
    }
}
