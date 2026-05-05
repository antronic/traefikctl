use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use colored::Colorize;

use crate::config::{ClientAuth, TlsCertificate, TlsConfig, TlsOptions, TlsStore};

pub struct InitCaOptions<'a> {
    pub ca_cert: &'a str,
    pub intermediate_cert: Option<&'a str>,
    pub cert: &'a str,
    pub key: &'a str,
    pub certs_dir: &'a str,
    pub mtls: bool,
    pub min_version: Option<&'a str>,
    pub force: bool,
    pub dry_run: bool,
    pub conf_dir: &'a Path,
}

fn validate_pem_file(path: &str, label: &str) -> Result<()> {
    let p = Path::new(path);
    if !p.exists() {
        bail!("{label} not found: {path}");
    }
    let content = fs::read_to_string(p)
        .with_context(|| format!("failed to read {label}: {path}"))?;
    if !content.contains("-----BEGIN") {
        bail!("{label} does not appear to be a PEM file: {path}");
    }
    Ok(())
}

fn copy_cert_file(src: &str, dest: &Path, label: &str) -> Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory {}", parent.display()))?;
    }
    fs::copy(src, dest)
        .with_context(|| format!("failed to copy {label} from {src} to {}", dest.display()))?;
    Ok(())
}

pub fn execute(opts: InitCaOptions) -> Result<()> {
    validate_pem_file(opts.ca_cert, "CA certificate")?;
    validate_pem_file(opts.cert, "server certificate")?;
    validate_pem_file(opts.key, "server private key")?;
    if let Some(ic) = opts.intermediate_cert {
        validate_pem_file(ic, "intermediate CA certificate")?;
    }

    let certs_dir = PathBuf::from(opts.certs_dir);
    let ca_dir = certs_dir.join("ca");
    let tls_dir = opts.conf_dir.join("tls");
    let tls_config_file = tls_dir.join("tls-default.yml");

    if tls_config_file.exists() && !opts.force {
        bail!(
            "TLS config already exists at {}. Use --force to overwrite.",
            tls_config_file.display()
        );
    }

    let dest_ca_cert = ca_dir.join("root-ca.crt");
    let dest_intermediate = ca_dir.join("intermediate-ca.crt");
    let dest_cert = certs_dir.join("default.crt");
    let dest_key = certs_dir.join("default.key");

    if opts.dry_run {
        println!("{}", "--- dry-run: would do ---".yellow().bold());
        println!("  copy CA cert:     {} → {}", opts.ca_cert, dest_ca_cert.display());
        if let Some(ic) = opts.intermediate_cert {
            println!("  copy intermediate: {} → {}", ic, dest_intermediate.display());
        }
        println!("  copy server cert: {} → {}", opts.cert, dest_cert.display());
        println!("  copy server key:  {} → {}", opts.key, dest_key.display());
        println!("  write TLS config: {}", tls_config_file.display());
        let config = build_tls_dynamic_config(&certs_dir, opts.intermediate_cert.is_some(), opts.mtls, opts.min_version)?;
        let yaml = serde_yaml::to_string(&config)?;
        println!("{yaml}");
        return Ok(());
    }

    copy_cert_file(opts.ca_cert, &dest_ca_cert, "CA certificate")?;
    println!("  {} CA cert → {}", "✓".green(), dest_ca_cert.display());

    if let Some(ic) = opts.intermediate_cert {
        copy_cert_file(ic, &dest_intermediate, "intermediate CA certificate")?;
        println!("  {} intermediate CA → {}", "✓".green(), dest_intermediate.display());
    }

    copy_cert_file(opts.cert, &dest_cert, "server certificate")?;
    println!("  {} server cert → {}", "✓".green(), dest_cert.display());

    copy_cert_file(opts.key, &dest_key, "server private key")?;
    println!("  {} server key → {}", "✓".green(), dest_key.display());

    let config = build_tls_dynamic_config(&certs_dir, opts.intermediate_cert.is_some(), opts.mtls, opts.min_version)?;
    let yaml = serde_yaml::to_string(&config)?;

    fs::create_dir_all(&tls_dir)
        .with_context(|| format!("failed to create {}", tls_dir.display()))?;

    fs::write(&tls_config_file, &yaml)
        .with_context(|| format!("failed to write {}", tls_config_file.display()))?;

    println!(
        "{} TLS default config written to {}",
        "✓".green().bold(),
        tls_config_file.display()
    );

    Ok(())
}

