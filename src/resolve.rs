use tokio::net::lookup_host;
use url::Url;

/// Resolve a URL to a socket address
pub async fn resolve_address_from_url(url: &str) -> std::io::Result<std::net::SocketAddr> {
    let url = Url::parse(url).map_err(|e| std::io::Error::other(format!("Invalid URL: {e}")))?;

    let host = url
        .host_str()
        .ok_or_else(|| std::io::Error::other("Missing host"))?;
    let port = url
        .port()
        .ok_or_else(|| std::io::Error::other("Missing port"))?;

    let maybe_addr = lookup_host(format!("{host}:{port}")).await?.next();

    match maybe_addr {
        Some(addr) => Ok(addr),
        None => Err(std::io::Error::other("No address found")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn valid_url() {
        let url = "http://localhost:8080";
        let addr = resolve_address_from_url(url).await.unwrap();
        assert_eq!(addr.port(), 8080);
    }

    #[tokio::test]
    async fn missing_port() {
        // Interestingly, this URL doesn't have a host either... so the URL parsing
        // is actually not entirely compliant.
        let url = "http:///some-path";
        let err = resolve_address_from_url(url).await.unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::Other);
        assert!(err.to_string().contains("Missing port"));
    }

    #[tokio::test]
    async fn invalid_host() {
        let url = "http://something-that-is-not-a-real-host:8080";
        resolve_address_from_url(url).await.unwrap_err();
    }
}
