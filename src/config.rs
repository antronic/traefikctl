use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct TraefikStaticConfig {
    #[serde(default)]
    pub providers: Option<Providers>,
    #[serde(flatten)]
    pub rest: BTreeMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Providers {
    #[serde(default)]
    pub file: Option<FileProvider>,
    #[serde(flatten)]
    pub rest: BTreeMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct FileProvider {
    #[serde(default)]
    pub directory: Option<String>,
    #[serde(default)]
    pub watch: Option<bool>,
    #[serde(flatten)]
    pub rest: BTreeMap<String, Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TraefikDynamicConfig {
    pub http: HttpConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpConfig {
    pub routers: BTreeMap<String, Router>,
    pub services: BTreeMap<String, Service>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Router {
    pub rule: String,
    pub entry_points: Vec<String>,
    pub service: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<RouterTls>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middlewares: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RouterTls {
    pub cert_resolver: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Service {
    pub load_balancer: LoadBalancer,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LoadBalancer {
    pub servers: Vec<Server>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Server {
    pub url: String,
}

// ---------------------------------------------------------------------------
// Middleware dynamic config (one file per middleware)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MiddlewareDynamicConfig {
    pub http: MiddlewareHttpConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MiddlewareHttpConfig {
    pub middlewares: BTreeMap<String, MiddlewareDefinition>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MiddlewareDefinition {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HeadersMiddleware>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit: Option<RateLimitMiddleware>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub redirect_scheme: Option<RedirectSchemeMiddleware>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub basic_auth: Option<BasicAuthMiddleware>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strip_prefix: Option<StripPrefixMiddleware>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compress: Option<CompressMiddleware>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct HeadersMiddleware {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sts_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sts_include_subdomains: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sts_preload: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_deny: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type_nosniff: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_xss_filter: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referrer_policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_response_headers: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_request_headers: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_control_allow_methods: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_control_allow_headers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_control_allow_origin_list: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub access_control_max_age: Option<u64>,
}

impl HeadersMiddleware {
    /// Security-hardened preset (HSTS, frame deny, nosniff, XSS filter).
    pub fn security_preset() -> Self {
        let mut strip_headers = BTreeMap::new();
        strip_headers.insert("X-Powered-By".to_string(), String::new());
        strip_headers.insert("Server".to_string(), String::new());

        Self {
            sts_seconds: Some(63_072_000),
            sts_include_subdomains: Some(true),
            sts_preload: Some(true),
            frame_deny: Some(true),
            content_type_nosniff: Some(true),
            browser_xss_filter: Some(true),
            referrer_policy: Some("strict-origin-when-cross-origin".to_string()),
            custom_response_headers: Some(strip_headers),
            custom_request_headers: None,
            access_control_allow_methods: None,
            access_control_allow_headers: None,
            access_control_allow_origin_list: None,
            access_control_max_age: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitMiddleware {
    pub average: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub burst: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub period: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RedirectSchemeMiddleware {
    pub scheme: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permanent: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BasicAuthMiddleware {
    pub users: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub realm: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct StripPrefixMiddleware {
    pub prefixes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompressMiddleware {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub excluded_content_types: Option<Vec<String>>,
}

impl MiddlewareDynamicConfig {
    pub fn new(name: &str, definition: MiddlewareDefinition) -> Self {
        let mut middlewares = BTreeMap::new();
        middlewares.insert(name.to_string(), definition);
        Self {
            http: MiddlewareHttpConfig { middlewares },
        }
    }

    pub fn to_yaml(&self) -> anyhow::Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }

    pub fn middleware_name(&self) -> Option<&str> {
        self.http.middlewares.keys().next().map(|s| s.as_str())
    }

    pub fn middleware_type(&self) -> Option<&'static str> {
        self.http.middlewares.values().next().and_then(|d| {
            if d.headers.is_some() {
                return Some("headers");
            }
            if d.rate_limit.is_some() {
                return Some("rate-limit");
            }
            if d.redirect_scheme.is_some() {
                return Some("redirect-scheme");
            }
            if d.basic_auth.is_some() {
                return Some("basic-auth");
            }
            if d.strip_prefix.is_some() {
                return Some("strip-prefix");
            }
            if d.compress.is_some() {
                return Some("compress");
            }
            None
        })
    }
}

impl MiddlewareDefinition {
    pub fn headers(h: HeadersMiddleware) -> Self {
        Self {
            headers: Some(h),
            rate_limit: None,
            redirect_scheme: None,
            basic_auth: None,
            strip_prefix: None,
            compress: None,
        }
    }
    pub fn rate_limit(r: RateLimitMiddleware) -> Self {
        Self {
            headers: None,
            rate_limit: Some(r),
            redirect_scheme: None,
            basic_auth: None,
            strip_prefix: None,
            compress: None,
        }
    }
    pub fn redirect_scheme(r: RedirectSchemeMiddleware) -> Self {
        Self {
            headers: None,
            rate_limit: None,
            redirect_scheme: Some(r),
            basic_auth: None,
            strip_prefix: None,
            compress: None,
        }
    }
    pub fn basic_auth(b: BasicAuthMiddleware) -> Self {
        Self {
            headers: None,
            rate_limit: None,
            redirect_scheme: None,
            basic_auth: Some(b),
            strip_prefix: None,
            compress: None,
        }
    }
    pub fn strip_prefix(s: StripPrefixMiddleware) -> Self {
        Self {
            headers: None,
            rate_limit: None,
            redirect_scheme: None,
            basic_auth: None,
            strip_prefix: Some(s),
            compress: None,
        }
    }
    pub fn compress(c: CompressMiddleware) -> Self {
        Self {
            headers: None,
            rate_limit: None,
            redirect_scheme: None,
            basic_auth: None,
            strip_prefix: None,
            compress: Some(c),
        }
    }
}

// ---------------------------------------------------------------------------
// Route dynamic config
// ---------------------------------------------------------------------------

impl TraefikDynamicConfig {
    pub fn new(
        name: &str,
        host: &str,
        backend_url: &str,
        entrypoints: Vec<String>,
        tls: Option<RouterTls>,
        middlewares: Option<Vec<String>>,
    ) -> Self {
        let mut routers = BTreeMap::new();
        routers.insert(
            name.to_string(),
            Router {
                rule: format!("Host(`{host}`)"),
                entry_points: entrypoints,
                service: name.to_string(),
                tls,
                middlewares,
            },
        );

        let mut services = BTreeMap::new();
        services.insert(
            name.to_string(),
            Service {
                load_balancer: LoadBalancer {
                    servers: vec![Server {
                        url: backend_url.to_string(),
                    }],
                },
            },
        );

        Self {
            http: HttpConfig { routers, services },
        }
    }

    /// Returns the first router name (the service/route name).
    pub fn route_name(&self) -> Option<&str> {
        self.http.routers.keys().next().map(|s| s.as_str())
    }

    /// Returns the host from the first router rule.
    pub fn host(&self) -> Option<String> {
        self.http.routers.values().next().and_then(|r| {
            // Parse Host(`example.com`) → example.com
            r.rule
                .strip_prefix("Host(`")
                .and_then(|s| s.strip_suffix("`)"))
                .map(|s| s.to_string())
        })
    }

    /// Returns the backend URL from the first service.
    pub fn backend_url(&self) -> Option<&str> {
        self.http
            .services
            .values()
            .next()
            .and_then(|s| s.load_balancer.servers.first())
            .map(|s| s.url.as_str())
    }

    /// Serialize to YAML string.
    pub fn to_yaml(&self) -> anyhow::Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }
}
