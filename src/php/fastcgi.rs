use anyhow::Result;
use bytes::{BufMut, BytesMut};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use super::connection_pool::{ConnectionPool, FastCgiStream, PoolConfig};

const FCGI_VERSION_1: u8 = 1;
const FCGI_BEGIN_REQUEST: u8 = 1;
#[allow(dead_code)]
const FCGI_ABORT_REQUEST: u8 = 2;
const FCGI_END_REQUEST: u8 = 3;
const FCGI_PARAMS: u8 = 4;
const FCGI_STDIN: u8 = 5;
const FCGI_STDOUT: u8 = 6;
const FCGI_STDERR: u8 = 7;

const FCGI_RESPONDER: u16 = 1;
#[allow(dead_code)]
const FCGI_KEEP_CONN: u8 = 1;

#[derive(Debug)]
pub struct FastCgiClient {
    pool: Arc<ConnectionPool>,
}

impl FastCgiClient {
    pub fn new(address: String) -> Self {
        let config = PoolConfig::default();
        Self {
            pool: Arc::new(ConnectionPool::new(address, config)),
        }
    }

    pub fn with_pool_config(address: String, config: PoolConfig) -> Self {
        Self {
            pool: Arc::new(ConnectionPool::new(address, config)),
        }
    }

    pub async fn execute(
        &self,
        script_path: &str,
        method: &str,
        uri: &str,
        query_string: &str,
        headers: &HashMap<String, String>,
        body: &[u8],
        remote_addr: &str,
    ) -> Result<(Vec<u8>, Vec<u8>)> {
        let mut pooled_conn = self.pool.get().await?;
        let stream = pooled_conn.stream();

        let request_id = 1u16;

        let begin_request = self.build_begin_request(request_id);
        stream.write_all(&begin_request).await?;

        let params = self.build_params(script_path, method, uri, query_string, headers, remote_addr);
        let params_records = self.build_params_records(request_id, &params);
        for record in params_records {
            stream.write_all(&record).await?;
        }

        let empty_params = self.build_record(FCGI_PARAMS, request_id, &[]);
        stream.write_all(&empty_params).await?;

        if !body.is_empty() {
            let stdin_records = self.build_data_records(FCGI_STDIN, request_id, body);
            for record in stdin_records {
                stream.write_all(&record).await?;
            }
        }

        let empty_stdin = self.build_record(FCGI_STDIN, request_id, &[]);
        stream.write_all(&empty_stdin).await?;

        let (stdout, stderr) = self.read_response(stream, request_id).await?;

        self.pool.put(pooled_conn).await;

        Ok((stdout, stderr))
    }

    fn build_begin_request(&self, request_id: u16) -> Vec<u8> {
        let mut buf = BytesMut::with_capacity(16);

        buf.put_u8(FCGI_VERSION_1);
        buf.put_u8(FCGI_BEGIN_REQUEST);
        buf.put_u16(request_id);
        buf.put_u16(8); // content length
        buf.put_u8(0);  // padding length
        buf.put_u8(0);  // reserved

        buf.put_u16(FCGI_RESPONDER);
        buf.put_u8(0);  // flags (no keep-alive for simplicity)
        buf.put(&[0u8; 5][..]); // reserved

        buf.to_vec()
    }

    fn build_params(
        &self,
        script_path: &str,
        method: &str,
        uri: &str,
        query_string: &str,
        headers: &HashMap<String, String>,
        remote_addr: &str,
    ) -> HashMap<String, String> {
        let mut params = HashMap::new();

        params.insert("SCRIPT_FILENAME".to_string(), script_path.to_string());
        params.insert("REQUEST_METHOD".to_string(), method.to_string());
        params.insert("REQUEST_URI".to_string(), uri.to_string());
        params.insert("QUERY_STRING".to_string(), query_string.to_string());
        params.insert("DOCUMENT_URI".to_string(), uri.split('?').next().unwrap_or(uri).to_string());
        params.insert("REMOTE_ADDR".to_string(), remote_addr.to_string());
        params.insert("REMOTE_PORT".to_string(), "0".to_string());
        params.insert("SERVER_SOFTWARE".to_string(), "fe-php/0.1.0".to_string());
        params.insert("SERVER_PROTOCOL".to_string(), "HTTP/1.1".to_string());
        params.insert("GATEWAY_INTERFACE".to_string(), "CGI/1.1".to_string());

        for (name, value) in headers {
            let name_upper = name.to_uppercase().replace("-", "_");
            let param_name = if name_upper == "CONTENT_TYPE" || name_upper == "CONTENT_LENGTH" {
                name_upper
            } else {
                format!("HTTP_{}", name_upper)
            };
            params.insert(param_name, value.clone());
        }

        params
    }

