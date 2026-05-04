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
