use anyhow::{Context, Result};
use std::collections::VecDeque;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context as TaskContext, Poll};
use std::time::{Duration, Instant};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpStream, UnixStream};
use tokio::sync::Mutex;
use tracing::debug;

pub enum FastCgiStream {
    Tcp(TcpStream),
    Unix(UnixStream),
}

impl AsyncRead for FastCgiStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match &mut *self {
            FastCgiStream::Tcp(stream) => Pin::new(stream).poll_read(cx, buf),
            FastCgiStream::Unix(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for FastCgiStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut TaskContext<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match &mut *self {
            FastCgiStream::Tcp(stream) => Pin::new(stream).poll_write(cx, buf),
            FastCgiStream::Unix(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        match &mut *self {
            FastCgiStream::Tcp(stream) => Pin::new(stream).poll_flush(cx),
            FastCgiStream::Unix(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<std::io::Result<()>> {
        match &mut *self {
            FastCgiStream::Tcp(stream) => Pin::new(stream).poll_shutdown(cx),
            FastCgiStream::Unix(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}

pub struct PooledConnection {
    stream: FastCgiStream,
    created_at: Instant,
    last_used: Instant,
}

impl std::fmt::Debug for PooledConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PooledConnection")
            .field("created_at", &self.created_at)
            .field("last_used", &self.last_used)
            .field("age", &self.age())
            .field("idle_time", &self.idle_time())
            .finish()
    }
}

impl PooledConnection {
    fn new(stream: FastCgiStream) -> Self {
        let now = Instant::now();
        Self {
            stream,
            created_at: now,
            last_used: now,
        }
    }

    pub fn stream(&mut self) -> &mut FastCgiStream {
        self.last_used = Instant::now();
        &mut self.stream
    }

    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }

    pub fn idle_time(&self) -> Duration {
        self.last_used.elapsed()
    }
}

#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub max_size: usize,
    pub max_idle_time: Duration,
    pub max_lifetime: Duration,
    pub connect_timeout: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_size: 20,
            max_idle_time: Duration::from_secs(60),
            max_lifetime: Duration::from_secs(3600),
            connect_timeout: Duration::from_secs(5),
        }
    }
}

#[derive(Debug)]
pub struct ConnectionPool {
    address: String,
    config: PoolConfig,
    #[allow(dead_code)]
    pool: Arc<Mutex<VecDeque<PooledConnection>>>,
}

impl ConnectionPool {
    pub fn new(address: String, config: PoolConfig) -> Self {
        Self {
            address,
            config,
            pool: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub async fn get(&self) -> Result<PooledConnection> {
        let mut pool = self.pool.lock().await;

        self.cleanup_stale(&mut pool);

        if let Some(conn) = pool.pop_front() {
            debug!("Reusing pooled connection (pool size: {})", pool.len());
            drop(pool); // Release lock
            return Ok(conn);
        }

        drop(pool); // Release lock before creating new connection

        debug!("Creating new FastCGI connection to {}", self.address);
        let stream = if self.address.starts_with("unix:") {
            // Unix socket connection
            let socket_path = self.address.strip_prefix("unix:").unwrap();
            let unix_stream = tokio::time::timeout(
                self.config.connect_timeout,
                UnixStream::connect(socket_path)
            )
            .await
            .context("Connection timeout")?
            .with_context(|| format!("Failed to connect to Unix socket at {}", socket_path))?;
            FastCgiStream::Unix(unix_stream)
        } else {
            // TCP connection
            let tcp_stream = tokio::time::timeout(
                self.config.connect_timeout,
                TcpStream::connect(&self.address)
            )
            .await
            .context("Connection timeout")?
            .with_context(|| format!("Failed to connect to FastCGI at {}", self.address))?;
            FastCgiStream::Tcp(tcp_stream)
        };

        Ok(PooledConnection::new(stream))
    }

    pub async fn put(&self, conn: PooledConnection) {
        let mut pool = self.pool.lock().await;

        if pool.len() >= self.config.max_size {
            debug!("Connection pool full, discarding connection");
            return;
        }

        if conn.age() > self.config.max_lifetime {
            debug!("Connection too old, discarding");
            return;
        }

        pool.push_back(conn);
        debug!("Returned connection to pool (pool size: {})", pool.len());
    }

    fn cleanup_stale(&self, pool: &mut VecDeque<PooledConnection>) {
        pool.retain(|conn| {
            let keep = conn.idle_time() < self.config.max_idle_time
                && conn.age() < self.config.max_lifetime;

            if !keep {
                debug!("Removing stale connection (age: {:?}, idle: {:?})",
                    conn.age(), conn.idle_time());
            }

            keep
        });
    }

    pub async fn stats(&self) -> PoolStats {
        let pool = self.pool.lock().await;
        PoolStats {
            size: pool.len(),
            max_size: self.config.max_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PoolStats {
    pub size: usize,
    pub max_size: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_default() {
        let config = PoolConfig::default();
        assert_eq!(config.max_size, 20);
        assert_eq!(config.max_idle_time, Duration::from_secs(60));
    }

    #[test]
    fn test_pooled_connection_age() {
        use tokio::net::TcpListener;

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let addr = listener.local_addr().unwrap();

            tokio::spawn(async move {
                let _ = listener.accept().await;
            });

            let stream = TcpStream::connect(addr).await.unwrap();
            let conn = PooledConnection::new(stream);

            assert!(conn.age() < Duration::from_secs(1));
            assert!(conn.idle_time() < Duration::from_secs(1));
        });
    }
}
