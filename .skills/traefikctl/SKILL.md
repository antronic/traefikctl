---
name: traefikctl
description: Manage Traefik reverse proxy dynamic configuration via CLI. Use when adding, removing, listing, or updating Traefik routes, working with Traefik file provider YAML, cross-compiling Rust CLI tools, or managing reverse proxy configurations.
---

# traefikctl Skill

CLI tool for managing Traefik dynamic configuration files. Generates per-service YAML that Traefik's file provider watches and hot-reloads.

## When to Use

- Adding/removing/updating Traefik routes
- Generating Traefik file provider YAML
- Cross-compiling Rust binaries for Linux x86_64/arm64
- Debugging Traefik dynamic config structure
- Setting up Traefik file provider directory

## Commands

```bash
traefikctl doctor                                                      # check/fix static config
traefikctl add -n <name> -H <host> -u <backend-url> [--tls] [--middlewares "a,b"]
traefikctl add -n <name> --protocol tcp -H <host> --address <host:port> --entrypoint <ep> [--tls] [--tls-passthrough]
traefikctl add -n <name> --protocol udp --address <host:port> --entrypoint <ep>
traefikctl add -n <name> --preset postgres --address <host:port>       # service presets
traefikctl remove -n <name>
traefikctl list
traefikctl update -n <name> [-H <host>] [-u <url>] [--tls true|false]
traefikctl add-middleware -n <name> -t <type> [type-specific flags]
traefikctl remove-middleware -n <name>
traefikctl init-acme --email <email> --provider <dns-provider> [--staging] [--resolver-name <name>]
traefikctl init-ca --ca-cert <path> --cert <path> --key <path> [--intermediate-cert <path>] [--mtls]
traefikctl add-cert -n <name> --cert <path> --key <path> [--certs-dir <path>]
```

`doctor` runs automatically before all mutating commands.

### Middleware Types

| Type | Key Flags |
|---|---|
| `headers` | `--security-preset`, `--sts-seconds`, `--frame-deny`, `--referrer-policy`, `--response-header KEY=VALUE` |
| `rate-limit` | `--average` (required), `--burst`, `--period` |
| `redirect-scheme` | `--scheme` (required), `--permanent` |
| `basic-auth` | `--user` (repeatable, required), `--realm` |
| `strip-prefix` | `--prefix` (repeatable, required) |
| `compress` | (no required flags) |

### Global Flags

| Flag | Effect |
|---|---|
| `--dir <path>` | Config directory (default: `/etc/traefik/conf.d`) |
| `--traefik-config <path>` | Traefik static config path (auto-detected if omitted) |
| `--reload` | `systemctl reload traefik` after changes |
| `--force` | Skip confirmation prompts, overwrite existing |
| `--dry-run` | Print YAML without writing files |

## Traefik YAML Schema

The tool generates files matching Traefik's file provider format. Field names use camelCase per Traefik's Go struct YAML tags:

```yaml
http:
  routers:
    <name>:
      rule: "Host(`<domain>`)"
      entryPoints:
        - web
        - websecure        # added when --tls
      service: <name>
      tls:                  # only when --tls
        certResolver: letsencrypt
      middlewares:           # only when --middlewares
        - rate-limit
        - headers
  services:
    <name>:
      loadBalancer:
        servers:
          - url: http://127.0.0.1:3000
```

### TCP Route

```yaml
tcp:
  routers:
    <name>:
      rule: "HostSNI(`<host>`)"   # HostSNI(`*`) when no --host
      entryPoints:
        - postgres
      service: <name>
      tls:                        # only when --tls
        passthrough: true         # only when --tls-passthrough
  services:
    <name>:
      loadBalancer:
        servers:
          - address: "10.0.0.5:5432"
```

### UDP Route

```yaml
udp:
  routers:
    <name>:                       # no rule for UDP
      entryPoints:
        - dns
      service: <name>
  services:
    <name>:
      loadBalancer:
        servers:
          - address: "10.0.0.5:53"
```

### Service Presets

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

