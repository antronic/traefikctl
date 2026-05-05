mod cli;
mod commands;
mod config;
mod traefik;
mod validation;

use anyhow::Result;
use clap::Parser;
use colored::Colorize;

use cli::{Cli, Commands};

fn main() -> Result<()> {
    let cli = Cli::parse();

    let is_mutating = matches!(
        cli.command,
        Commands::Add { .. }
            | Commands::Remove { .. }
            | Commands::Update { .. }
            | Commands::AddMiddleware { .. }
            | Commands::RemoveMiddleware { .. }
            | Commands::AddCert { .. }
    );

    // init-acme modifies static config directly, skip doctor for it
    if is_mutating && !cli.dry_run {
        commands::doctor::ensure_setup(&cli.dir, cli.traefik_config.as_deref(), false)?;
    }

    let result = match &cli.command {
        Commands::Add {
            name,
            host,
            url,
            address,
            entrypoint,
            protocol,
            preset,
            tls,
            tls_passthrough,
            cert_resolver,
            middlewares,
        } => commands::add::execute(
            &cli.dir,
            commands::add::AddOptions {
                name,
                host: host.as_deref(),
                url: url.as_deref(),
                address: address.as_deref(),
                entrypoint: entrypoint.as_deref(),
                protocol: *protocol,
                preset: *preset,
                tls: *tls,
                tls_passthrough: *tls_passthrough,
                cert_resolver: cert_resolver.as_deref(),
                middlewares: middlewares.as_deref(),
                force: cli.force,
                dry_run: cli.dry_run,
            },
        ),

        Commands::Remove { name } => commands::remove::execute(
            &cli.dir,
            commands::remove::RemoveOptions {
                name,
                force: cli.force,
                dry_run: cli.dry_run,
            },
        ),

        Commands::List => commands::list::execute(&cli.dir),

        Commands::Update {
            name,
            host,
            url,
            entrypoint,
            tls,
            middlewares,
        } => commands::update::execute(
            &cli.dir,
            commands::update::UpdateOptions {
                name,
                host: host.as_deref(),
                url: url.as_deref(),
                entrypoint: entrypoint.as_deref(),
                tls: *tls,
                middlewares: middlewares.as_deref(),
                dry_run: cli.dry_run,
            },
        ),

        Commands::AddMiddleware {
            name,
            mw_type,
            security_preset,
            sts_seconds,
            frame_deny,
            referrer_policy,
            response_headers,
            request_headers,
            average,
            burst,
            period,
            scheme,
            permanent,
            users,
            realm,
            prefixes,
        } => commands::add_middleware::execute(
            &cli.dir,
            commands::add_middleware::AddMiddlewareOptions {
                name,
                mw_type,
                security_preset: *security_preset,
                sts_seconds: *sts_seconds,
                frame_deny: *frame_deny,
                referrer_policy: referrer_policy.as_deref(),
                response_headers,
                request_headers,
                average: *average,
                burst: *burst,
                period: period.as_deref(),
                scheme: scheme.as_deref(),
                permanent: *permanent,
                users,
                realm: realm.as_deref(),
                prefixes,
                force: cli.force,
                dry_run: cli.dry_run,
            },
        ),

        Commands::RemoveMiddleware { name } => commands::remove_middleware::execute(
            &cli.dir,
            commands::remove_middleware::RemoveMiddlewareOptions {
                name,
                force: cli.force,
                dry_run: cli.dry_run,
            },
        ),

        Commands::InitAcme {
            resolver_name,
            email,
            provider,
            staging,
            storage,
            dns_resolvers,
            key_type,
            propagation_delay,
            disable_propagation_check,
        } => commands::init_acme::execute(commands::init_acme::InitAcmeOptions {
            resolver_name,
            email,
            provider,
            staging: *staging,
            storage,
            dns_resolvers,
            key_type: key_type.as_deref(),
            propagation_delay: *propagation_delay,
            disable_propagation_check: *disable_propagation_check,
            force: cli.force,
            dry_run: cli.dry_run,
            traefik_config: cli.traefik_config.as_deref(),
        }),

        Commands::InitCa {
            ca_cert,
            intermediate_cert,
            cert,
            key,
            certs_dir,
            mtls,
            min_version,
        } => commands::init_ca::execute(commands::init_ca::InitCaOptions {
            ca_cert,
            intermediate_cert: intermediate_cert.as_deref(),
            cert,
            key,
            certs_dir,
            mtls: *mtls,
            min_version: min_version.as_deref(),
            force: cli.force,
            dry_run: cli.dry_run,
            conf_dir: &cli.dir,
        }),

        Commands::AddCert {
            name,
            cert,
            key,
            certs_dir,
        } => commands::add_cert::execute(
            &cli.dir,
            commands::add_cert::AddCertOptions {
                name,
                cert,
                key,
                certs_dir,
                force: cli.force,
                dry_run: cli.dry_run,
            },
        ),

        Commands::Doctor => {
            commands::doctor::execute(&cli.dir, cli.traefik_config.as_deref(), cli.dry_run)
        }
    };

    // Reload traefik if requested and command succeeded
    if result.is_ok() && cli.reload && !cli.dry_run {
        // Only reload for mutating commands
        let should_reload = !matches!(cli.command, Commands::List);
        if should_reload {
            print!("  {} reloading traefik... ", "↻".dimmed());
            match traefik::reload_traefik() {
                Ok(()) => println!("{}", "done".green()),
                Err(e) => eprintln!("{} {e}", "failed:".red().bold()),
            }
        }
    }

    result
}
