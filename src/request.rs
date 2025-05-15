use anyhow::Result;

use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    io::{BufRead, BufReader, Read},
    net::TcpStream,
};

use thiserror::Error;

#[derive(Debug)]
pub(crate) struct Request {
    pub method: String,
    pub path: String,
    pub version: String,
    pub headers: HeadersHere,
    pub body: Vec<u8>,
    pub persistent: bool,
}

#[derive(Debug, Error)]
pub(crate) enum RequestError {
    IoErr(std::io::Error),
    ConnectionClosed,
    MissingMethod,
    MissingPath,
    MissingVersion,
    InvalidHeader,
}

impl Display for RequestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::ConnectionClosed => write!(f, "Connection closed by the client"),
            RequestError::MissingMethod => write!(f, "Missing method"),
            RequestError::MissingPath => write!(f, "Missing path"),
            RequestError::MissingVersion => write!(f, "Missing version"),
            RequestError::IoErr(e) => write!(f, "io error: {e}"),
            RequestError::InvalidHeader => write!(f, "Invalid header"),
        }
    }
}

impl Request {
    pub(crate) fn new(
        method: &str,
        path: &str,
        version: &str,
        headers: HeadersHere,
        body: Vec<u8>,
        persistent: bool,
    ) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
            version: version.to_string(),
            headers,
            body,
            persistent,
        }
    }

    pub(crate) fn is_persistent(&self) -> bool {
        self.persistent
    }
}

pub(crate) type HeadersHere = HashMap<String, String>;

pub(crate) fn parse_request(stream: &mut TcpStream) -> Result<Option<Request>> {
    let mut buf_reader = BufReader::new(stream);

    // get request specification: start line
    let mut start_line_part = String::new();
    let _size = buf_reader
        .read_line(&mut start_line_part)
        .map_err(RequestError::IoErr)?;

    // empty buffer => connection closed
    if start_line_part.is_empty() {
        return Err(RequestError::ConnectionClosed.into());
    }

    start_line_part = start_line_part.trim().into();
    // buffer = \n => no message
    if start_line_part.is_empty() {
        return Ok(None);
    }

    let mut start_line = start_line_part.split(' ');
    let method = start_line.next().ok_or(RequestError::MissingMethod)?;
    let path = start_line.next().ok_or(RequestError::MissingPath)?;
    let version = start_line.next().ok_or(RequestError::MissingVersion)?;

    // get headers
    let mut headers: HeadersHere = HashMap::new();
    let mut header_part = String::new();
    while buf_reader
        .read_line(&mut header_part)
        .map_err(RequestError::IoErr)?
        > 0
    {
        header_part = header_part.trim().into();
        if header_part.is_empty() {
            break;
        }

        let mut header_parts = header_part.split(": ");
        let key = header_parts
            .next()
            .ok_or(RequestError::InvalidHeader)?
            .to_string()
            .to_lowercase();
        let value = header_parts
            .next()
            .ok_or(RequestError::InvalidHeader)?
            .to_string();
        headers.insert(key, value);
        header_part.clear();
    }

    // get connection close state
    let connexion_close = matches!(headers.get("connection"), Some(v) if v == "close");

    // get body with the right length
    let content_length = headers
        .get("content-length")
        .unwrap_or(&String::from("0"))
        .parse::<usize>()
        .unwrap_or(0);

    let body = if content_length == 0 {
        Vec::new()
    } else {
        let mut body = vec![0; content_length];
        buf_reader
            .read_exact(&mut body)
            .map_err(RequestError::IoErr)?;
        body
    };

    let persistent = version.contains("1.1") && !connexion_close;
    Ok(Some(Request::new(
        method, path, version, headers, body, persistent,
    )))
}
