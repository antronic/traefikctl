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
    /// Add a new route
    Add {
        /// Service name (used as filename and identifier)
        #[arg(short, long)]
        name: String,

        /// Domain to route (e.g. app.example.com)
        #[arg(short = 'H', long)]
        host: String,

        /// Backend URL (e.g. http://127.0.0.1:3000)
        #[arg(short, long)]
        url: String,

        /// Traefik entrypoint (default: web)
        #[arg(short, long, default_value = "web")]
        entrypoint: String,

        /// Enable TLS (adds websecure entrypoint + certresolver)
        #[arg(long, default_value_t = false)]
        tls: bool,

        /// TLS certificate resolver name
        #[arg(long, default_value = "letsencrypt")]
        cert_resolver: String,

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
