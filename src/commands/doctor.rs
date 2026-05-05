use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use colored::Colorize;
use serde_yaml::Value;

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

    // 0. Detect self-signed CA
    let has_self_signed_ca = dir.join("tls").join("tls-default.yml").exists();

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
            let mut config_modified = false;

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
                    config_modified = true;
                    println!(
                        "  {} updated {} → providers.file.directory: {}",
                        "✓".green().bold(),
                        path.display(),
                        dir_str.cyan()
                    );
                }
            }

            // 3. Check entrypoint-level certResolver conflicts with self-signed CA
            if has_self_signed_ca {
                let conflicting =
                    find_entrypoint_cert_resolvers(&config.rest);
                if !conflicting.is_empty() {
                    for (ep_name, resolver_name) in &conflicting {
                        println!(
                            "  {} entrypoint {} has certResolver: {} (overrides per-route TLS)",
                            "✗".red().bold(),
                            ep_name.yellow(),
                            resolver_name.red()
                        );
                    }
                    if dry_run {
                        println!(
                            "  {} would remove entrypoint-level certResolver(s) to use self-signed CA default",
                            "○".yellow()
                        );
                        all_ok = false;
                    } else {
                        remove_entrypoint_cert_resolvers(&mut config.rest);
                        config_modified = true;
                        println!(
                            "  {} removed entrypoint-level certResolver(s) — routes will use default TLS store",
                            "✓".green().bold()
                        );
                    }
                } else {
                    println!(
                        "  {} no entrypoint-level certResolver conflicts",
                        "✓".green().bold()
                    );
                }
            }

            if config_modified {
                let yaml = serde_yaml::to_string(&config)
                    .context("failed to serialize traefik config")?;
                fs::write(&path, &yaml)
                    .with_context(|| format!("failed to write {}", path.display()))?;
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
                    certificates_resolvers: None,
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
    check_acme_resolvers(traefik_config);

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

fn check_acme_resolvers(traefik_config: Option<&Path>) {
    let config_path = find_traefik_config(traefik_config);
    let config_path = match config_path {
        Some(p) => p,
        None => return,
    };

    let content = match fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let config: TraefikStaticConfig = match serde_yaml::from_str(&content) {
        Ok(c) => c,
        Err(_) => return,
    };

    match &config.certificates_resolvers {
        Some(resolvers) if !resolvers.is_empty() => {
            for (name, resolver) in resolvers {
                let challenge = resolver
                    .acme
                    .dns_challenge
                    .as_ref()
                    .map(|d| format!("DNS-01/{}", d.provider))
                    .unwrap_or_else(|| "unknown".to_string());

                let mode = if resolver
                    .acme
                    .ca_server
                    .as_deref()
                    .map(|s| s.contains("staging"))
                    .unwrap_or(false)
                {
                    "staging".yellow().to_string()
                } else {
                    "production".green().to_string()
                };

                println!(
                    "  {} ACME resolver: {} ({}, {})",
                    "✓".green().bold(),
                    name.cyan(),
                    challenge,
                    mode
                );
            }
        }
        _ => {
            println!(
                "  {} no ACME certificate resolvers configured",
                "–".dimmed()
            );
            println!(
                "    {} run `traefikctl init-acme` to set up DNS-01 challenge",
                "→".dimmed()
            );
        }
    }
}

fn find_entrypoint_cert_resolvers(
    rest: &BTreeMap<String, Value>,
) -> Vec<(String, String)> {
    let mut conflicts = Vec::new();
    let entry_points = match rest.get("entryPoints") {
        Some(Value::Mapping(m)) => m,
        _ => return conflicts,
    };
    for (ep_key, ep_val) in entry_points {
        let ep_name = match ep_key.as_str() {
            Some(s) => s.to_string(),
            None => continue,
        };
        let resolver = ep_val
            .get("http")
            .and_then(|h| h.get("tls"))
            .and_then(|t| t.get("certResolver"))
            .and_then(|v| v.as_str());
        if let Some(r) = resolver {
            conflicts.push((ep_name, r.to_string()));
        }
    }
    conflicts
}

fn remove_entrypoint_cert_resolvers(rest: &mut BTreeMap<String, Value>) {
    let entry_points = match rest.get_mut("entryPoints") {
        Some(Value::Mapping(m)) => m,
        _ => return,
    };
    for (_ep_key, ep_val) in entry_points.iter_mut() {
        let has_resolver = ep_val
            .get("http")
            .and_then(|h| h.get("tls"))
            .and_then(|t| t.get("certResolver"))
            .is_some();
        if !has_resolver {
            continue;
        }

        let http = match ep_val.get_mut("http") {
            Some(v) => v,
            None => continue,
        };
        let tls = match http.get_mut("tls") {
            Some(v) => v,
            None => continue,
        };
        if let Value::Mapping(tls_map) = tls {
            tls_map.remove(Value::String("certResolver".to_string()));
        }

        let tls_empty = http
            .get("tls")
            .and_then(|t| t.as_mapping())
            .is_some_and(|m| m.is_empty());
        if tls_empty {
            if let Value::Mapping(http_map) = http {
                http_map.remove(Value::String("tls".to_string()));
            }
        }

        let http_empty = ep_val
            .get("http")
            .and_then(|h| h.as_mapping())
            .is_some_and(|m| m.is_empty());
        if http_empty {
            if let Value::Mapping(ep_map) = ep_val {
                ep_map.remove(Value::String("http".to_string()));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn find_traefik_config_override_exists() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("traefik.yml");
        fs::write(&cfg_path, "providers: {}").unwrap();
        assert_eq!(
            find_traefik_config(Some(&cfg_path)),
            Some(cfg_path)
        );
    }

    #[test]
    fn find_traefik_config_override_missing() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_path = dir.path().join("nope.yml");
        assert_eq!(find_traefik_config(Some(&cfg_path)), None);
    }

    #[test]
    fn find_traefik_config_no_override_no_candidates() {
        assert_eq!(find_traefik_config(None), None);
    }

    #[test]
    fn ensure_setup_creates_dir() {
        let dir = tempfile::tempdir().unwrap();
        let conf_d = dir.path().join("conf.d");
        let cfg_path = dir.path().join("traefik.yml");

        ensure_setup(&conf_d, Some(&cfg_path), false).unwrap();

        assert!(conf_d.exists());
        assert!(cfg_path.exists());
    }

    #[test]
    fn ensure_setup_creates_config_from_scratch() {
        let dir = tempfile::tempdir().unwrap();
        let conf_d = dir.path().join("conf.d");
        let cfg_path = dir.path().join("traefik.yml");

        ensure_setup(&conf_d, Some(&cfg_path), false).unwrap();

        let content = fs::read_to_string(&cfg_path).unwrap();
        let parsed: TraefikStaticConfig = serde_yaml::from_str(&content).unwrap();
        let fp = parsed.providers.unwrap().file.unwrap();
        assert_eq!(fp.directory, Some(conf_d.to_string_lossy().to_string()));
        assert_eq!(fp.watch, Some(true));
    }

    #[test]
    fn ensure_setup_fixes_wrong_directory() {
        let dir = tempfile::tempdir().unwrap();
        let conf_d = dir.path().join("conf.d");
        fs::create_dir_all(&conf_d).unwrap();
        let cfg_path = dir.path().join("traefik.yml");

        let initial = "providers:\n  file:\n    directory: /wrong/path\n    watch: true\n";
        fs::write(&cfg_path, initial).unwrap();

        ensure_setup(&conf_d, Some(&cfg_path), false).unwrap();

        let content = fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains(&conf_d.to_string_lossy().to_string()));
    }

    #[test]
    fn ensure_setup_fixes_missing_watch() {
        let dir = tempfile::tempdir().unwrap();
        let conf_d = dir.path().join("conf.d");
        fs::create_dir_all(&conf_d).unwrap();
        let cfg_path = dir.path().join("traefik.yml");

        let initial = format!(
            "providers:\n  file:\n    directory: {}\n",
            conf_d.display()
        );
        fs::write(&cfg_path, &initial).unwrap();

        ensure_setup(&conf_d, Some(&cfg_path), false).unwrap();

        let content = fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains("watch: true"));
    }

    #[test]
    fn ensure_setup_all_ok_returns_true() {
        let dir = tempfile::tempdir().unwrap();
        let conf_d = dir.path().join("conf.d");
        fs::create_dir_all(&conf_d).unwrap();
        let cfg_path = dir.path().join("traefik.yml");

        let yaml = format!(
            "providers:\n  file:\n    directory: {}\n    watch: true\n",
            conf_d.display()
        );
        fs::write(&cfg_path, &yaml).unwrap();

        let result = ensure_setup(&conf_d, Some(&cfg_path), false).unwrap();
        assert!(result);
    }

    #[test]
    fn ensure_setup_dry_run_no_writes() {
        let dir = tempfile::tempdir().unwrap();
        let conf_d = dir.path().join("conf.d");
        let cfg_path = dir.path().join("traefik.yml");

        let result = ensure_setup(&conf_d, Some(&cfg_path), true).unwrap();
        assert!(!result);
        assert!(!conf_d.exists());
        assert!(!cfg_path.exists());
    }

    #[test]
    fn ensure_setup_preserves_existing_keys() {
        let dir = tempfile::tempdir().unwrap();
        let conf_d = dir.path().join("conf.d");
        fs::create_dir_all(&conf_d).unwrap();
        let cfg_path = dir.path().join("traefik.yml");

        let initial = "entryPoints:\n  web:\n    address: ':80'\nproviders:\n  file:\n    directory: /wrong\n";
        fs::write(&cfg_path, initial).unwrap();

        ensure_setup(&conf_d, Some(&cfg_path), false).unwrap();

        let content = fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains("entryPoints:"));
        assert!(content.contains("':80'") || content.contains(":80"));
    }

    #[test]
    fn execute_doctor_all_ok() {
        let dir = tempfile::tempdir().unwrap();
        let conf_d = dir.path().join("conf.d");
        fs::create_dir_all(&conf_d).unwrap();
        let cfg_path = dir.path().join("traefik.yml");
        let yaml = format!(
            "providers:\n  file:\n    directory: {}\n    watch: true\n",
            conf_d.display()
        );
        fs::write(&cfg_path, &yaml).unwrap();

        execute(&conf_d, Some(cfg_path.as_path()), false).unwrap();
    }

    fn setup_with_self_signed_ca(conf_d: &Path, cfg_path: &Path, traefik_yaml: &str) {
        fs::create_dir_all(conf_d).unwrap();
        let tls_dir = conf_d.join("tls");
        fs::create_dir_all(&tls_dir).unwrap();
        fs::write(tls_dir.join("tls-default.yml"), "tls: {}").unwrap();
        let yaml = traefik_yaml.replace("{DIR}", &conf_d.to_string_lossy());
        fs::write(cfg_path, yaml).unwrap();
    }

    #[test]
    fn find_entrypoint_cert_resolvers_detects_conflict() {
        let yaml = r#"
entryPoints:
  websecure:
    address: ':443'
    http:
      tls:
        certResolver: letsencrypt
  web:
    address: ':80'
"#;
        let parsed: TraefikStaticConfig = serde_yaml::from_str(yaml).unwrap();
        let conflicts = find_entrypoint_cert_resolvers(&parsed.rest);
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].0, "websecure");
        assert_eq!(conflicts[0].1, "letsencrypt");
    }

    #[test]
    fn find_entrypoint_cert_resolvers_no_conflict() {
        let yaml = r#"
entryPoints:
  websecure:
    address: ':443'
  web:
    address: ':80'
"#;
        let parsed: TraefikStaticConfig = serde_yaml::from_str(yaml).unwrap();
        let conflicts = find_entrypoint_cert_resolvers(&parsed.rest);
        assert!(conflicts.is_empty());
    }

    #[test]
    fn find_entrypoint_cert_resolvers_multiple() {
        let yaml = r#"
entryPoints:
  websecure:
    address: ':443'
    http:
      tls:
        certResolver: letsencrypt
  internal:
    address: ':8443'
    http:
      tls:
        certResolver: internal-ca
"#;
        let parsed: TraefikStaticConfig = serde_yaml::from_str(yaml).unwrap();
        let conflicts = find_entrypoint_cert_resolvers(&parsed.rest);
        assert_eq!(conflicts.len(), 2);
    }

    #[test]
    fn remove_entrypoint_cert_resolvers_cleans_up() {
        let yaml = r#"
entryPoints:
  websecure:
    address: ':443'
    http:
      tls:
        certResolver: letsencrypt
  web:
    address: ':80'
    http:
      redirections:
        entryPoint:
          to: websecure
          scheme: https
"#;
        let mut parsed: TraefikStaticConfig = serde_yaml::from_str(yaml).unwrap();
        remove_entrypoint_cert_resolvers(&mut parsed.rest);

        let conflicts = find_entrypoint_cert_resolvers(&parsed.rest);
        assert!(conflicts.is_empty());

        let serialized = serde_yaml::to_string(&parsed).unwrap();
        assert!(!serialized.contains("certResolver"));
        assert!(serialized.contains(":443"));
        assert!(serialized.contains("redirections:"));
    }

    #[test]
    fn remove_entrypoint_cert_resolvers_preserves_other_tls_keys() {
        let yaml = r#"
entryPoints:
  websecure:
    address: ':443'
    http:
      tls:
        certResolver: letsencrypt
        options: myoptions
"#;
        let mut parsed: TraefikStaticConfig = serde_yaml::from_str(yaml).unwrap();
        remove_entrypoint_cert_resolvers(&mut parsed.rest);

        let serialized = serde_yaml::to_string(&parsed).unwrap();
        assert!(!serialized.contains("certResolver"));
        assert!(serialized.contains("options: myoptions"));
    }

    #[test]
    fn ensure_setup_fixes_cert_resolver_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let conf_d = dir.path().join("conf.d");
        let cfg_path = dir.path().join("traefik.yml");

        let traefik_yaml = r#"providers:
  file:
    directory: {DIR}
    watch: true
