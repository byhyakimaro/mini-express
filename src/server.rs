use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

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

pub struct MiniExpress {
    routes: HashMap<String, Handler>,
}

type Handler = fn(Request, Response);

impl MiniExpress {
    pub fn new() -> Self {
        MiniExpress {
            routes: HashMap::new(),
        }
    }

    pub fn get(&mut self, path: &str, handler: Handler) {
        self.routes.insert(format!("GET {}", path), handler);
    }

    pub fn post(&mut self, path: &str, handler: Handler) {
        self.routes.insert(format!("POST {}", path), handler);
    }

    pub fn listen(&self, addr: &str) {
        let listener = TcpListener::bind(addr).unwrap();
        println!("Listening on {}", addr);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let routes = self.routes.clone();
                    std::thread::spawn(move || {
                        handle_connection(stream, routes);
                    });
                }
                Err(e) => eprintln!("Connection failed: {}", e),
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream, routes: HashMap<String, Handler>) {
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

            if let Some(handler) = routes.get(&format!("{} {}", method, path)) {
                handler(req, res);
            } else {
                let mut res = Response::new(stream);
                res.status(404).send("Not Found");
            }
        }
    }
}
