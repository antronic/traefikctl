use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{bail, Context, Result};
use colored::Colorize;

use crate::cli::MiddlewareType;
use crate::config::{
    BasicAuthMiddleware, CompressMiddleware, HeadersMiddleware, MiddlewareDefinition,
    MiddlewareDynamicConfig, RateLimitMiddleware, RedirectSchemeMiddleware, StripPrefixMiddleware,
};
use crate::validation::validate_name;

pub struct AddMiddlewareOptions<'a> {
    pub name: &'a str,
    pub mw_type: &'a MiddlewareType,
    pub security_preset: bool,
    pub sts_seconds: Option<u64>,
    pub frame_deny: Option<bool>,
    pub referrer_policy: Option<&'a str>,
    pub response_headers: &'a [String],
    pub request_headers: &'a [String],
    pub average: Option<u64>,
    pub burst: Option<u64>,
    pub period: Option<&'a str>,
    pub scheme: Option<&'a str>,
    pub permanent: Option<bool>,
    pub users: &'a [String],
    pub realm: Option<&'a str>,
    pub prefixes: &'a [String],
    pub force: bool,
    pub dry_run: bool,
}

fn parse_kv_pairs(pairs: &[String]) -> Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    for pair in pairs {
        let (k, v) = pair
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("invalid header format {pair:?}, expected KEY=VALUE"))?;
        map.insert(k.trim().to_string(), v.trim().to_string());
    }
    Ok(map)
}

fn build_definition(opts: &AddMiddlewareOptions) -> Result<MiddlewareDefinition> {
    match opts.mw_type {
        MiddlewareType::Headers => {
            let mut h = if opts.security_preset {
                HeadersMiddleware::security_preset()
            } else {
                HeadersMiddleware {
                    sts_seconds: None,
                    sts_include_subdomains: None,
                    sts_preload: None,
                    frame_deny: None,
                    content_type_nosniff: None,
                    browser_xss_filter: None,
                    referrer_policy: None,
                    custom_response_headers: None,
                    custom_request_headers: None,
                    access_control_allow_methods: None,
                    access_control_allow_headers: None,
                    access_control_allow_origin_list: None,
                    access_control_max_age: None,
                }
            };

            if let Some(v) = opts.sts_seconds {
                h.sts_seconds = Some(v);
            }
            if let Some(v) = opts.frame_deny {
                h.frame_deny = Some(v);
            }
            if let Some(v) = &opts.referrer_policy {
                h.referrer_policy = Some(v.to_string());
            }

            if !opts.response_headers.is_empty() {
                let existing = h.custom_response_headers.get_or_insert_with(BTreeMap::new);
                existing.extend(parse_kv_pairs(opts.response_headers)?);
            }
            if !opts.request_headers.is_empty() {
                let existing = h.custom_request_headers.get_or_insert_with(BTreeMap::new);
                existing.extend(parse_kv_pairs(opts.request_headers)?);
            }

            Ok(MiddlewareDefinition::headers(h))
        }

        MiddlewareType::RateLimit => {
            let average = opts.average.ok_or_else(|| {
                anyhow::anyhow!("--average is required for rate-limit middleware")
            })?;
            Ok(MiddlewareDefinition::rate_limit(RateLimitMiddleware {
                average,
                burst: opts.burst,
                period: opts.period.map(|s| s.to_string()),
            }))
        }

        MiddlewareType::RedirectScheme => {
            let scheme = opts.scheme.ok_or_else(|| {
                anyhow::anyhow!("--scheme is required for redirect-scheme middleware")
            })?;
            Ok(MiddlewareDefinition::redirect_scheme(
                RedirectSchemeMiddleware {
                    scheme: scheme.to_string(),
                    permanent: opts.permanent,
                },
            ))
        }

        MiddlewareType::BasicAuth => {
            if opts.users.is_empty() {
                bail!("--user is required for basic-auth middleware (at least one)");
            }
            Ok(MiddlewareDefinition::basic_auth(BasicAuthMiddleware {
                users: opts.users.to_vec(),
                realm: opts.realm.map(|s| s.to_string()),
            }))
        }

        MiddlewareType::StripPrefix => {
            if opts.prefixes.is_empty() {
                bail!("--prefix is required for strip-prefix middleware (at least one)");
            }
            Ok(MiddlewareDefinition::strip_prefix(StripPrefixMiddleware {
                prefixes: opts.prefixes.to_vec(),
            }))
        }

        MiddlewareType::Compress => Ok(MiddlewareDefinition::compress(CompressMiddleware {
            excluded_content_types: None,
        })),
    }
}

