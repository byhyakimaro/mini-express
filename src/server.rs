use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::Arc;

use serde::Serialize;
use serde_json;

pub struct Request {
    pub method: String,
    pub path: String,
    pub body: String,
}

pub struct Response {
    stream: TcpStream,
    status_code: u16,
    body: String,
    headers: HashMap<String, String>,
}

impl Response {
    pub fn new(stream: TcpStream) -> Self {
        Response {
            stream,
            status_code: 200,
            body: String::new(),
            headers: HashMap::new(),
        }
    }

    pub fn status(&mut self, code: u16) -> &mut Self {
        self.status_code = code;
        self
    }

    pub fn header(&mut self, key: &str, value: &str) -> &mut Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn send(&mut self, body: &str) {
        self.body = body.to_string();

        let status_text = match self.status_code {
            200 => "OK",
            201 => "Created",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            500 => "Internal Server Error",
            _ => "Unknown",
        };

        self.header("Content-Length", &self.body.len().to_string());

        let mut headers_str = String::new();
        for (key, value) in &self.headers {
            headers_str.push_str(&format!("{}: {}\r\n", key, value));
        }

        let response = format!(
            "HTTP/1.1 {} {}\r\n{}\
            \r\n{}",
            self.status_code,
            status_text,
            headers_str,
            self.body
        );

        let _ = self.stream.write_all(response.as_bytes());
    }

    pub fn json<T: Serialize>(&mut self, data: &T) {
        if let Ok(json_str) = serde_json::to_string(data) {
            self.header("Content-Type", "application/json");
            self.send(&json_str);
        } else {
            self.status(500).send("{\"error\": \"Failed to serialize JSON\"}");
        }
    }
}

type Handler = fn(Request, Response, Option<HashMap<String, String>>);

#[derive(Clone)]
pub struct Route {
    method: String,
    path: String,
    handler: Handler,
}

type Next<'a> = Box<dyn FnOnce(Request, Response, Option<HashMap<String, String>>) + Send + 'a>;
type Middleware = Arc<dyn Fn(Request, Response, Option<HashMap<String, String>>, Next) + Send + Sync>;

pub struct MiniExpress {
    routes: Vec<Route>,
    middlewares: Vec<Middleware>,
}

impl MiniExpress {
    pub fn new() -> Self {
        MiniExpress {
            routes: Vec::new(),
            middlewares: Vec::new(),
        }
    }
    
    pub fn use_middleware(&mut self, mw: Middleware) {
        self.middlewares.push(mw);
    }
    
    pub fn get(&mut self, path: &str, handler: Handler) {
        self.routes.push(Route {
            method: "GET".to_string(),
            path: path.to_string(),
            handler,
        });
    }
    
    pub fn post(&mut self, path: &str, handler: Handler) {
        self.routes.push(Route {
            method: "POST".to_string(),
            path: path.to_string(),
            handler,
        });
    }

    pub fn listen(&self, addr: &str) {
        let listener = TcpListener::bind(addr).unwrap();
        println!("Listening on {}", addr);
    
        let routes = self.routes.clone();
        let middlewares = self.middlewares.clone();
    
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let routes = routes.clone();
                    let middlewares = middlewares.clone();
                    std::thread::spawn(move || {
                        handle_connection(stream, routes, middlewares);
                    });
                }
                Err(e) => eprintln!("Connection failed: {}", e),
            }
        }
    }
}

fn match_route<'a>(
    method: &str,
    req_path: &str,
    routes: &'a [Route],
) -> Option<(&'a Handler, Option<HashMap<String, String>>)> {
    for route in routes {
        if route.method != method {
            continue;
        }

        let route_parts: Vec<&str> = route.path.split('/').collect();
        let req_parts: Vec<&str> = req_path.split('/').collect();

        if route_parts.len() != req_parts.len() {
            continue;
        }

        let mut params = HashMap::new();
        let mut matched = true;

        for (rp, rq) in route_parts.iter().zip(req_parts.iter()) {
            if rp.starts_with(':') {
                // parâmetro, extrai o nome sem ':'
                params.insert(rp[1..].to_string(), rq.to_string());
            } else if rp != rq {
                matched = false;
                break;
            }
        }

        if matched {
            if params.is_empty() {
                return Some((&route.handler, None));
            } else {
                return Some((&route.handler, Some(params)));
            }
        }
    }
    None
}

fn run_middlewares<'a>(
    req: Request,
    res: Response,
    params: Option<HashMap<String, String>>,
    middlewares: &'a [Middleware],
    handler: &'a Handler,
) {
    if let Some((first, rest)) = middlewares.split_first() {
        let next = Box::new(move |req, res, params| {
            run_middlewares(req, res, params, rest, handler);
        });
        first(req, res, params, next);
    } else {
        handler(req, res, params);
    }
}

fn handle_connection(mut stream: TcpStream, routes: Vec<Route>, middlewares: Vec<Middleware>) {
    let mut buffer = [0; 512];
    let _ = stream.read(&mut buffer);

    let request_str = String::from_utf8_lossy(&buffer);
    let mut lines = request_str.lines();

    if let Some(first_line) = lines.next() {
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() >= 2 {
            let method = parts[0].to_string();
            let path = parts[1].to_string();
            let body = request_str.split("\r\n\r\n").nth(1).unwrap_or("").to_string();

            let req = Request { method: method.clone(), path: path.clone(), body };
            let res = Response::new(stream.try_clone().unwrap());

            if let Some((handler, params)) = match_route(&method, &path, &routes) {
                run_middlewares(req, res, params, &middlewares, handler);
            } else {
                let mut res = Response::new(stream);
                res.status(404).send("Not Found");
            }
        }
    }
}
