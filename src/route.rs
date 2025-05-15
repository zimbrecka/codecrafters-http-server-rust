use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use crate::request::Request;
use crate::{HttpCode, Response};

pub(crate) fn handle_request(request: &Request, dest_dir: &str) -> Response {
    // the router...
    let version = request.version.clone();
    match request.path.as_str() {
        "/" => Response::default(),
        "/user-agent" => handle_user_agent(request),
        path if path.starts_with("/echo/") => handle_echo(request, &path[6..]),
        path if request.method == *"GET" && path.starts_with("/files/") => {
            handle_file_content(request, &path[7..], dest_dir)
        }
        path if request.method == *"POST" && path.starts_with("/files/") => {
            handle_file_upload(request, &path[7..], dest_dir)
        }
        _ => Response {
            version,
            status: HttpCode::NotFound,
            ..Default::default()
        },
    }
}

fn handle_echo(_r: &Request, repeat: &str) -> Response {
    Response {
        content: repeat.to_string().into_bytes(),
        ..Default::default()
    }
}

fn handle_user_agent(request: &Request) -> Response {
    if let Some(ua_spec) = request.headers.get("user-agent") {
        Response {
            content: ua_spec.to_string().into_bytes(),
            ..Default::default()
        }
    } else {
        Response {
            status: HttpCode::BadRequest,
            content: String::from("Missing User-Agent header").into_bytes(),
            ..Default::default()
        }
    }
}

fn handle_file_content(_r: &Request, filename: &str, dest_dir: &str) -> Response {
    let mut path = PathBuf::new();
    path.push(dest_dir);
    path.push(filename);

    if let Ok(content) = std::fs::read_to_string(path) {
        Response {
            content_type: String::from("application/octet-stream"),
            content: content.into_bytes(),
            ..Default::default()
        }
    } else {
        Response {
            status: HttpCode::NotFound,
            content: String::from("File not found").into_bytes(),
            ..Default::default()
        }
    }
}

fn handle_file_upload(request: &Request, filename: &str, dest_dir: &str) -> Response {
    let mut path = PathBuf::new();
    path.push(dest_dir);
    path.push(filename);

    match File::create(path) {
        Ok(mut file) => match file.write_all(&request.body) {
            Ok(()) => Response {
                status: HttpCode::Created,
                ..Default::default()
            },
            Err(_) => Response {
                status: HttpCode::InternalServerError,
                content: String::from("Failed to write file").into_bytes(),
                ..Default::default()
            },
        },
        Err(_) => Response {
            status: HttpCode::InternalServerError,
            content: String::from("Failed to create file").into_bytes(),
            ..Default::default()
        },
    }
}