    fn build_params_records(&self, request_id: u16, params: &HashMap<String, String>) -> Vec<Vec<u8>> {
        let mut params_data = BytesMut::new();

        for (name, value) in params {
            self.encode_name_value_pair(&mut params_data, name, value);
        }

        self.build_data_records(FCGI_PARAMS, request_id, &params_data)
    }

    fn encode_name_value_pair(&self, buf: &mut BytesMut, name: &str, value: &str) {
        let name_bytes = name.as_bytes();
        let value_bytes = value.as_bytes();

        if name_bytes.len() < 128 {
            buf.put_u8(name_bytes.len() as u8);
        } else {
            buf.put_u32((name_bytes.len() as u32) | 0x80000000);
        }

        if value_bytes.len() < 128 {
            buf.put_u8(value_bytes.len() as u8);
        } else {
            buf.put_u32((value_bytes.len() as u32) | 0x80000000);
        }

        buf.put(name_bytes);
        buf.put(value_bytes);
    }

    fn build_data_records(&self, record_type: u8, request_id: u16, data: &[u8]) -> Vec<Vec<u8>> {
        const MAX_CONTENT_LEN: usize = 65535;
        let mut records = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            let chunk_len = std::cmp::min(MAX_CONTENT_LEN, data.len() - offset);
            let chunk = &data[offset..offset + chunk_len];
            records.push(self.build_record(record_type, request_id, chunk));
            offset += chunk_len;
        }

        if records.is_empty() {
            records.push(self.build_record(record_type, request_id, &[]));
        }

        records
    }

    fn build_record(&self, record_type: u8, request_id: u16, content: &[u8]) -> Vec<u8> {
        let content_len = content.len();
        let padding_len = (8 - (content_len % 8)) % 8;

        let mut buf = BytesMut::with_capacity(8 + content_len + padding_len);

        buf.put_u8(FCGI_VERSION_1);
        buf.put_u8(record_type);
        buf.put_u16(request_id);
        buf.put_u16(content_len as u16);
        buf.put_u8(padding_len as u8);
        buf.put_u8(0); // reserved

        buf.put(content);
        buf.put(&vec![0u8; padding_len][..]);

        buf.to_vec()
    }

    async fn read_response(&self, stream: &mut FastCgiStream, expected_request_id: u16) -> Result<(Vec<u8>, Vec<u8>)> {
        let mut stdout_data = Vec::new();
        let mut stderr_data = Vec::new();

        loop {
            let mut header = [0u8; 8];
            match stream.read_exact(&mut header).await {
                Ok(_) => {},
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(e.into()),
            }

            let version = header[0];
            let record_type = header[1];
            let request_id = u16::from_be_bytes([header[2], header[3]]);
            let content_length = u16::from_be_bytes([header[4], header[5]]) as usize;
            let padding_length = header[6] as usize;

            if version != FCGI_VERSION_1 {
                return Err(anyhow::anyhow!("Unsupported FastCGI version: {}", version));
            }

            if request_id != expected_request_id {
                let total_skip = content_length + padding_length;
                let mut discard = vec![0u8; total_skip];
                stream.read_exact(&mut discard).await?;
                continue;
            }

            let mut content = vec![0u8; content_length];
            if content_length > 0 {
                stream.read_exact(&mut content).await?;
            }

            if padding_length > 0 {
                let mut padding = vec![0u8; padding_length];
                stream.read_exact(&mut padding).await?;
            }

            match record_type {
                FCGI_STDOUT => {
                    if content_length > 0 {
                        stdout_data.extend_from_slice(&content);
                    }
                }
                FCGI_STDERR => {
                    if content_length > 0 {
                        stderr_data.extend_from_slice(&content);
                    }
                }
                FCGI_END_REQUEST => {
                    // Request completed
                    break;
                }
                _ => {
                    // Unknown record type, ignore
                }
            }
        }

        Ok((stdout_data, stderr_data))
    }
}
