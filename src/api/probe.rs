use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::config::server::ServerConfig;

#[derive(Debug, thiserror::Error)]
pub enum ProbeError {
    #[error("no answer from {0}: {1}")]
    Io(SocketAddr, std::io::Error),
    #[error("no answer from {0} within {1:?}")]
    Timeout(SocketAddr, Duration),
    #[error("{0} is not ready: {1}")]
    NotReady(SocketAddr, String),
}

/// The container's health probe: `GET /ready` on the server's own address, for an image with no shell and no curl.
#[derive(Debug)]
pub struct HttpProbe;

impl HttpProbe {
    const TIMEOUT: Duration = Duration::from_secs(2);

    /// A wildcard bind is reached on loopback of the same family.
    fn address(server: &ServerConfig) -> SocketAddr {
        let host = match server.host {
            IpAddr::V4(host) if host.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
            IpAddr::V6(host) if host.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
            host => host,
        };
        SocketAddr::new(host, server.port)
    }

    async fn exchange(address: SocketAddr) -> std::io::Result<String> {
        let mut stream = TcpStream::connect(address).await?;
        stream
            .write_all(b"GET /ready HTTP/1.0\r\nHost: probe\r\nConnection: close\r\n\r\n")
            .await?;
        let mut response = String::new();
        stream.read_to_string(&mut response).await?;
        Ok(response)
    }

    /// Ok once the server answers 200 on `/ready`.
    ///
    /// # Errors
    /// Nothing listens (`Io`), no answer within two seconds (`Timeout`), any other status (`NotReady`).
    pub async fn ready(server: &ServerConfig) -> Result<(), ProbeError> {
        let address = Self::address(server);
        let response = tokio::time::timeout(Self::TIMEOUT, Self::exchange(address))
            .await
            .map_err(|_| ProbeError::Timeout(address, Self::TIMEOUT))?
            .map_err(|error| ProbeError::Io(address, error))?;
        let status = response.lines().next().unwrap_or_default();
        match status.split_whitespace().nth(1) {
            Some("200") => Ok(()),
            _ => Err(ProbeError::NotReady(address, status.to_owned())),
        }
    }
}
