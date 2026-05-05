use anyhow::{bail, Result};
use url::Url;

/// Validate a hostname (domain) for use in a Traefik Host() rule.
///
/// Accepts simple domain names like `app.example.com`.
/// Rejects empty, whitespace-only, or domains with invalid characters.
pub fn validate_host(host: &str) -> Result<()> {
    let host = host.trim();

    if host.is_empty() {
        bail!("host cannot be empty");
    }

    // Must not contain whitespace
    if host.chars().any(|c| c.is_whitespace()) {
        bail!("host cannot contain whitespace: {host:?}");
    }

    // Must not start or end with a dot or hyphen
    if host.starts_with('.') || host.starts_with('-') {
        bail!("host cannot start with '.' or '-': {host:?}");
    }
    if host.ends_with('.') || host.ends_with('-') {
        bail!("host cannot end with '.' or '-': {host:?}");
    }

    if !host.contains('.') {
        bail!("host must be a fully qualified domain name (e.g. app.example.com): {host:?}");
    }

    for label in host.split('.') {
        if label.is_empty() {
            bail!("host contains an empty label (consecutive dots): {host:?}");
        }
        if label.starts_with('-') || label.ends_with('-') {
            bail!("host label {label:?} cannot start or end with '-': {host:?}");
        }
        for ch in label.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '-' {
                bail!("host contains invalid character {ch:?}: {host:?}");
            }
        }
    }

    Ok(())
}

/// Validate a backend URL (must be a valid HTTP/HTTPS URL).
pub fn validate_url(raw: &str) -> Result<()> {
    let raw = raw.trim();

    if raw.is_empty() {
        bail!("backend URL cannot be empty");
    }

    let parsed =
        Url::parse(raw).map_err(|e| anyhow::anyhow!("invalid backend URL {raw:?}: {e}"))?;

    match parsed.scheme() {
        "http" | "https" => {}
        other => bail!("backend URL must use http:// or https:// scheme, got {other:?}"),
    }

    if parsed.host_str().is_none() {
        bail!("backend URL is missing a host: {raw:?}");
    }

    Ok(())
}

/// Validate a TCP/UDP backend address (host:port format).
pub fn validate_address(addr: &str) -> Result<()> {
    let addr = addr.trim();

    if addr.is_empty() {
        bail!("backend address cannot be empty");
    }

    let parts: Vec<&str> = addr.rsplitn(2, ':').collect();
    if parts.len() != 2 {
        bail!("backend address must be in host:port format (e.g. 127.0.0.1:5432): {addr:?}");
    }

    let port_str = parts[0];
    let host = parts[1];

    if host.is_empty() {
        bail!("backend address is missing host: {addr:?}");
    }

    let port: u16 = port_str
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid port number in address {addr:?}"))?;

    if port == 0 {
        bail!("port cannot be 0 in address {addr:?}");
    }

    // Allow IP addresses and hostnames
    if host.chars().any(|c| c.is_whitespace()) {
        bail!("address host cannot contain whitespace: {addr:?}");
    }

    Ok(())
}

/// Validate a service/route name (alphanumeric, hyphens, underscores).
pub fn validate_name(name: &str) -> Result<()> {
    let name = name.trim();

    if name.is_empty() {
        bail!("route name cannot be empty");
    }

    if name.len() > 128 {
        bail!("route name too long (max 128 characters): {name:?}");
    }

    for ch in name.chars() {
        if !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_' {
            bail!(
                "route name contains invalid character {ch:?} (allowed: a-z, 0-9, -, _): {name:?}"
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_hosts() {
        assert!(validate_host("app.example.com").is_ok());
        assert!(validate_host("sub.domain.example.org").is_ok());
        assert!(validate_host("my-app.test.io").is_ok());
    }

    #[test]
    fn invalid_hosts() {
        assert!(validate_host("").is_err());
        assert!(validate_host("localhost").is_err());
        assert!(validate_host(".bad.com").is_err());
        assert!(validate_host("bad-.com").is_err());
        assert!(validate_host("has space.com").is_err());
        assert!(validate_host("bad!char.com").is_err());
    }

    #[test]
    fn valid_urls() {
        assert!(validate_url("http://127.0.0.1:3000").is_ok());
        assert!(validate_url("https://backend.local:8080").is_ok());
        assert!(validate_url("http://localhost").is_ok());
    }

    #[test]
    fn invalid_urls() {
        assert!(validate_url("").is_err());
        assert!(validate_url("ftp://nope.com").is_err());
        assert!(validate_url("not-a-url").is_err());
    }

    #[test]
    fn valid_names() {
        assert!(validate_name("my-app").is_ok());
        assert!(validate_name("web_service_1").is_ok());
        assert!(validate_name("api").is_ok());
    }

    #[test]
    fn valid_addresses() {
        assert!(validate_address("127.0.0.1:5432").is_ok());
        assert!(validate_address("postgres.local:5432").is_ok());
        assert!(validate_address("192.168.1.100:6379").is_ok());
        assert!(validate_address("[::1]:8080").is_ok());
    }

    #[test]
    fn invalid_addresses() {
        assert!(validate_address("").is_err());
        assert!(validate_address("no-port").is_err());
        assert!(validate_address(":5432").is_err());
        assert!(validate_address("host:0").is_err());
        assert!(validate_address("host:99999").is_err());
        assert!(validate_address("host:abc").is_err());
        assert!(validate_address("has space:80").is_err());
    }

    #[test]
    fn invalid_names() {
        assert!(validate_name("").is_err());
        assert!(validate_name("has space").is_err());
        assert!(validate_name("bad!name").is_err());
    }
}
