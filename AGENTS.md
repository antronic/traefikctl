# traefikctl

Rust CLI tool that manages Traefik reverse proxy dynamic configuration via the file provider. One YAML file per route in a watched directory.

## Architecture

Single-binary CLI. No runtime, no daemon, no database.

```
src/
├── main.rs               # Entry point, CLI dispatch, pre-mutation doctor check, post-command reload
├── cli.rs                # clap derive structs (Cli + Commands enum + MiddlewareType)
├── config.rs             # Traefik dynamic + static + middleware config types (serde)
├── validation.rs         # Host, URL, and name validation (with unit tests)
├── traefik.rs            # systemctl reload/restart wrapper
└── commands/
    ├── mod.rs
    ├── add.rs            # Create route YAML file (idempotent with --force)
    ├── add_middleware.rs  # Create middleware YAML file (headers, rate-limit, etc.)
    ├── remove.rs         # Delete route file (confirmation prompt unless --force)
    ├── remove_middleware.rs # Delete middleware file
    ├── list.rs           # Read + parse all .yml files, print routes + middlewares
    ├── update.rs         # Partial update of existing route file
    └── doctor.rs         # Check/fix Traefik static config + ensure conf.d dir exists
```

## Key patterns

- **One file per route**: `<dir>/<name>.yml` — Traefik watches the directory.
- **One file per middleware**: `<dir>/mw-<name>.yml` — same directory, `mw-` prefix distinguishes from routes.
- **serde camelCase**: `#[serde(rename_all = "camelCase")]` matches Traefik's Go YAML tags (`entryPoints`, `loadBalancer`, `certResolver`).
- **BTreeMap for deterministic output**: Routers/services use BTreeMap so YAML key order is stable.
- **Validation before mutation**: All inputs validated (validate_name, validate_host, validate_url) before any filesystem writes.
- **Dry-run is first-class**: Every mutating command checks `dry_run` and prints YAML without writing.
- **Doctor before mutation**: `add`/`update`/`remove` auto-run `doctor::ensure_setup` to validate Traefik static config and create the conf.d directory.

## Build

```bash
make build              # native release binary → dist/<os>-<arch>/traefikctl
make build-linux-x86    # Docker cross-compile → dist/linux-x86_64/traefikctl
make build-linux-arm    # Docker cross-compile → dist/linux-aarch64/traefikctl
make all                # native + linux-x86
```

## Dependencies

clap 4 (derive), serde + serde_yaml, anyhow, url, colored. No async runtime.

## YAML output structure

```yaml
http:
  routers:
    <name>:
      rule: "Host(`<host>`)"
      entryPoints: [web]          # + websecure when --tls
      service: <name>
      tls:                        # only when --tls
        certResolver: letsencrypt
      middlewares: [...]           # only when --middlewares
  services:
    <name>:
      loadBalancer:
        servers:
          - url: <backend-url>
```

## Rules

- Never suppress type errors.
- Validation lives in validation.rs — don't scatter it across commands.
- All mutating commands must respect --dry-run and --force flags.
- Config structs must match Traefik's exact camelCase field names.
- BTreeMap, not HashMap, for serialized output.
