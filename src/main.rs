mod middleware;
mod request;
mod route;

use anyhow::Result;
#[allow(unused_imports)]
use std::net::TcpListener;
use std::net::TcpStream;
use std::{
    env::{self},
    io::Write,
    thread,
};

use flate2::write::{DeflateEncoder, GzEncoder};
use flate2::Compression;

use request::Request;

#[derive(Debug)]
enum HttpCode {
    Ok,
    NotFound,
    InternalServerError,
    BadRequest,
    Created,
}

#[derive(Debug)]
struct Response {
    status: HttpCode,
    version: String,
    content_type: String,
    content_encoding: Option<String>,
    connection: Option<String>,
    content: Vec<u8>,
}

impl Default for Response {
    fn default() -> Self {
        Response {
            status: HttpCode::Ok,
            version: "HTTP/1.1".into(),
            content_type: "text/plain".to_string(),
            content_encoding: None,
            connection: None,
            content: Vec::new(),
        }
    }
}

impl Response {
    fn compress(self, compression: Option<&str>) -> Self {
        match compression {
            Some(compression) => {
                let algorithm = match compression {
                    // order matters
                    c if c.contains("gzip") => Some("gzip"),
                    c if c.contains("deflate") => Some("deflate"),
                    _ => None,
                };

                let compressed_content = match algorithm {
                    Some(c) if c.contains("gzip") => compress_gzip(&self.content),
                    Some(c) if c.contains("deflate") => compress_deflate(&self.content),
                    _ => Ok(self.content.clone()),
                };

                match compressed_content {
                    Ok(compressed_content) => Response {
                        content_encoding: algorithm.map(std::string::ToString::to_string),
                        content: compressed_content,
                        ..self
                    },
                    Err(e) => Response {
                        status: HttpCode::InternalServerError,
                        content: format!("Error compressing content: {e}").into(),
                        ..self
                    },
                }
            }
            None => self,
        }
    }
}

fn main() {
    println!("Logs from your program will appear here! => http://127.0.0.1:4221");

    let destination_directory = extract_destination_directory_from_args();
    println!("dest dir: {destination_directory}");

    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        let dest_dir = destination_directory.clone();
        match stream {
            Ok(mut stream) => {
                println!("accepted new connection");
                thread::spawn(move || match handle_connection(&mut stream, &dest_dir) {
                    Ok(()) => {}
                    Err(e) => {
                        println!("connection error: {e}");
                    }
                });
            }
            Err(e) => {
                println!("error: {e}");
            }
        }
    }
}

fn handle_connection(stream: &mut TcpStream, dest_dir: &str) -> Result<()> {
    loop {
        let Some(request) = request::parse_request(stream)? else {
            continue;
        };
        println!("parsed request: {request:?}");

        let bytes = handle_http_message(&request, dest_dir);
        stream.write_all(&bytes)?;

        if !request.is_persistent() {
            println!("closing connection");
            break;
        }
    }

    Ok(())
}

fn handle_http_message(request: &Request, dest_dir: &str) -> Vec<u8> {
    let response = route::handle_request(request, dest_dir);
    let response = middleware::handle_middlewares(request, response);
    handle_response(response)
}

fn handle_response(response: Response) -> Vec<u8> {
    let content = response.content;
    let head = match response.status {
        HttpCode::Ok => "200 OK",
        HttpCode::BadRequest => "400 Bad Request",
        HttpCode::NotFound => "404 Not Found",
        HttpCode::InternalServerError => "500 Internal Server Error",
        HttpCode::Created => "201 Created",
    };

    let content_type = response.content_type;

    let mut raw_response: Vec<Vec<u8>> = Vec::new();
    raw_response.push(format!("{} {head}\r\n", response.version).into());

    if let Some(connection) = response.connection {
        raw_response.push(format!("Connection: {connection}\r\n").into());
    }

    match content.len() {
        0 => {
            raw_response.push("\r\n".into());
        }
        n => {
            raw_response.push(format!("Content-Type: {content_type}\r\n").into());
            raw_response.push(format!("Content-Length: {n}\r\n").into());
            if let Some(compression) = response.content_encoding {
                raw_response.push(format!("Content-Encoding: {compression}\r\n").into());
            }
            raw_response.push("\r\n".into());
            raw_response.push(content);
        }
    }
    raw_response.concat()
}

fn extract_destination_directory_from_args() -> String {
    let argv = env::args().collect::<Vec<String>>();
    let mut search_dir = argv
        .into_iter()
        .skip_while(|arg| arg.as_str() != "--directory");

    // --directory
    search_dir.next();
    search_dir.next().unwrap_or("/tmp/".into())
}

fn compress_gzip(content: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(content)?;
    encoder.finish().map_err(anyhow::Error::from)
}

fn compress_deflate(content: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(content)?;
    encoder.finish().map_err(anyhow::Error::from)
}