entryPoints:
  websecure:
    address: ':443'
    http:
      tls:
        certResolver: letsencrypt
"#;
        setup_with_self_signed_ca(&conf_d, &cfg_path, traefik_yaml);

        ensure_setup(&conf_d, Some(&cfg_path), false).unwrap();

        let content = fs::read_to_string(&cfg_path).unwrap();
        assert!(!content.contains("certResolver"));
        assert!(content.contains(":443"));
    }

    #[test]
    fn ensure_setup_no_fix_without_self_signed_ca() {
        let dir = tempfile::tempdir().unwrap();
        let conf_d = dir.path().join("conf.d");
        fs::create_dir_all(&conf_d).unwrap();
        let cfg_path = dir.path().join("traefik.yml");

        let yaml = format!(
            "providers:\n  file:\n    directory: {}\n    watch: true\nentryPoints:\n  websecure:\n    address: ':443'\n    http:\n      tls:\n        certResolver: letsencrypt\n",
            conf_d.display()
        );
        fs::write(&cfg_path, &yaml).unwrap();

        ensure_setup(&conf_d, Some(&cfg_path), false).unwrap();

        let content = fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains("certResolver"));
    }

    #[test]
    fn ensure_setup_dry_run_reports_cert_resolver_conflict() {
        let dir = tempfile::tempdir().unwrap();
        let conf_d = dir.path().join("conf.d");
        let cfg_path = dir.path().join("traefik.yml");

        let traefik_yaml = r#"providers:
  file:
    directory: {DIR}
    watch: true
entryPoints:
  websecure:
    address: ':443'
    http:
      tls:
        certResolver: letsencrypt
"#;
        setup_with_self_signed_ca(&conf_d, &cfg_path, traefik_yaml);

        let result = ensure_setup(&conf_d, Some(&cfg_path), true).unwrap();
        assert!(!result);

        let content = fs::read_to_string(&cfg_path).unwrap();
        assert!(content.contains("certResolver"));
    }
}
