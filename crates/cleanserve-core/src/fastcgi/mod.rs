//! FastCGI Protocol Implementation
//!
//! This module implements the FastCGI protocol for communication with PHP-FPM/php-cgi.
//! FastCGI is a binary protocol that allows for persistent connections and better
//! performance compared to traditional CGI.

use byteorder::{BigEndian, WriteBytesExt};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::TcpStream;
use tracing::{debug, error};

/// FastCGI record types
const FCGI_BEGIN_REQUEST: u8 = 1;
const FCGI_ABORT_REQUEST: u8 = 2;
const FCGI_END_REQUEST: u8 = 3;
const FCGI_PARAMS: u8 = 4;
const FCGI_STDIN: u8 = 5;
const FCGI_STDOUT: u8 = 6;
const FCGI_STDERR: u8 = 7;
const FCGI_DATA: u8 = 8;
const FCGI_GET_VALUES: u8 = 9;
const FCGI_GET_VALUES_RESULT: u8 = 10;
const FCGI_UNKNOWN_TYPE: u8 = 11;

/// FastCGI role constants
const FCGI_RESPONDER: u16 = 1;
const FCGI_AUTHORIZER: u16 = 2;
const FCGI_FILTER: u16 = 3;

/// FastCGI flags
const FCGI_KEEP_CONN: u8 = 1;

/// Protocol version
const FCGI_VERSION_1: u8 = 1;

/// FastCGI header size
const FCGI_HEADER_SIZE: usize = 8;

/// FastCGI protocol record
#[derive(Debug)]
struct FastCgiRecord {
    version: u8,
    record_type: u8,
    request_id: u16,
    content_length: u16,
    padding_length: u8,
    reserved: u8,
}

impl FastCgiRecord {
    fn new(record_type: u8, request_id: u16, content: &[u8]) -> Self {
        Self {
            version: FCGI_VERSION_1,
            record_type,
            request_id,
            content_length: content.len() as u16,
            padding_length: 0,
            reserved: 0,
        }
    }

    fn encode(&self, content: &[u8], buffer: &mut Vec<u8>) {
        buffer.write_u8(self.version).unwrap();
        buffer.write_u8(self.record_type).unwrap();
        buffer.write_u16::<BigEndian>(self.request_id).unwrap();
        buffer.write_u16::<BigEndian>(self.content_length).unwrap();
        buffer.write_u8(self.padding_length).unwrap();
        buffer.write_u8(self.reserved).unwrap();
        buffer.extend_from_slice(content);
    }
}

/// FastCGI Client for communicating with PHP-FPM
pub struct FastCgiClient {
    stream: TcpStream,
    request_id: u16,
}

impl FastCgiClient {
    /// Connect to a FastCGI server (PHP-FPM socket)
    pub fn connect(addr: &str) -> std::io::Result<Self> {
        let stream = TcpStream::connect(addr)?;
        stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(std::time::Duration::from_secs(30)))?;

