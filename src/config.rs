use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::BTreeMap;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TraefikStaticConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub providers: Option<Providers>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificates_resolvers: Option<BTreeMap<String, CertificateResolver>>,
    #[serde(flatten)]
    pub rest: BTreeMap<String, Value>,
}

// ---------------------------------------------------------------------------
// ACME / Certificate Resolver config (static config)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CertificateResolver {
    pub acme: AcmeConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AcmeConfig {
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ca_server: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_challenge: Option<DnsChallenge>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DnsChallenge {
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolvers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub propagation: Option<DnsPropagation>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DnsPropagation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delay_before_checks: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_checks: Option<bool>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http: Option<HttpConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tcp: Option<TcpConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub udp: Option<UdpConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls: Option<TlsConfig>,
}

// ---------------------------------------------------------------------------
// TLS dynamic config (certificates, stores, options)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TlsConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub certificates: Option<Vec<TlsCertificate>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stores: Option<BTreeMap<String, TlsStore>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<BTreeMap<String, TlsOptions>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TlsCertificate {
    pub cert_file: String,
    pub key_file: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TlsStore {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_certificate: Option<TlsCertificate>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TlsOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_auth: Option<ClientAuth>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ClientAuth {
    pub ca_files: Vec<String>,
    pub client_auth_type: String,
}

// ---------------------------------------------------------------------------
// TCP dynamic config
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TcpConfig {
    pub routers: BTreeMap<String, TcpRouter>,
    pub services: BTreeMap<String, TcpService>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TcpRouter {
    pub rule: String,
    pub entry_points: Vec<String>,
    pub service: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<TcpRouterTls>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub middlewares: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct TcpRouterTls {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passthrough: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cert_resolver: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TcpService {
    pub load_balancer: TcpLoadBalancer,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TcpLoadBalancer {
    pub servers: Vec<TcpServer>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TcpServer {
    pub address: String,
}

// ---------------------------------------------------------------------------
// UDP dynamic config
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UdpConfig {
    pub routers: BTreeMap<String, UdpRouter>,
    pub services: BTreeMap<String, UdpService>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UdpRouter {
    pub entry_points: Vec<String>,
    pub service: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UdpService {
    pub load_balancer: UdpLoadBalancer,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UdpLoadBalancer {
    pub servers: Vec<UdpServer>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UdpServer {
    pub address: String,
}

// ---------------------------------------------------------------------------
// HTTP dynamic config
// ---------------------------------------------------------------------------

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

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct RouterTls {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cert_resolver: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub options: Option<String>,
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
    pub fn new_http(
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
            http: Some(HttpConfig { routers, services }),
            tcp: None,
            udp: None,
            tls: None,
        }
    }

    pub fn new_tcp(
        name: &str,
        rule: &str,
        address: &str,
        entrypoints: Vec<String>,
        tls: Option<TcpRouterTls>,
    ) -> Self {
        let mut routers = BTreeMap::new();
        routers.insert(
            name.to_string(),
            TcpRouter {
                rule: rule.to_string(),
                entry_points: entrypoints,
                service: name.to_string(),
                tls,
                middlewares: None,
                priority: None,
            },
        );

        let mut services = BTreeMap::new();
        services.insert(
            name.to_string(),
            TcpService {
                load_balancer: TcpLoadBalancer {
                    servers: vec![TcpServer {
                        address: address.to_string(),
                    }],
                },
            },
        );

        Self {
            http: None,
            tcp: Some(TcpConfig { routers, services }),
            udp: None,
            tls: None,
        }
    }

    pub fn new_udp(name: &str, address: &str, entrypoints: Vec<String>) -> Self {
        let mut routers = BTreeMap::new();
        routers.insert(
            name.to_string(),
            UdpRouter {
                entry_points: entrypoints,
                service: name.to_string(),
            },
        );

        let mut services = BTreeMap::new();
        services.insert(
            name.to_string(),
            UdpService {
                load_balancer: UdpLoadBalancer {
                    servers: vec![UdpServer {
                        address: address.to_string(),
                    }],
                },
            },
        );

        Self {
            http: None,
            tcp: None,
            udp: Some(UdpConfig { routers, services }),
            tls: None,
        }
    }

    pub fn route_name(&self) -> Option<&str> {
        if let Some(http) = &self.http {
            return http.routers.keys().next().map(|s| s.as_str());
        }
        if let Some(tcp) = &self.tcp {
            return tcp.routers.keys().next().map(|s| s.as_str());
        }
        if let Some(udp) = &self.udp {
            return udp.routers.keys().next().map(|s| s.as_str());
        }
        None
    }

    pub fn protocol(&self) -> &'static str {
        if self.http.is_some() {
            "http"
        } else if self.tcp.is_some() {
            "tcp"
        } else if self.udp.is_some() {
            "udp"
        } else {
            "unknown"
        }
    }

    pub fn host(&self) -> Option<String> {
        self.http.as_ref()?.routers.values().next().and_then(|r| {
            r.rule
                .strip_prefix("Host(`")
                .and_then(|s| s.strip_suffix("`)"))
                .map(|s| s.to_string())
        })
    }

    pub fn tcp_rule(&self) -> Option<&str> {
        self.tcp
            .as_ref()?
            .routers
            .values()
            .next()
            .map(|r| r.rule.as_str())
    }

    pub fn backend_url(&self) -> Option<&str> {
        self.http
            .as_ref()?
            .services
            .values()
            .next()
            .and_then(|s| s.load_balancer.servers.first())
            .map(|s| s.url.as_str())
    }

    pub fn backend_address(&self) -> Option<&str> {
        if let Some(tcp) = &self.tcp {
            return tcp
                .services
                .values()
                .next()
                .and_then(|s| s.load_balancer.servers.first())
                .map(|s| s.address.as_str());
        }
        if let Some(udp) = &self.udp {
            return udp
                .services
                .values()
                .next()
                .and_then(|s| s.load_balancer.servers.first())
                .map(|s| s.address.as_str());
        }
        None
    }

    pub fn to_yaml(&self) -> anyhow::Result<String> {
        Ok(serde_yaml::to_string(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dynamic_config_new_basic() {
        let cfg = TraefikDynamicConfig::new_http(
            "myapp",
            "app.example.com",
            "http://127.0.0.1:3000",
            vec!["web".into()],
            None,
            None,
        );
        assert_eq!(cfg.route_name(), Some("myapp"));
        assert_eq!(cfg.protocol(), "http");
        assert_eq!(cfg.host(), Some("app.example.com".to_string()));
        assert_eq!(cfg.backend_url(), Some("http://127.0.0.1:3000"));
        let http = cfg.http.as_ref().unwrap();
        let router = http.routers.get("myapp").unwrap();
        assert_eq!(router.rule, "Host(`app.example.com`)");
        assert_eq!(router.entry_points, vec!["web"]);
        assert_eq!(router.service, "myapp");
        assert!(router.tls.is_none());
        assert!(router.middlewares.is_none());
    }

    #[test]
    fn dynamic_config_new_with_tls() {
        let cfg = TraefikDynamicConfig::new_http(
            "secure",
            "sec.example.com",
            "https://backend:8443",
            vec!["web".into(), "websecure".into()],
            Some(RouterTls {
                cert_resolver: Some("letsencrypt".to_string()),
                ..Default::default()
            }),
            None,
        );
        let http = cfg.http.as_ref().unwrap();
        let router = http.routers.get("secure").unwrap();
        assert!(router.tls.is_some());
        assert_eq!(
            router.tls.as_ref().unwrap().cert_resolver,
            Some("letsencrypt".to_string())
        );
        assert_eq!(router.entry_points, vec!["web", "websecure"]);
    }

    #[test]
    fn dynamic_config_new_with_middlewares() {
        let cfg = TraefikDynamicConfig::new_http(
            "mw-test",
            "mw.example.com",
            "http://localhost:4000",
            vec!["web".into()],
            None,
            Some(vec!["headers".into(), "rate-limit".into()]),
        );
        let http = cfg.http.as_ref().unwrap();
        let router = http.routers.get("mw-test").unwrap();
        assert_eq!(
            router.middlewares.as_ref().unwrap(),
            &vec!["headers".to_string(), "rate-limit".to_string()]
        );
    }

    #[test]
    fn dynamic_config_to_yaml_camelcase() {
        let cfg = TraefikDynamicConfig::new_http(
            "demo",
            "demo.test.io",
            "http://127.0.0.1:5000",
            vec!["web".into(), "websecure".into()],
            Some(RouterTls {
                cert_resolver: Some("letsencrypt".to_string()),
                ..Default::default()
            }),
            Some(vec!["headers".into()]),
        );
        let yaml = cfg.to_yaml().unwrap();
        assert!(yaml.contains("entryPoints:"));
        assert!(yaml.contains("loadBalancer:"));
        assert!(yaml.contains("certResolver:"));
        assert!(yaml.contains("http:"));
        assert!(yaml.contains("routers:"));
        assert!(yaml.contains("services:"));
        assert!(yaml.contains("Host(`demo.test.io`)"));
    }

    #[test]
    fn dynamic_config_yaml_roundtrip() {
        let cfg = TraefikDynamicConfig::new_http(
            "roundtrip",
            "rt.example.com",
            "http://10.0.0.1:9090",
            vec!["web".into()],
            None,
            None,
        );
        let yaml = cfg.to_yaml().unwrap();
        let parsed: TraefikDynamicConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.route_name(), Some("roundtrip"));
        assert_eq!(parsed.host(), Some("rt.example.com".to_string()));
        assert_eq!(parsed.backend_url(), Some("http://10.0.0.1:9090"));
    }

    #[test]
    fn dynamic_config_tls_yaml_roundtrip() {
        let cfg = TraefikDynamicConfig::new_http(
            "tls-rt",
            "tls.example.com",
            "https://back:443",
            vec!["websecure".into()],
            Some(RouterTls {
                cert_resolver: Some("myresolver".to_string()),
                ..Default::default()
            }),
            Some(vec!["auth".into()]),
        );
        let yaml = cfg.to_yaml().unwrap();
        let parsed: TraefikDynamicConfig = serde_yaml::from_str(&yaml).unwrap();
        let http = parsed.http.as_ref().unwrap();
        let router = http.routers.get("tls-rt").unwrap();
        assert_eq!(
            router.tls.as_ref().unwrap().cert_resolver,
            Some("myresolver".to_string())
        );
        assert_eq!(
            router.middlewares.as_ref().unwrap(),
            &vec!["auth".to_string()]
        );
    }

    #[test]
    fn dynamic_config_btreemap_deterministic_order() {
        let cfg = TraefikDynamicConfig::new_http(
            "zzz",
            "z.example.com",
            "http://z:1",
            vec!["web".into()],
            None,
            None,
        );
        let yaml1 = cfg.to_yaml().unwrap();
        let yaml2 = cfg.to_yaml().unwrap();
        assert_eq!(yaml1, yaml2);
    }

    #[test]
    fn host_parsing_non_standard_rule() {
        let mut cfg = TraefikDynamicConfig::new_http(
            "weird",
            "x.io",
            "http://x:1",
            vec!["web".into()],
            None,
            None,
        );
        cfg.http.as_mut().unwrap().routers.get_mut("weird").unwrap().rule =
            "PathPrefix(`/api`)".to_string();
        assert_eq!(cfg.host(), None);
    }

    #[test]
    fn tcp_config_new_basic() {
        let cfg = TraefikDynamicConfig::new_tcp(
            "postgres",
            "HostSNI(`*`)",
            "10.0.0.1:5432",
            vec!["postgres".into()],
            None,
        );
        assert_eq!(cfg.route_name(), Some("postgres"));
        assert_eq!(cfg.protocol(), "tcp");
        assert_eq!(cfg.tcp_rule(), Some("HostSNI(`*`)"));
        assert_eq!(cfg.backend_address(), Some("10.0.0.1:5432"));
        assert!(cfg.http.is_none());
        assert!(cfg.udp.is_none());
    }

    #[test]
    fn tcp_config_with_tls_passthrough() {
        let cfg = TraefikDynamicConfig::new_tcp(
            "db",
            "HostSNI(`db.example.com`)",
            "10.0.0.1:5432",
            vec!["websecure".into()],
            Some(TcpRouterTls {
                passthrough: Some(true),
                ..Default::default()
            }),
        );
        let yaml = cfg.to_yaml().unwrap();
        assert!(yaml.contains("tcp:"));
        assert!(yaml.contains("HostSNI(`db.example.com`)"));
        assert!(yaml.contains("passthrough: true"));
    }

    #[test]
    fn tcp_config_yaml_roundtrip() {
        let cfg = TraefikDynamicConfig::new_tcp(
            "redis",
            "HostSNI(`*`)",
            "10.0.0.2:6379",
            vec!["redis".into()],
            None,
        );
        let yaml = cfg.to_yaml().unwrap();
        let parsed: TraefikDynamicConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.route_name(), Some("redis"));
        assert_eq!(parsed.protocol(), "tcp");
        assert_eq!(parsed.backend_address(), Some("10.0.0.2:6379"));
    }

    #[test]
    fn udp_config_new_basic() {
        let cfg = TraefikDynamicConfig::new_udp(
            "dns",
            "10.0.0.53:53",
            vec!["dns".into()],
        );
        assert_eq!(cfg.route_name(), Some("dns"));
        assert_eq!(cfg.protocol(), "udp");
        assert_eq!(cfg.backend_address(), Some("10.0.0.53:53"));
        assert!(cfg.http.is_none());
        assert!(cfg.tcp.is_none());
    }

    #[test]
    fn udp_config_yaml_roundtrip() {
        let cfg = TraefikDynamicConfig::new_udp(
            "syslog",
            "10.0.0.10:514",
            vec!["syslog".into()],
        );
        let yaml = cfg.to_yaml().unwrap();
        let parsed: TraefikDynamicConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.route_name(), Some("syslog"));
        assert_eq!(parsed.protocol(), "udp");
        assert_eq!(parsed.backend_address(), Some("10.0.0.10:514"));
    }

    #[test]
    fn udp_config_no_rule_in_yaml() {
        let cfg = TraefikDynamicConfig::new_udp(
            "dns",
            "10.0.0.53:53",
            vec!["dns".into()],
        );
        let yaml = cfg.to_yaml().unwrap();
        assert!(!yaml.contains("rule:"));
        assert!(yaml.contains("udp:"));
        assert!(yaml.contains("entryPoints:"));
    }

    #[test]
    fn middleware_config_new_headers() {
        let def = MiddlewareDefinition::headers(HeadersMiddleware::security_preset());
        let cfg = MiddlewareDynamicConfig::new("sec-headers", def);
        assert_eq!(cfg.middleware_name(), Some("sec-headers"));
        assert_eq!(cfg.middleware_type(), Some("headers"));
    }

    #[test]
    fn middleware_config_new_rate_limit() {
        let def = MiddlewareDefinition::rate_limit(RateLimitMiddleware {
            average: 100,
            burst: Some(200),
            period: Some("1m".to_string()),
        });
        let cfg = MiddlewareDynamicConfig::new("rl", def);
        assert_eq!(cfg.middleware_type(), Some("rate-limit"));
    }

    #[test]
    fn middleware_config_new_redirect_scheme() {
        let def = MiddlewareDefinition::redirect_scheme(RedirectSchemeMiddleware {
            scheme: "https".to_string(),
            permanent: Some(true),
        });
        let cfg = MiddlewareDynamicConfig::new("redir", def);
        assert_eq!(cfg.middleware_type(), Some("redirect-scheme"));
    }

    #[test]
    fn middleware_config_new_basic_auth() {
        let def = MiddlewareDefinition::basic_auth(BasicAuthMiddleware {
            users: vec!["user:pass".to_string()],
            realm: Some("test".to_string()),
        });
        let cfg = MiddlewareDynamicConfig::new("ba", def);
        assert_eq!(cfg.middleware_type(), Some("basic-auth"));
    }

    #[test]
    fn middleware_config_new_strip_prefix() {
        let def = MiddlewareDefinition::strip_prefix(StripPrefixMiddleware {
            prefixes: vec!["/api".to_string()],
        });
        let cfg = MiddlewareDynamicConfig::new("sp", def);
        assert_eq!(cfg.middleware_type(), Some("strip-prefix"));
    }

    #[test]
    fn middleware_config_new_compress() {
        let def = MiddlewareDefinition::compress(CompressMiddleware {
            excluded_content_types: None,
        });
        let cfg = MiddlewareDynamicConfig::new("cmp", def);
        assert_eq!(cfg.middleware_type(), Some("compress"));
    }

    #[test]
    fn middleware_yaml_camelcase() {
        let def = MiddlewareDefinition::rate_limit(RateLimitMiddleware {
            average: 50,
            burst: Some(100),
            period: None,
        });
        let cfg = MiddlewareDynamicConfig::new("rl-test", def);
        let yaml = cfg.to_yaml().unwrap();
        assert!(yaml.contains("rateLimit:"));
        assert!(yaml.contains("average:"));
    }

    #[test]
    fn middleware_yaml_roundtrip() {
        let def = MiddlewareDefinition::headers(HeadersMiddleware::security_preset());
        let cfg = MiddlewareDynamicConfig::new("hdr-rt", def);
        let yaml = cfg.to_yaml().unwrap();
        let parsed: MiddlewareDynamicConfig = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.middleware_name(), Some("hdr-rt"));
        assert_eq!(parsed.middleware_type(), Some("headers"));
        let hdr = parsed
            .http
            .middlewares
            .get("hdr-rt")
            .unwrap()
            .headers
            .as_ref()
            .unwrap();
        assert_eq!(hdr.sts_seconds, Some(63_072_000));
        assert_eq!(hdr.frame_deny, Some(true));
    }

    #[test]
    fn middleware_skip_serializing_none_fields() {
        let def = MiddlewareDefinition::compress(CompressMiddleware {
            excluded_content_types: None,
        });
        let cfg = MiddlewareDynamicConfig::new("cmp-skip", def);
        let yaml = cfg.to_yaml().unwrap();
        assert!(yaml.contains("compress:"));
        assert!(!yaml.contains("headers:"));
        assert!(!yaml.contains("rateLimit:"));
        assert!(!yaml.contains("redirectScheme:"));
        assert!(!yaml.contains("basicAuth:"));
        assert!(!yaml.contains("stripPrefix:"));
    }

    #[test]
    fn security_preset_values() {
        let h = HeadersMiddleware::security_preset();
        assert_eq!(h.sts_seconds, Some(63_072_000));
        assert_eq!(h.sts_include_subdomains, Some(true));
        assert_eq!(h.sts_preload, Some(true));
        assert_eq!(h.frame_deny, Some(true));
        assert_eq!(h.content_type_nosniff, Some(true));
        assert_eq!(h.browser_xss_filter, Some(true));
        assert_eq!(
            h.referrer_policy,
            Some("strict-origin-when-cross-origin".to_string())
        );
        let resp_headers = h.custom_response_headers.as_ref().unwrap();
        assert_eq!(resp_headers.get("Server"), Some(&String::new()));
        assert_eq!(resp_headers.get("X-Powered-By"), Some(&String::new()));
    }

    #[test]
    fn static_config_roundtrip_preserves_unknown_keys() {
        let yaml = r#"
entryPoints:
  web:
    address: ":80"
  websecure:
    address: ":443"
providers:
  file:
    directory: /etc/traefik/conf.d
    watch: true
api:
  dashboard: true
"#;
        let parsed: TraefikStaticConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(parsed.providers.is_some());
        let fp = parsed.providers.as_ref().unwrap().file.as_ref().unwrap();
        assert_eq!(fp.directory, Some("/etc/traefik/conf.d".to_string()));
        assert_eq!(fp.watch, Some(true));
        assert!(parsed.rest.contains_key("entryPoints"));
        assert!(parsed.rest.contains_key("api"));
        let serialized = serde_yaml::to_string(&parsed).unwrap();
        assert!(serialized.contains("entryPoints:"));
        assert!(serialized.contains("dashboard: true"));
    }

    #[test]
    fn static_config_empty() {
        let yaml = "{}";
        let parsed: TraefikStaticConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(parsed.providers.is_none());
        assert!(parsed.rest.is_empty());
    }
}
