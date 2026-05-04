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
traefikctl remove -n <name>
traefikctl list
traefikctl update -n <name> [-H <host>] [-u <url>] [--tls true|false]
traefikctl add-middleware -n <name> -t <type> [type-specific flags]
traefikctl remove-middleware -n <name>
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

## Build Targets

```bash
make build              # native → dist/<os>-<arch>/traefikctl
make build-linux-x86    # Docker → dist/linux-x86_64/traefikctl
make build-linux-arm    # Docker → dist/linux-aarch64/traefikctl
make all                # native + linux-x86
```

## Project Structure

```
src/
├── main.rs               # CLI dispatch, pre-mutation doctor check, post-command reload
├── cli.rs                # clap v4 derive definitions + MiddlewareType enum
├── config.rs             # Traefik dynamic + static + middleware config serde structs
├── validation.rs         # Input validation with unit tests
├── traefik.rs            # systemctl reload/restart
└── commands/             # doctor, add, remove, list, update, add_middleware, remove_middleware
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