        Ok(Self {
            stream,
            request_id: 1,
        })
    }

    /// Make a FastCGI request
    pub fn request(
        &mut self,
        script: &str,
        method: &str,
        uri: &str,
        query_string: &str,
        headers: &HashMap<String, String>,
        body: &[u8],
    ) -> std::io::Result<FastCgiResponse> {
        // Build environment variables
        let mut params = HashMap::new();

        // Required FastCGI params
        params.insert("REQUEST_METHOD".to_string(), method.to_string());
        params.insert("SCRIPT_FILENAME".to_string(), script.to_string());
        params.insert("SCRIPT_NAME".to_string(), script.to_string());
        params.insert("REQUEST_URI".to_string(), uri.to_string());
        params.insert("QUERY_STRING".to_string(), query_string.to_string());
        params.insert("SERVER_PROTOCOL".to_string(), "HTTP/1.1".to_string());
        params.insert("GATEWAY_INTERFACE".to_string(), "CGI/1.1".to_string());
        params.insert("SERVER_SOFTWARE".to_string(), "CleanServe/0.1".to_string());

        // Map HTTP headers to FastCGI params
        for (key, value) in headers {
            let fcgi_key = format!("HTTP_{}", key.to_uppercase().replace('-', "_"));
            params.insert(fcgi_key, value.clone());
        }

        // Send BeginRequest
        self.send_begin_request()?;

        // Send Params
        self.send_params(&params)?;

        // Send Stdin (request body)
        self.send_stdin(body)?;

        // Receive response
        self.receive_response()
    }

    fn send_begin_request(&mut self) -> std::io::Result<()> {
        // FCGI_BEGIN_REQUEST body: role (2 bytes) + flags (1 byte) + reserved (5 bytes)
        let mut body = Vec::with_capacity(8);
        body.write_u16::<BigEndian>(FCGI_RESPONDER).unwrap();
        body.write_u8(FCGI_KEEP_CONN).unwrap();
        body.extend_from_slice(&[0u8; 5]); // reserved

        let record = FastCgiRecord::new(FCGI_BEGIN_REQUEST, self.request_id, &body);
        let mut packet = Vec::new();
        record.encode(&body, &mut packet);

        self.stream.write_all(&packet)?;
        self.stream.flush()?;

        debug!("Sent FastCGI BEGIN_REQUEST");
        Ok(())
    }

    fn send_params(&mut self, params: &HashMap<String, String>) -> std::io::Result<()> {
        // Send empty params to indicate end
        if params.is_empty() {
            let record = FastCgiRecord::new(FCGI_PARAMS, self.request_id, &[]);
            let mut packet = Vec::new();
            record.encode(&[], &mut packet);
            self.stream.write_all(&packet)?;
            return Ok(());
        }

        // Encode and send params
        let encoded = self.encode_params(params);
        let mut offset = 0;

        while offset < encoded.len() {
            let chunk_size = std::cmp::min(65535, encoded.len() - offset);
            let chunk = &encoded[offset..offset + chunk_size];

            let record = FastCgiRecord::new(FCGI_PARAMS, self.request_id, chunk);
            let mut packet = Vec::new();
            record.encode(chunk, &mut packet);

            self.stream.write_all(&packet)?;
            offset += chunk_size;
        }

        // Send empty params to indicate end
        let record = FastCgiRecord::new(FCGI_PARAMS, self.request_id, &[]);
        let mut packet = Vec::new();
        record.encode(&[], &mut packet);
        self.stream.write_all(&packet)?;
        self.stream.flush()?;

        debug!("Sent {} FastCGI params", params.len());
        Ok(())
    }

    fn encode_params(&self, params: &HashMap<String, String>) -> Vec<u8> {
        let mut encoded = Vec::new();

        for (name, value) in params {
            // Name-value encoding:
            // - If name length < 128: 1 byte
            // - If name length >= 128: 4 bytes (31-bit length)
            let name_len = name.len() as u32;
            let value_len = value.len() as u32;

            // Encode name length
            if name_len < 128 {
                encoded.push(name_len as u8);
            } else {
                encoded.push(((name_len >> 24) | 0x80) as u8);
                encoded.push((name_len >> 16) as u8);
                encoded.push((name_len >> 8) as u8);
                encoded.push(name_len as u8);
            }

            // Encode value length
            if value_len < 128 {
                encoded.push(value_len as u8);
            } else {
                encoded.push(((value_len >> 24) | 0x80) as u8);
                encoded.push((value_len >> 16) as u8);
                encoded.push((value_len >> 8) as u8);
                encoded.push(value_len as u8);
            }

            encoded.extend_from_slice(name.as_bytes());
            encoded.extend_from_slice(value.as_bytes());
        }

        encoded
    }

    fn send_stdin(&mut self, body: &[u8]) -> std::io::Result<()> {
        let mut offset = 0;

        while offset < body.len() {
            let chunk_size = std::cmp::min(65535, body.len() - offset);
            let chunk = &body[offset..offset + chunk_size];

            let record = FastCgiRecord::new(FCGI_STDIN, self.request_id, chunk);
            let mut packet = Vec::new();
            record.encode(chunk, &mut packet);

            self.stream.write_all(&packet)?;
            offset += chunk_size;
        }

        // Send empty stdin to indicate end
        let record = FastCgiRecord::new(FCGI_STDIN, self.request_id, &[]);
        let mut packet = Vec::new();
        record.encode(&[], &mut packet);
        self.stream.write_all(&packet)?;
        self.stream.flush()?;

        debug!("Sent {} bytes via FastCGI stdin", body.len());
        Ok(())
    }

    fn receive_response(&mut self) -> std::io::Result<FastCgiResponse> {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut response_headers = HashMap::new();
        let mut response_status = 200;
        let mut body_started = false;

        loop {
            let mut header = [0u8; FCGI_HEADER_SIZE];
            let bytes_read = self.stream.read(&mut header)?;

            if bytes_read == 0 {
                break;
            }

            if bytes_read < FCGI_HEADER_SIZE {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Incomplete FastCGI header",
                ));
            }

            let version = header[0];
            let record_type = header[1];
            let request_id = ((header[2] as u16) << 8) | (header[3] as u16);
            let content_length = ((header[4] as u16) << 8) | (header[5] as u16);
            let padding_length = header[6];

            // Read content
            let mut content = vec![0u8; content_length as usize];
            self.stream.read_exact(&mut content)?;

            // Skip padding
            if padding_length > 0 {
                let mut padding = vec![0u8; padding_length as usize];
                self.stream.read_exact(&mut padding)?;
            }

            match record_type {
                FCGI_STDOUT => {
                    // Parse headers until we hit empty line
                    if !body_started {
                        let content_str = String::from_utf8_lossy(&content);
                        for line in content_str.lines() {
                            if line.is_empty() {
                                body_started = true;
                            } else if let Some(pos) = line.find(':') {
                                let key = line[..pos].trim().to_string();
                                let value = line[pos + 1..].trim().to_string();

                                if key.to_lowercase() == "status" {
                                    if let Ok(code) =
                                        value.split_whitespace().next().unwrap_or("200").parse()
                                    {
                                        response_status = code;
                                    }
                                } else {
                                    response_headers.insert(key, value);
                                }
                            }
                        }
                    }

                    // Add to body after headers
                    if body_started {
                        stdout.extend_from_slice(&content);
                    }
                }
                FCGI_STDERR => {
                    stderr.extend_from_slice(&content);
                    error!("FastCGI stderr: {}", String::from_utf8_lossy(&content));
                }
                FCGI_END_REQUEST => {
                    break;
                }
                _ => {
                    debug!("Unknown FastCGI record type: {}", record_type);
                }
            }
        }

        Ok(FastCgiResponse {
            status: response_status,
            headers: response_headers,
            body: stdout,
            stderr: String::from_utf8_lossy(&stderr).to_string(),
        })
    }

    /// Increment request ID (for multiplexed connections)
    fn next_request_id(&mut self) {
        self.request_id = self.request_id.wrapping_add(1);
    }
}

/// FastCGI response
#[derive(Debug)]
pub struct FastCgiResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
    pub stderr: String,
}

impl FastCgiResponse {
    /// Get content type from headers
    pub fn content_type(&self) -> Option<&str> {
        self.headers.get("Content-Type").map(|s| s.as_str())
    }

    /// Check if response is HTML
    pub fn is_html(&self) -> bool {
        self.content_type()
            .map(|ct| ct.contains("text/html") || ct.contains("application/xhtml+xml"))
            .unwrap_or(false)
    }
}

/// Connection pool for FastCGI
pub struct FastCgiPool {
    addr: String,
    max_connections: usize,
}

impl FastCgiPool {
    pub fn new(addr: &str, max_connections: usize) -> std::io::Result<Self> {
        Ok(Self {
            addr: addr.to_string(),
            max_connections,
        })
    }

    pub fn get_client(&self) -> std::io::Result<FastCgiClient> {
        // Simple implementation - create a new connection each time
        // For production, consider using a proper connection pool library
        FastCgiClient::connect(&self.addr)
    }
}
