use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, ValueEnum)]
pub enum MiddlewareType {
    Headers,
    RateLimit,
    RedirectScheme,
    BasicAuth,
    StripPrefix,
    Compress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum Protocol {
    Http,
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ServicePreset {
    Postgres,
    Mysql,
    Mariadb,
    Redis,
    Mongodb,
    Dns,
    Mqtt,
    Nats,
    Syslog,
}

pub struct PresetDefaults {
    pub protocol: Protocol,
    pub port: u16,
    pub entrypoint: &'static str,
}

impl ServicePreset {
    pub fn defaults(&self) -> PresetDefaults {
        match self {
            Self::Postgres => PresetDefaults { protocol: Protocol::Tcp, port: 5432, entrypoint: "postgres" },
            Self::Mysql | Self::Mariadb => PresetDefaults { protocol: Protocol::Tcp, port: 3306, entrypoint: "mysql" },
            Self::Redis => PresetDefaults { protocol: Protocol::Tcp, port: 6379, entrypoint: "redis" },
            Self::Mongodb => PresetDefaults { protocol: Protocol::Tcp, port: 27017, entrypoint: "mongodb" },
            Self::Dns => PresetDefaults { protocol: Protocol::Udp, port: 53, entrypoint: "dns" },
            Self::Mqtt => PresetDefaults { protocol: Protocol::Tcp, port: 1883, entrypoint: "mqtt" },
            Self::Nats => PresetDefaults { protocol: Protocol::Tcp, port: 4222, entrypoint: "nats" },
            Self::Syslog => PresetDefaults { protocol: Protocol::Udp, port: 514, entrypoint: "syslog" },
        }
    }
}

/// traefikctl — Manage Traefik dynamic configuration via the file provider
#[derive(Parser, Debug)]
#[command(name = "traefikctl", version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Directory for Traefik dynamic config files
    #[arg(long, global = true, default_value = "/etc/traefik/conf.d")]
    pub dir: PathBuf,

    /// Reload Traefik after changes (via systemctl)
    #[arg(long, global = true, default_value_t = false)]
    pub reload: bool,

    /// Skip confirmation prompts
    #[arg(long, global = true, default_value_t = false)]
    pub force: bool,

    /// Dry-run mode — print what would be done without writing
    #[arg(long, global = true, default_value_t = false)]
    pub dry_run: bool,

    /// Path to Traefik static config file (auto-detected if omitted)
    #[arg(long, global = true)]
    pub traefik_config: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add a new route (HTTP, TCP, or UDP)
    Add {
        /// Service name (used as filename and identifier)
        #[arg(short, long)]
        name: String,

        /// Protocol type (default: http)
        #[arg(short, long, value_enum, default_value = "http")]
        protocol: Protocol,

        /// Use a service preset (sets protocol, port, and entrypoint automatically)
        #[arg(long, value_enum)]
        preset: Option<ServicePreset>,

        /// Domain to route — required for HTTP, optional for TCP (HostSNI)
        #[arg(short = 'H', long)]
        host: Option<String>,

        /// Backend URL — required for HTTP (e.g. http://127.0.0.1:3000)
        #[arg(short, long)]
        url: Option<String>,

        /// Backend address — required for TCP/UDP (e.g. 10.0.0.1:5432)
        #[arg(short, long)]
        address: Option<String>,

        /// Traefik entrypoint (default: web for HTTP, preset-specific for presets)
        #[arg(short, long)]
        entrypoint: Option<String>,

        /// Enable TLS (HTTP: adds websecure entrypoint; TCP: adds tls section)
        #[arg(long, default_value_t = false)]
        tls: bool,

        /// TLS passthrough — TCP only (forward encrypted traffic without termination)
        #[arg(long, default_value_t = false)]
        tls_passthrough: bool,

        /// TLS certificate resolver name (omit to use default TLS store)
        #[arg(long)]
        cert_resolver: Option<String>,

        /// Middleware names to attach (comma-separated)
        #[arg(long)]
        middlewares: Option<String>,
    },

    /// Remove an existing route
    Remove {
        /// Service name to remove
        #[arg(short, long)]
        name: String,
    },

    /// List all configured routes
    List,

    /// Check and fix Traefik static config (file provider setup)
    Doctor,

    /// Add a middleware definition
    AddMiddleware {
        /// Middleware name (used as filename and identifier)
        #[arg(short, long)]
        name: String,

        /// Middleware type
        #[arg(short = 't', long = "type", value_enum)]
        mw_type: MiddlewareType,

        /// Use security-headers preset (headers type only)
        #[arg(long, default_value_t = false)]
        security_preset: bool,

        /// STS max-age in seconds (headers type only)
        #[arg(long)]
        sts_seconds: Option<u64>,

        /// Deny framing (headers type only)
        #[arg(long)]
        frame_deny: Option<bool>,

        /// Referrer policy (headers type only)
        #[arg(long)]
        referrer_policy: Option<String>,

        /// Custom response headers (key=value, repeatable)
        #[arg(long = "response-header", value_name = "KEY=VALUE")]
        response_headers: Vec<String>,

        /// Custom request headers (key=value, repeatable)
        #[arg(long = "request-header", value_name = "KEY=VALUE")]
        request_headers: Vec<String>,

        /// Rate limit average requests per period (rate-limit type)
        #[arg(long)]
        average: Option<u64>,

        /// Rate limit burst (rate-limit type)
        #[arg(long)]
        burst: Option<u64>,

        /// Rate limit period (rate-limit type, e.g. "1s", "1m")
        #[arg(long)]
        period: Option<String>,

        /// Redirect target scheme (redirect-scheme type, e.g. "https")
        #[arg(long)]
        scheme: Option<String>,

        /// Permanent redirect (redirect-scheme type)
        #[arg(long)]
        permanent: Option<bool>,

        /// Basic auth users (user:password htpasswd format, repeatable)
        #[arg(long = "user")]
        users: Vec<String>,

        /// Basic auth realm (basic-auth type)
        #[arg(long)]
        realm: Option<String>,

        /// Prefixes to strip (strip-prefix type, repeatable)
        #[arg(long = "prefix")]
        prefixes: Vec<String>,
    },

    /// Remove a middleware definition
    RemoveMiddleware {
        /// Middleware name to remove
        #[arg(short, long)]
        name: String,
    },

    /// Set up an ACME certificate resolver with DNS-01 challenge
    InitAcme {
        /// Resolver name (default: letsencrypt)
        #[arg(long, default_value = "letsencrypt")]
        resolver_name: String,

        /// ACME account email
        #[arg(short, long)]
        email: String,

        /// DNS challenge provider (lego provider code, e.g. cloudflare, route53, digitalocean)
        #[arg(short, long)]
        provider: String,

        /// Use Let's Encrypt staging CA
        #[arg(long, default_value_t = false)]
        staging: bool,

        /// ACME certificate storage path
        #[arg(long, default_value = "/etc/traefik/acme.json")]
        storage: String,

        /// Custom DNS resolvers (e.g. 1.1.1.1:53, repeatable)
        #[arg(long = "dns-resolver")]
        dns_resolvers: Vec<String>,

        /// Key type (RSA2048, RSA4096, EC256, EC384)
        #[arg(long)]
        key_type: Option<String>,

        /// Seconds to wait before propagation check
        #[arg(long)]
        propagation_delay: Option<u64>,

        /// Disable DNS propagation checks
        #[arg(long, default_value_t = false)]
        disable_propagation_check: bool,
    },

    /// Set up self-signed CA and default TLS certificate
    InitCa {
        /// Path to Root CA certificate file (.crt/.pem)
        #[arg(long)]
        ca_cert: String,

        /// Path to Intermediate CA certificate file (optional)
        #[arg(long)]
        intermediate_cert: Option<String>,

        /// Path to default server certificate file
        #[arg(long)]
        cert: String,

        /// Path to default server private key file
        #[arg(long)]
        key: String,

        /// Certificate storage directory
        #[arg(long, default_value = "/etc/traefik/certs")]
        certs_dir: String,

        /// TLS options name for mTLS client verification
        #[arg(long)]
        mtls: bool,

        /// Minimum TLS version (VersionTLS12, VersionTLS13)
        #[arg(long)]
        min_version: Option<String>,
    },

    /// Import a TLS certificate for a specific route
    AddCert {
        /// Route name to attach the certificate to
        #[arg(short, long)]
        name: String,

        /// Path to server certificate file (.crt/.pem)
        #[arg(long)]
        cert: String,

        /// Path to server private key file
        #[arg(long)]
        key: String,

        /// Certificate storage directory
        #[arg(long, default_value = "/etc/traefik/certs")]
        certs_dir: String,
    },

    /// Update an existing route
    Update {
        /// Service name to update
        #[arg(short, long)]
        name: String,

        /// New domain (optional)
        #[arg(short = 'H', long)]
        host: Option<String>,

        /// New backend URL (optional)
        #[arg(short, long)]
        url: Option<String>,

        /// New entrypoint (optional)
        #[arg(short, long)]
        entrypoint: Option<String>,

        /// Enable TLS
        #[arg(long)]
        tls: Option<bool>,

        /// Middleware names to attach (comma-separated, replaces existing)
        #[arg(long)]
        middlewares: Option<String>,
    },
}