fn build_tls_dynamic_config(
    certs_dir: &Path,
    has_intermediate: bool,
    mtls: bool,
    min_version: Option<&str>,
) -> Result<TlsDynamicConfig> {
    let cert_path = certs_dir.join("default.crt").to_string_lossy().to_string();
    let key_path = certs_dir.join("default.key").to_string_lossy().to_string();

    let default_cert = TlsCertificate {
        cert_file: cert_path.clone(),
        key_file: key_path.clone(),
    };

    let mut stores = std::collections::BTreeMap::new();
    stores.insert(
        "default".to_string(),
        TlsStore {
            default_certificate: Some(TlsCertificate {
                cert_file: cert_path,
                key_file: key_path,
            }),
        },
    );

    let options = if mtls || min_version.is_some() {
        let mut ca_files = vec![
            certs_dir.join("ca").join("root-ca.crt").to_string_lossy().to_string(),
        ];
        if has_intermediate {
            ca_files.push(
                certs_dir.join("ca").join("intermediate-ca.crt").to_string_lossy().to_string(),
            );
        }

        let client_auth = if mtls {
            Some(ClientAuth {
                ca_files,
                client_auth_type: "RequireAndVerifyClientCert".to_string(),
            })
        } else {
            None
        };

        let mut opts_map = std::collections::BTreeMap::new();
        opts_map.insert(
            "default".to_string(),
            TlsOptions {
                client_auth,
                min_version: min_version.map(|s| s.to_string()),
            },
        );
        Some(opts_map)
    } else {
        None
    };

    Ok(TlsDynamicConfig {
        tls: TlsConfig {
            certificates: Some(vec![default_cert]),
            stores: Some(stores),
            options,
        },
    })
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct TlsDynamicConfig {
    pub tls: TlsConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_pem(dir: &Path, name: &str) -> PathBuf {
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
    fn init_ca_basic() {
        let src_dir = tempfile::tempdir().unwrap();
        let certs_dir = tempfile::tempdir().unwrap();
        let conf_dir = tempfile::tempdir().unwrap();

        let ca = create_test_pem(src_dir.path(), "ca.crt");
        let cert = create_test_pem(src_dir.path(), "server.crt");
        let key = create_test_key(src_dir.path(), "server.key");

        execute(InitCaOptions {
            ca_cert: ca.to_str().unwrap(),
            intermediate_cert: None,
            cert: cert.to_str().unwrap(),
            key: key.to_str().unwrap(),
            certs_dir: certs_dir.path().to_str().unwrap(),
            mtls: false,
            min_version: None,
            force: false,
            dry_run: false,
            conf_dir: conf_dir.path(),
        })
        .unwrap();

        assert!(certs_dir.path().join("ca/root-ca.crt").exists());
        assert!(certs_dir.path().join("default.crt").exists());
        assert!(certs_dir.path().join("default.key").exists());
        assert!(conf_dir.path().join("tls/tls-default.yml").exists());

        let yaml = fs::read_to_string(conf_dir.path().join("tls/tls-default.yml")).unwrap();
        assert!(yaml.contains("certFile:"));
        assert!(yaml.contains("keyFile:"));
        assert!(yaml.contains("defaultCertificate:"));
    }

    #[test]
    fn init_ca_with_intermediate() {
        let src_dir = tempfile::tempdir().unwrap();
        let certs_dir = tempfile::tempdir().unwrap();
        let conf_dir = tempfile::tempdir().unwrap();

        let ca = create_test_pem(src_dir.path(), "ca.crt");
        let inter = create_test_pem(src_dir.path(), "intermediate.crt");
        let cert = create_test_pem(src_dir.path(), "server.crt");
        let key = create_test_key(src_dir.path(), "server.key");

        execute(InitCaOptions {
            ca_cert: ca.to_str().unwrap(),
            intermediate_cert: Some(inter.to_str().unwrap()),
            cert: cert.to_str().unwrap(),
            key: key.to_str().unwrap(),
            certs_dir: certs_dir.path().to_str().unwrap(),
            mtls: false,
            min_version: None,
            force: false,
            dry_run: false,
            conf_dir: conf_dir.path(),
        })
        .unwrap();

        assert!(certs_dir.path().join("ca/intermediate-ca.crt").exists());
    }

    #[test]
    fn init_ca_with_mtls() {
        let src_dir = tempfile::tempdir().unwrap();
        let certs_dir = tempfile::tempdir().unwrap();
        let conf_dir = tempfile::tempdir().unwrap();

        let ca = create_test_pem(src_dir.path(), "ca.crt");
        let cert = create_test_pem(src_dir.path(), "server.crt");
        let key = create_test_key(src_dir.path(), "server.key");

        execute(InitCaOptions {
            ca_cert: ca.to_str().unwrap(),
            intermediate_cert: None,
            cert: cert.to_str().unwrap(),
            key: key.to_str().unwrap(),
            certs_dir: certs_dir.path().to_str().unwrap(),
            mtls: true,
            min_version: Some("VersionTLS13"),
            force: false,
            dry_run: false,
            conf_dir: conf_dir.path(),
        })
        .unwrap();

        let yaml = fs::read_to_string(conf_dir.path().join("tls/tls-default.yml")).unwrap();
        assert!(yaml.contains("clientAuth:"));
        assert!(yaml.contains("caFiles:"));
        assert!(yaml.contains("RequireAndVerifyClientCert"));
        assert!(yaml.contains("minVersion: VersionTLS13"));
    }

    #[test]
    fn init_ca_dry_run_no_writes() {
        let src_dir = tempfile::tempdir().unwrap();
        let certs_dir = tempfile::tempdir().unwrap();
        let conf_dir = tempfile::tempdir().unwrap();

        let ca = create_test_pem(src_dir.path(), "ca.crt");
        let cert = create_test_pem(src_dir.path(), "server.crt");
        let key = create_test_key(src_dir.path(), "server.key");

        execute(InitCaOptions {
            ca_cert: ca.to_str().unwrap(),
            intermediate_cert: None,
            cert: cert.to_str().unwrap(),
            key: key.to_str().unwrap(),
            certs_dir: certs_dir.path().to_str().unwrap(),
            mtls: false,
            min_version: None,
            force: false,
            dry_run: true,
            conf_dir: conf_dir.path(),
        })
        .unwrap();

        assert!(!certs_dir.path().join("ca/root-ca.crt").exists());
        assert!(!conf_dir.path().join("tls/tls-default.yml").exists());
    }

    #[test]
    fn init_ca_rejects_duplicate_without_force() {
        let src_dir = tempfile::tempdir().unwrap();
        let certs_dir = tempfile::tempdir().unwrap();
        let conf_dir = tempfile::tempdir().unwrap();

        let ca = create_test_pem(src_dir.path(), "ca.crt");
        let cert = create_test_pem(src_dir.path(), "server.crt");
        let key = create_test_key(src_dir.path(), "server.key");

        let opts = || InitCaOptions {
            ca_cert: ca.to_str().unwrap(),
            intermediate_cert: None,
            cert: cert.to_str().unwrap(),
            key: key.to_str().unwrap(),
            certs_dir: certs_dir.path().to_str().unwrap(),
            mtls: false,
            min_version: None,
            force: false,
            dry_run: false,
            conf_dir: conf_dir.path(),
        };

        execute(opts()).unwrap();
        let err = execute(opts()).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn init_ca_force_overwrites() {
        let src_dir = tempfile::tempdir().unwrap();
        let certs_dir = tempfile::tempdir().unwrap();
        let conf_dir = tempfile::tempdir().unwrap();

        let ca = create_test_pem(src_dir.path(), "ca.crt");
        let cert = create_test_pem(src_dir.path(), "server.crt");
        let key = create_test_key(src_dir.path(), "server.key");

        execute(InitCaOptions {
            ca_cert: ca.to_str().unwrap(),
            intermediate_cert: None,
            cert: cert.to_str().unwrap(),
            key: key.to_str().unwrap(),
            certs_dir: certs_dir.path().to_str().unwrap(),
            mtls: false,
            min_version: None,
            force: false,
            dry_run: false,
            conf_dir: conf_dir.path(),
        })
        .unwrap();

        execute(InitCaOptions {
            ca_cert: ca.to_str().unwrap(),
            intermediate_cert: None,
            cert: cert.to_str().unwrap(),
            key: key.to_str().unwrap(),
            certs_dir: certs_dir.path().to_str().unwrap(),
            mtls: false,
            min_version: None,
            force: true,
            dry_run: false,
            conf_dir: conf_dir.path(),
        })
        .unwrap();

        assert!(conf_dir.path().join("tls/tls-default.yml").exists());
    }

    #[test]
    fn init_ca_validates_missing_ca() {
        let conf_dir = tempfile::tempdir().unwrap();
        let err = execute(InitCaOptions {
            ca_cert: "/nonexistent/ca.crt",
            intermediate_cert: None,
            cert: "/nonexistent/server.crt",
            key: "/nonexistent/server.key",
            certs_dir: "/tmp/certs",
            mtls: false,
            min_version: None,
            force: false,
            dry_run: false,
            conf_dir: conf_dir.path(),
        })
        .unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn init_ca_validates_non_pem() {
        let src_dir = tempfile::tempdir().unwrap();
        let conf_dir = tempfile::tempdir().unwrap();

        let bad = src_dir.path().join("bad.crt");
        fs::write(&bad, "not a pem file").unwrap();
        let cert = create_test_pem(src_dir.path(), "server.crt");
        let key = create_test_key(src_dir.path(), "server.key");

        let err = execute(InitCaOptions {
            ca_cert: bad.to_str().unwrap(),
            intermediate_cert: None,
            cert: cert.to_str().unwrap(),
            key: key.to_str().unwrap(),
            certs_dir: "/tmp/certs",
            mtls: false,
            min_version: None,
            force: false,
            dry_run: false,
            conf_dir: conf_dir.path(),
        })
        .unwrap_err();
        assert!(err.to_string().contains("PEM"));
    }

    #[test]
    fn tls_dynamic_config_yaml_roundtrip() {
        let config = build_tls_dynamic_config(
            Path::new("/etc/traefik/certs"),
            false,
            false,
            None,
        )
        .unwrap();

        let yaml = serde_yaml::to_string(&config).unwrap();
        let parsed: TlsDynamicConfig = serde_yaml::from_str(&yaml).unwrap();
        assert!(parsed.tls.certificates.is_some());
        assert!(parsed.tls.stores.is_some());
        assert!(parsed.tls.options.is_none());
    }

    #[test]
    fn tls_dynamic_config_yaml_camelcase() {
        let config = build_tls_dynamic_config(
            Path::new("/etc/traefik/certs"),
            true,
            true,
            Some("VersionTLS13"),
        )
        .unwrap();

        let yaml = serde_yaml::to_string(&config).unwrap();
        assert!(yaml.contains("certFile:"));
        assert!(yaml.contains("keyFile:"));
        assert!(yaml.contains("defaultCertificate:"));
        assert!(yaml.contains("clientAuth:"));
        assert!(yaml.contains("caFiles:"));
        assert!(yaml.contains("clientAuthType:"));
        assert!(yaml.contains("minVersion:"));
    }
}
