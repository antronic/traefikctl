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
  - [Middlewares](#middlewares)
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
- **TLS support** — Automatic `websecure` entrypoint + cert resolver configuration
- **Middleware management** — Built-in support for headers, rate-limit, redirect-scheme, basic-auth, strip-prefix, compress
- **Security presets** — One-flag hardened security headers (HSTS, X-Frame-Options, XSS protection, referrer policy)
- **Doctor command** — Auto-checks and fixes Traefik static config (`traefik.yml`) before any mutation
- **Dry-run mode** — Preview YAML output without writing files
- **Idempotent operations** — `--force` flag for overwriting existing routes
- **Hot-reload** — Optional `--reload` triggers Traefik systemctl reload after changes
- **Deterministic output** — BTreeMap-based serialization for stable YAML key ordering
- **Validation** — Strict DNS hostname, URL, and name validation before any filesystem write
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

### Doctor

```bash
# Check and fix Traefik static config
traefikctl doctor

# Doctor auto-runs before add, update, and remove
# It ensures:
#   1. /etc/traefik/conf.d directory exists
#   2. traefik.yml has providers.file.directory pointing to conf.d
#   3. providers.file.watch is enabled
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

## Traefik Setup

`traefikctl doctor` automatically ensures the Traefik static config (`/etc/traefik/traefik.yml`) has the file provider pointing to the route directory. It runs automatically before `add`, `update`, and `remove`.

To set up manually:

```yaml
# /etc/traefik/traefik.yml
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
- [ ] Support for TCP/UDP routers
- [ ] `status` command — query Traefik API for route health and active connections

## License

MIT
