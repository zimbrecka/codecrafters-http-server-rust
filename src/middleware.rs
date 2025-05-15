use crate::request::Request;
use crate::Response;

pub(crate) fn handle_middlewares(request: &Request, response: Response) -> Response {
    let response = mw_version(request, response);
    let response = mw_close_connection(request, response);
    mw_compress(request, response)
}

fn mw_close_connection(request: &Request, response: Response) -> Response {
    Response {
        connection: request
            .headers
            .get("connection")
            .map(std::string::ToString::to_string),
        ..response
    }
}

fn mw_version(request: &Request, response: Response) -> Response {
    Response {
        version: request.version.clone(),
        ..response
    }
}

fn mw_compress(request: &Request, response: Response) -> Response {
    response.compress(
        request
            .headers
            .get("accept-encoding")
            .map(std::string::String::as_str),
    )
}
