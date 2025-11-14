use std::fmt;
use std::net::{IpAddr, SocketAddr};

/// Represents a peer address for either TCP or Unix socket connections
#[derive(Debug, Clone)]
pub enum PeerAddr {
    Tcp(SocketAddr),
    Unix(String),
}

impl PeerAddr {
    pub fn from_tcp(addr: SocketAddr) -> Self {
        PeerAddr::Tcp(addr)
    }

    pub fn from_unix(path: impl Into<String>) -> Self {
        PeerAddr::Unix(path.into())
    }

    pub fn ip(&self) -> Option<IpAddr> {
        match self {
            PeerAddr::Tcp(addr) => Some(addr.ip()),
            PeerAddr::Unix(_) => None,
        }
    }

    pub fn socket_addr(&self) -> Option<SocketAddr> {
        match self {
            PeerAddr::Tcp(addr) => Some(*addr),
            PeerAddr::Unix(_) => None,
        }
    }
}

impl fmt::Display for PeerAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PeerAddr::Tcp(addr) => write!(f, "{}", addr),
            PeerAddr::Unix(path) => write!(f, "unix:{}", path),
        }
    }
}