## Build Targets

```bash
make build              # native → dist/<os>-<arch>/traefikctl
make build-linux-x86    # Docker → dist/linux-x86_64/traefikctl
make build-linux-arm    # Docker → dist/linux-aarch64/traefikctl
make all                # native + linux-x86
```

## ACME DNS-01 Setup

`init-acme` configures `certificatesResolvers` in Traefik's static config:

| Flag | Description |
|---|---|
| `--email` | ACME account email (required) |
| `--provider` | DNS provider code — cloudflare, route53, digitalocean, hetzner, etc. |
| `--resolver-name` | Resolver name (default: `letsencrypt`) |
| `--staging` | Use Let's Encrypt staging CA |
| `--storage` | ACME storage path (default: `/etc/traefik/acme.json`) |
| `--dns-resolver` | Custom DNS resolvers (repeatable, e.g. `1.1.1.1:53`) |
| `--key-type` | Key type: RSA2048, RSA4096, EC256, EC384 |
| `--propagation-delay` | Seconds to wait before propagation check |
| `--disable-propagation-check` | Skip DNS propagation verification |

Provider credentials are set via environment variables in Traefik's runtime. The tool warns about missing env vars for known providers.

## Self-Signed CA / mTLS

`init-ca` imports existing CA certificates and configures Traefik's TLS default store:

| Flag | Description |
|---|---|
| `--ca-cert` | Root CA certificate path (required) |
| `--intermediate-cert` | Intermediate CA certificate path (optional) |
| `--cert` | Default server certificate path (required) |
| `--key` | Default server key path (required) |
| `--certs-dir` | Certificate storage directory (default: `/etc/traefik/certs`) |
| `--mtls` | Enable mutual TLS (require client certificates) |
| `--min-version` | Minimum TLS version (e.g. `VersionTLS12`, `VersionTLS13`) |

Creates `tls-default.yml` in conf.d with `tls.stores.default.defaultCertificate`, `tls.certificates`, and optional `tls.options.default.clientAuth`.

`add-cert` adds per-service TLS certificate overrides:

| Flag | Description |
|---|---|
| `-n/--name` | Route name (must match existing route) |
| `--cert` | Service certificate path (required) |
| `--key` | Service key path (required) |
| `--certs-dir` | Certificate storage directory (default: `/etc/traefik/certs`) |

Copies certs to `<certs-dir>/services/<name>/` and updates the route YAML with `tls.certificates`.

## Project Structure

```
src/
├── main.rs               # CLI dispatch, pre-mutation doctor check, post-command reload
├── cli.rs                # clap v4 derive definitions + MiddlewareType + Protocol + ServicePreset + InitAcme + InitCa + AddCert
├── config.rs             # Traefik dynamic (HTTP/TCP/UDP) + static + middleware + ACME + TLS config serde structs
├── validation.rs         # Input validation with unit tests
├── traefik.rs            # systemctl reload/restart
└── commands/             # doctor, add, remove, list, update, add_middleware, remove_middleware, init_acme, init_ca, add_cert
```

## Key Implementation Details

- **serde_yaml** with `#[serde(rename_all = "camelCase")]` for Traefik-compatible output
- **BTreeMap** (not HashMap) for deterministic YAML key ordering
- **url crate** for backend URL validation (http/https only)
- **Per-label DNS validation** in validate_host (rejects `bad-.com`)
- Validation always runs before any filesystem mutation
- `--dry-run` is checked before every write operation

## Traefik Static Config Setup

`traefikctl doctor` auto-validates and fixes the static config. It:
- Creates the conf.d directory if missing
- Finds traefik.yml/yaml at `/etc/traefik/` (or uses `--traefik-config`)
- Ensures `providers.file.directory` points to `--dir` with `watch: true`
- Preserves existing config keys (uses `#[serde(flatten)]` with `serde_yaml::Value`)
- Runs automatically before every `add`/`update`/`remove`

Manual setup:

```yaml
providers:
  file:
    directory: /etc/traefik/conf.d
    watch: true
```
