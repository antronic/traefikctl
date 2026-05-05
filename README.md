# traefikctl

[![CI](https://github.com/antronic/traefikctl/actions/workflows/ci.yml/badge.svg)](https://github.com/antronic/traefikctl/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/antronic/traefikctl/branch/main/graph/badge.svg)](https://codecov.io/gh/antronic/traefikctl)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

CLI tool to manage [Traefik](https://traefik.io) reverse proxy dynamic configuration via the file provider.

Generates per-service YAML files that Traefik watches and hot-reloads — no Docker labels, no Kubernetes CRDs, just files.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
  - [Routes](#routes)
  - [TCP / UDP Routes](#tcp--udp-routes)
  - [Service Presets](#service-presets)
  - [Middlewares](#middlewares)
  - [ACME / TLS Certificates](#acme--tls-certificates)
  - [Self-Signed CA / mTLS](#self-signed-ca--mtls)
  - [Doctor](#doctor)
  - [Global Flags](#global-flags)
- [Example Output](#example-output)
- [Traefik Setup](#traefik-setup)
- [Build from Source](#build-from-source)
- [Roadmap](#roadmap)
- [License](#license)

## Features

- **One file per route** — Each service gets its own `<name>.yml` in the Traefik watched directory
- **One file per middleware** — Middleware definitions stored as `mw-<name>.yml` alongside routes
- **HTTP, TCP & UDP** — Full protocol support for HTTP reverse proxy, TCP passthrough, and UDP routing
- **Service presets** — One-flag setup for common services (postgres, redis, mysql, mongodb, dns, mqtt, nats, syslog)
- **TLS support** — Automatic `websecure` entrypoint + cert resolver configuration (ACME or self-signed)
- **Self-signed CA** — Import existing CA certificates, configure default TLS store, per-service cert overrides
- **Middleware management** — Built-in support for headers, rate-limit, redirect-scheme, basic-auth, strip-prefix, compress
- **Security presets** — One-flag hardened security headers (HSTS, X-Frame-Options, XSS protection, referrer policy)
- **Doctor command** — Auto-checks and fixes Traefik static config (`traefik.yml`) before any mutation
- **Dry-run mode** — Preview YAML output without writing files
- **Idempotent operations** — `--force` flag for overwriting existing routes
- **Hot-reload** — Optional `--reload` triggers Traefik systemctl reload after changes
- **Deterministic output** — BTreeMap-based serialization for stable YAML key ordering
- **Validation** — Strict DNS hostname, URL, address, and name validation before any filesystem write
- **mTLS support** — Mutual TLS with client certificate verification via CA files
- **ACME DNS-01** — Automated Let's Encrypt certificate setup with DNS-01 challenge (195+ DNS providers via lego)
- **Zero runtime** — Single static binary, no daemon, no database

## Installation

### Pre-built binaries

Download the latest release from [GitHub Releases](https://github.com/antronic/traefikctl/releases):

```bash
# Linux x86_64
curl -L https://github.com/antronic/traefikctl/releases/latest/download/traefikctl-linux-x86_64 -o traefikctl
chmod +x traefikctl
sudo mv traefikctl /usr/local/bin/

# Linux aarch64
curl -L https://github.com/antronic/traefikctl/releases/latest/download/traefikctl-linux-aarch64 -o traefikctl
chmod +x traefikctl
sudo mv traefikctl /usr/local/bin/

# macOS (Apple Silicon)
curl -L https://github.com/antronic/traefikctl/releases/latest/download/traefikctl-darwin-aarch64 -o traefikctl
chmod +x traefikctl
sudo mv traefikctl /usr/local/bin/
```

### From source

```bash
cargo install --path .
```

## Usage

### Routes

```bash
# Add a route
traefikctl add -n my-app -H app.example.com -u http://127.0.0.1:3000

# Add with TLS + middlewares
traefikctl add -n secure-app -H secure.example.com -u http://10.0.0.5:8443 \
  --tls --cert-resolver letsencrypt --middlewares "rate-limit,headers"

# List all routes and middlewares
traefikctl list

# Update an existing route
traefikctl update -n my-app -H new-domain.example.com -u http://127.0.0.1:4000

# Remove a route (with confirmation prompt)
traefikctl remove -n my-app

# Remove without confirmation
traefikctl --force remove -n my-app
```

### TCP / UDP Routes

```bash
# TCP route — PostgreSQL proxy
traefikctl add -n postgres --protocol tcp --host db.internal.local \
  --address 10.0.0.5:5432 --entrypoint postgres

# TCP with TLS passthrough (terminates TLS at the backend)
traefikctl add -n secure-db --protocol tcp --host db.internal.local \
  --address 10.0.0.5:5432 --tls --tls-passthrough

# TCP without SNI host (catch-all HostSNI(`*`))
traefikctl add -n redis --protocol tcp --address 10.0.0.5:6379 --entrypoint redis

# UDP route — DNS proxy (no rule, no TLS)
traefikctl add -n dns --protocol udp --address 10.0.0.5:53 --entrypoint dns

# List shows protocol labels: [HTTP], [TCP], [UDP]
traefikctl list
```

TCP/UDP differences from HTTP:
- **TCP** uses `HostSNI(...)` rules instead of `Host(...)`. Without `--host`, defaults to `HostSNI(\`*\`)` (catch-all)
- **TCP** supports `--tls-passthrough` for end-to-end encryption without termination
- **UDP** has no routing rules, no TLS, and no middleware — only entrypoints and backend address
- Backend is specified via `--address host:port` instead of `--url http://...`

### Service Presets

Presets auto-configure protocol, default port, and entrypoint for common infrastructure:

```bash
# PostgreSQL (TCP, port 5432, entrypoint "postgres")
traefikctl add -n my-postgres --preset postgres --address 10.0.0.5:5432

# Redis with default address (127.0.0.1:6379)
traefikctl add -n my-redis --preset redis

# MySQL / MariaDB (TCP, port 3306, entrypoint "mysql")
traefikctl add -n my-mysql --preset mysql --address db-server:3306

# MongoDB (TCP, port 27017, entrypoint "mongodb")
traefikctl add -n my-mongo --preset mongodb --address 10.0.0.5:27017

# DNS (UDP, port 53, entrypoint "dns")
traefikctl add -n my-dns --preset dns --address 10.0.0.5:53

# MQTT (TCP, port 1883, entrypoint "mqtt")
traefikctl add -n my-mqtt --preset mqtt --address broker:1883

# NATS (TCP, port 4222, entrypoint "nats")
traefikctl add -n my-nats --preset nats

# Syslog (UDP, port 514, entrypoint "syslog")
traefikctl add -n my-syslog --preset syslog --address log-server:514
```

| Preset | Protocol | Default Port | Entrypoint |
|--------|----------|:------------:|------------|
| `postgres` | TCP | 5432 | postgres |
| `mysql` | TCP | 3306 | mysql |
| `mariadb` | TCP | 3306 | mysql |
| `redis` | TCP | 6379 | redis |
| `mongodb` | TCP | 27017 | mongodb |
| `dns` | UDP | 53 | dns |
| `mqtt` | TCP | 1883 | mqtt |
| `nats` | TCP | 4222 | nats |
| `syslog` | UDP | 514 | syslog |

When using presets, `--address` defaults to `127.0.0.1:<default-port>` if omitted.

### Middlewares

```bash
# Security-hardened headers (HSTS, X-Frame, XSS, referrer policy)
traefikctl add-middleware -n headers -t headers --security-preset

# Rate limiting
traefikctl add-middleware -n rate-limit -t rate-limit --average 100 --burst 50

# HTTPS redirect
traefikctl add-middleware -n https-redirect -t redirect-scheme --scheme https --permanent true

# Basic auth
traefikctl add-middleware -n auth -t basic-auth --user "admin:$$apr1$$..." --realm "Protected"

# Strip URL prefix
traefikctl add-middleware -n strip-api -t strip-prefix --prefix /api

# Compression
traefikctl add-middleware -n compress -t compress

# Remove a middleware
traefikctl remove-middleware -n rate-limit
```

### ACME / TLS Certificates

```bash
# Set up Let's Encrypt DNS-01 with Cloudflare
traefikctl init-acme --email admin@example.com --provider cloudflare

# Use staging CA for testing
traefikctl init-acme --email admin@example.com --provider cloudflare --staging

# Custom resolver name, key type, DNS resolvers
traefikctl init-acme --email admin@example.com --provider route53 \
  --resolver-name myresolver --key-type EC256 \
  --dns-resolver 1.1.1.1:53 --dns-resolver 8.8.8.8:53

# Disable propagation checks (useful for split-horizon DNS)
traefikctl init-acme --email admin@example.com --provider hetzner \
  --disable-propagation-check

# Then add routes with TLS
traefikctl add -n my-app -H app.example.com -u http://127.0.0.1:3000 --tls
```

Supported DNS providers include: cloudflare, route53, digitalocean, hetzner, ovh, gandiv5, gcloud, azuredns, and [195+ more via lego](https://go-acme.github.io/lego/dns/). Provider credentials are passed via environment variables in Traefik's runtime (systemd unit, docker-compose, etc.).

### Self-Signed CA / mTLS

For internal services using self-signed certificates:

```bash
# Import existing CA and default server certificate
traefikctl init-ca \
  --ca-cert /path/to/root-ca.crt \
  --cert /path/to/server.crt \
  --key /path/to/server.key

# With intermediate CA
traefikctl init-ca \
  --ca-cert /path/to/root-ca.crt \
  --intermediate-cert /path/to/intermediate-ca.crt \
  --cert /path/to/server.crt \
  --key /path/to/server.key

# Enable mTLS (mutual TLS — require client certificates)
traefikctl init-ca \
  --ca-cert /path/to/root-ca.crt \
  --cert /path/to/server.crt \
  --key /path/to/server.key \
  --mtls --min-version VersionTLS12

# Custom certificate storage directory
traefikctl init-ca \
  --ca-cert /path/to/root-ca.crt \
  --cert /path/to/server.crt \
  --key /path/to/server.key \
  --certs-dir /opt/traefik/certs

# Add a per-service certificate override
traefikctl add-cert -n my-app --cert /path/to/app.crt --key /path/to/app.key

# Then add a route (TLS auto-enabled with self-signed)
traefikctl add -n my-app -H app.internal.local -u http://127.0.0.1:3000 --tls
```

`init-ca` creates:
- `<certs-dir>/ca/root-ca.crt` — Root CA certificate
- `<certs-dir>/ca/intermediate-ca.crt` — Intermediate CA (if provided)
- `<certs-dir>/default.crt` + `default.key` — Default server certificate
- `<conf.d>/tls/tls-default.yml` — Traefik dynamic TLS config (default store + certificates + mTLS options)

`add-cert` creates:
- `<certs-dir>/services/<name>/<name>.crt` + `<name>.key` — Per-service certificate
- Updates `<name>.yml` with `tls.certificates` section and enables TLS on the router

### Doctor

```bash
# Check and fix Traefik static config
traefikctl doctor

# Doctor auto-runs before add, update, and remove
# It ensures:
#   1. /etc/traefik/conf.d directory exists
#   2. traefik.yml has providers.file.directory pointing to conf.d
#   3. providers.file.watch is enabled
#   4. Reports ACME resolver status (if configured via init-acme)
```

### Global Flags

| Flag | Description |
|------|-------------|
| `--dir <PATH>` | Config directory (default: `/etc/traefik/conf.d`) |
| `--traefik-config <PATH>` | Traefik static config path (auto-detected if omitted) |
| `--reload` | Reload Traefik via systemctl after changes |
| `--force` | Skip confirmation prompts / overwrite existing files |
| `--dry-run` | Print YAML output without writing files |

## Example Output

### HTTP Route

```bash
$ traefikctl --dry-run add -n my-app -H app.example.com -u http://127.0.0.1:3000 --tls
```

```yaml
http:
  routers:
    my-app:
      rule: Host(`app.example.com`)
      entryPoints:
      - web
      - websecure
      service: my-app
      tls:
        certResolver: letsencrypt
  services:
    my-app:
      loadBalancer:
        servers:
        - url: http://127.0.0.1:3000
```

### TCP Route (Preset)

```bash
$ traefikctl --dry-run add -n my-postgres --preset postgres --address 10.0.0.5:5432
```

```yaml
tcp:
  routers:
    my-postgres:
      rule: HostSNI(`*`)
      entryPoints:
      - postgres
      service: my-postgres
  services:
    my-postgres:
      loadBalancer:
        servers:
        - address: 10.0.0.5:5432
```

## Traefik Setup

`traefikctl doctor` automatically ensures the Traefik static config (`/etc/traefik/traefik.yml`) has the file provider pointing to the route directory. It runs automatically before `add`, `update`, and `remove`.

To set up manually:

```yaml
# /etc/traefik/traefik.yml
entryPoints:
  web:
    address: ":80"
  websecure:
    address: ":443"
  # Add entrypoints for TCP/UDP services as needed:
  postgres:
    address: ":5432"
  redis:
    address: ":6379"
  dns:
    address: ":53/udp"    # UDP requires /udp suffix

providers:
  file:
    directory: /etc/traefik/conf.d
    watch: true
```

Each route gets its own file (`<name>.yml`) and each middleware gets `mw-<name>.yml` in the `conf.d/` directory. Traefik watches the directory and hot-reloads on any change.

## Build from Source

Requires [Rust](https://rustup.rs/) 1.70+.

```bash
cargo build --release
# binary at target/release/traefikctl
```

### Cross-compilation

All builds output to `dist/<os>-<arch>/traefikctl`:

```bash
make build              # native release → dist/<os>-<arch>/traefikctl
make build-linux-x86    # Docker cross-compile → dist/linux-x86_64/traefikctl
make build-linux-arm    # Docker cross-compile → dist/linux-aarch64/traefikctl
make all                # native + linux-x86
make clean              # remove dist/ and target/
```

## Roadmap

- [ ] Integration with official [Traefik CLI](https://doc.traefik.io/traefik/) for config validation
- [ ] `import` command — convert existing Docker labels / Kubernetes IngressRoute to file provider YAML
- [ ] `export` command — generate Docker Compose labels from existing routes
- [ ] Middleware chaining templates (e.g. `--preset production` = headers + rate-limit + compress)
- [x] ~~Support for TCP/UDP routers~~ — Added in v0.2.0 with service presets
- [ ] `status` command — query Traefik API for route health and active connections

## License

MIT