pub fn execute(dir: &Path, opts: AddMiddlewareOptions) -> Result<()> {
    validate_name(opts.name)?;

    let file_path = dir.join(format!("mw-{}.yml", opts.name));

    if file_path.exists() && !opts.force {
        bail!(
            "middleware {:?} already exists at {}. Use --force to overwrite.",
            opts.name,
            file_path.display()
        );
    }

    let definition = build_definition(&opts)?;
    let config = MiddlewareDynamicConfig::new(opts.name, definition);
    let yaml = config
        .to_yaml()
        .context("failed to serialize middleware to YAML")?;

    if opts.dry_run {
        println!("{}", "--- dry-run: would write ---".yellow().bold());
        println!("{}: {}", "file".bold(), file_path.display());
        println!("{yaml}");
        return Ok(());
    }

    if !dir.exists() {
        fs::create_dir_all(dir)
            .with_context(|| format!("failed to create directory {}", dir.display()))?;
    }

    fs::write(&file_path, &yaml)
        .with_context(|| format!("failed to write {}", file_path.display()))?;

    let mw_type_label = config.middleware_type().unwrap_or("unknown");
    println!(
        "{} middleware {} ({}) → {}",
        "✓".green().bold(),
        opts.name.cyan(),
        mw_type_label.yellow(),
        file_path.display()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::MiddlewareDynamicConfig;
    use std::fs;

    fn default_header_opts<'a>(
        name: &'a str,
        mw_type: &'a MiddlewareType,
    ) -> AddMiddlewareOptions<'a> {
        AddMiddlewareOptions {
            name,
            mw_type,
            security_preset: false,
            sts_seconds: None,
            frame_deny: None,
            referrer_policy: None,
            response_headers: &[],
            request_headers: &[],
            average: None,
            burst: None,
            period: None,
            scheme: None,
            permanent: None,
            users: &[],
            realm: None,
            prefixes: &[],
            force: false,
            dry_run: false,
        }
    }

    #[test]
    fn add_headers_security_preset() {
        let dir = tempfile::tempdir().unwrap();
        let mw_type = MiddlewareType::Headers;
        let mut opts = default_header_opts("sec-hdr", &mw_type);
        opts.security_preset = true;
        execute(dir.path(), opts).unwrap();

        let content = fs::read_to_string(dir.path().join("mw-sec-hdr.yml")).unwrap();
        let parsed: MiddlewareDynamicConfig = serde_yaml::from_str(&content).unwrap();
        assert_eq!(parsed.middleware_type(), Some("headers"));
        let hdr = parsed.http.middlewares.get("sec-hdr").unwrap().headers.as_ref().unwrap();
        assert_eq!(hdr.sts_seconds, Some(63_072_000));
        assert_eq!(hdr.frame_deny, Some(true));
    }

    #[test]
    fn add_rate_limit() {
        let dir = tempfile::tempdir().unwrap();
        let mw_type = MiddlewareType::RateLimit;
        let mut opts = default_header_opts("rl", &mw_type);
        opts.average = Some(100);
        opts.burst = Some(200);
        let period = "1m".to_string();
        opts.period = Some(&period);
        execute(dir.path(), opts).unwrap();

        let content = fs::read_to_string(dir.path().join("mw-rl.yml")).unwrap();
        assert!(content.contains("rateLimit:"));
        assert!(content.contains("average: 100"));
    }

    #[test]
    fn add_rate_limit_requires_average() {
        let dir = tempfile::tempdir().unwrap();
        let mw_type = MiddlewareType::RateLimit;
        let opts = default_header_opts("rl-fail", &mw_type);
        let err = execute(dir.path(), opts).unwrap_err();
        assert!(err.to_string().contains("--average"));
    }

    #[test]
    fn add_redirect_scheme() {
        let dir = tempfile::tempdir().unwrap();
        let mw_type = MiddlewareType::RedirectScheme;
        let mut opts = default_header_opts("redir", &mw_type);
        let scheme = "https".to_string();
        opts.scheme = Some(&scheme);
        opts.permanent = Some(true);
        execute(dir.path(), opts).unwrap();

        let content = fs::read_to_string(dir.path().join("mw-redir.yml")).unwrap();
        assert!(content.contains("redirectScheme:"));
        assert!(content.contains("scheme: https"));
    }

    #[test]
    fn add_redirect_scheme_requires_scheme() {
        let dir = tempfile::tempdir().unwrap();
        let mw_type = MiddlewareType::RedirectScheme;
        let opts = default_header_opts("redir-fail", &mw_type);
        let err = execute(dir.path(), opts).unwrap_err();
        assert!(err.to_string().contains("--scheme"));
    }

    #[test]
    fn add_basic_auth() {
        let dir = tempfile::tempdir().unwrap();
        let mw_type = MiddlewareType::BasicAuth;
        let users = vec!["admin:$apr1$xyz".to_string()];
        let mut opts = default_header_opts("ba", &mw_type);
        opts.users = &users;
        let realm = "restricted".to_string();
        opts.realm = Some(&realm);
        execute(dir.path(), opts).unwrap();

        let content = fs::read_to_string(dir.path().join("mw-ba.yml")).unwrap();
        assert!(content.contains("basicAuth:"));
    }

    #[test]
    fn add_basic_auth_requires_users() {
        let dir = tempfile::tempdir().unwrap();
        let mw_type = MiddlewareType::BasicAuth;
        let opts = default_header_opts("ba-fail", &mw_type);
        let err = execute(dir.path(), opts).unwrap_err();
        assert!(err.to_string().contains("--user"));
    }

    #[test]
    fn add_strip_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let mw_type = MiddlewareType::StripPrefix;
        let prefixes = vec!["/api".to_string(), "/v1".to_string()];
        let mut opts = default_header_opts("sp", &mw_type);
        opts.prefixes = &prefixes;
        execute(dir.path(), opts).unwrap();

        let content = fs::read_to_string(dir.path().join("mw-sp.yml")).unwrap();
        assert!(content.contains("stripPrefix:"));
        assert!(content.contains("/api"));
    }

    #[test]
    fn add_strip_prefix_requires_prefixes() {
        let dir = tempfile::tempdir().unwrap();
        let mw_type = MiddlewareType::StripPrefix;
        let opts = default_header_opts("sp-fail", &mw_type);
        let err = execute(dir.path(), opts).unwrap_err();
        assert!(err.to_string().contains("--prefix"));
    }

    #[test]
    fn add_compress() {
        let dir = tempfile::tempdir().unwrap();
        let mw_type = MiddlewareType::Compress;
        let opts = default_header_opts("cmp", &mw_type);
        execute(dir.path(), opts).unwrap();

        let content = fs::read_to_string(dir.path().join("mw-cmp.yml")).unwrap();
        assert!(content.contains("compress:"));
    }

    #[test]
    fn add_middleware_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        let mw_type = MiddlewareType::Compress;
        let mut opts = default_header_opts("dry-mw", &mw_type);
        opts.dry_run = true;
        execute(dir.path(), opts).unwrap();
        assert!(!dir.path().join("mw-dry-mw.yml").exists());
    }

    #[test]
    fn add_middleware_duplicate_without_force() {
        let dir = tempfile::tempdir().unwrap();
        let mw_type = MiddlewareType::Compress;
        execute(dir.path(), default_header_opts("dup-mw", &mw_type)).unwrap();
        let err = execute(dir.path(), default_header_opts("dup-mw", &mw_type)).unwrap_err();
        assert!(err.to_string().contains("already exists"));
    }

    #[test]
    fn add_middleware_force_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let mw_type = MiddlewareType::Compress;
        execute(dir.path(), default_header_opts("force-mw", &mw_type)).unwrap();
        let mut opts = default_header_opts("force-mw", &mw_type);
        opts.force = true;
        execute(dir.path(), opts).unwrap();
    }

    #[test]
    fn parse_kv_pairs_valid() {
        let pairs = vec!["X-Custom=value".to_string(), "Server=".to_string()];
        let map = parse_kv_pairs(&pairs).unwrap();
        assert_eq!(map.get("X-Custom"), Some(&"value".to_string()));
        assert_eq!(map.get("Server"), Some(&String::new()));
    }

    #[test]
    fn parse_kv_pairs_invalid() {
        let pairs = vec!["no-equals-sign".to_string()];
        let err = parse_kv_pairs(&pairs).unwrap_err();
        assert!(err.to_string().contains("KEY=VALUE"));
    }

    #[test]
    fn add_headers_with_custom_headers() {
        let dir = tempfile::tempdir().unwrap();
        let mw_type = MiddlewareType::Headers;
        let resp_headers = vec!["X-Frame-Options=DENY".to_string()];
        let req_headers = vec!["X-Request-Id=auto".to_string()];
        let mut opts = default_header_opts("custom-hdr", &mw_type);
        opts.response_headers = &resp_headers;
        opts.request_headers = &req_headers;
        execute(dir.path(), opts).unwrap();

        let content = fs::read_to_string(dir.path().join("mw-custom-hdr.yml")).unwrap();
        assert!(content.contains("X-Frame-Options"));
        assert!(content.contains("X-Request-Id"));
    }
}
