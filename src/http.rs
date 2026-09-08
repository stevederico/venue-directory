use std::collections::HashMap;
use std::io::{Read, Write};

const MAX_HEADERS: usize = 64 * 1024;
const MAX_BODY: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Request {
    #[allow(dead_code)]
    pub fn new(method: &str, path: &str) -> Self {
        Self {
            method: method.to_string(),
            path: path.to_string(),
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn header(&self, key: &str) -> &str {
        self.headers
            .get(&key.to_ascii_lowercase())
            .map(String::as_str)
            .unwrap_or("")
    }

    #[allow(dead_code)]
    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers
            .insert(key.to_ascii_lowercase(), value.to_string());
        self
    }

    #[allow(dead_code)]
    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }
}

#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.push((key.to_string(), value.to_string()));
        self
    }

    pub fn body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.body = body.into();
        self
    }
}

pub fn json_response(status: u16, body: &str) -> Response {
    Response::new(status)
        .header("content-type", "application/json; charset=utf-8")
        .header("access-control-allow-origin", "*")
        .body(body.as_bytes().to_vec())
}

pub fn text_response(status: u16, ctype: &str, body: impl Into<Vec<u8>>) -> Response {
    Response::new(status)
        .header("content-type", ctype)
        .header("access-control-allow-origin", "*")
        .body(body)
}

pub fn html_response(status: u16, body: String) -> Response {
    text_response(status, "text/html; charset=utf-8", body)
}

pub fn read_request(stream: &mut impl Read) -> Result<Request, String> {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 2048];
    let header_end;
    loop {
        let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("empty request".into());
        }
        buf.extend_from_slice(&tmp[..n]);
        if let Some(pos) = find_double_crlf(&buf) {
            header_end = pos;
            break;
        }
        if buf.len() > MAX_HEADERS {
            return Err("headers too large".into());
        }
    }
    let (mut req, already) = parse_head(&buf, header_end)?;
    let want = content_length(&req);
    if want > MAX_BODY {
        return Err("body too large".into());
    }
    req.body = already;
    while req.body.len() < want {
        let n = stream.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        req.body.extend_from_slice(&tmp[..n]);
        if req.body.len() > MAX_BODY {
            return Err("body too large".into());
        }
    }
    req.body.truncate(want);
    Ok(req)
}

pub fn write_response(stream: &mut impl Write, res: &Response) -> Result<(), String> {
    let reason = reason(res.status);
    let mut head = format!("HTTP/1.1 {status} {reason}\r\n", status = res.status);
    let mut has_len = false;
    for (k, v) in &res.headers {
        if k.eq_ignore_ascii_case("content-length") {
            has_len = true;
        }
        head.push_str(k);
        head.push_str(": ");
        head.push_str(v);
        head.push_str("\r\n");
    }
    if !has_len {
        head.push_str(&format!("content-length: {}\r\n", res.body.len()));
    }
    head.push_str("connection: close\r\n\r\n");
    stream
        .write_all(head.as_bytes())
        .map_err(|e| e.to_string())?;
    stream.write_all(&res.body).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn path_only(path: &str) -> &str {
    path.split('?').next().unwrap_or(path)
}

pub fn query_param(path: &str, key: &str) -> Option<String> {
    let q = path.split_once('?')?.1;
    for pair in q.split('&') {
        let (k, v) = match pair.split_once('=') {
            Some(kv) => kv,
            None => (pair, ""),
        };
        if percent_decode(k) == key {
            return Some(percent_decode(v));
        }
    }
    None
}

pub fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (from_hex(bytes[i + 1]), from_hex(bytes[i + 2])) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(b' ');
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn wants_json(req: &Request) -> bool {
    let path = path_only(&req.path);
    path.ends_with(".json") || req.header("accept").contains("application/json")
}

pub fn wants_markdown(req: &Request) -> bool {
    let path = path_only(&req.path);
    path.ends_with(".md") || req.header("accept").contains("text/markdown")
}

fn find_double_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n").map(|i| i + 4)
}

fn parse_head(buf: &[u8], header_end: usize) -> Result<(Request, Vec<u8>), String> {
    let head = std::str::from_utf8(&buf[..header_end]).map_err(|_| "headers not utf-8")?;
    let mut lines = head.split("\r\n");
    let start = lines.next().ok_or("empty request")?;
    let mut parts = start.splitn(3, ' ');
    let method = parts.next().ok_or("bad request line")?.to_string();
    let path = parts.next().ok_or("bad request line")?.to_string();
    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let Some((k, v)) = line.split_once(':') else {
            continue;
        };
        headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_string());
    }
    Ok((
        Request {
            method,
            path,
            headers,
            body: Vec::new(),
        },
        buf[header_end..].to_vec(),
    ))
}

fn content_length(req: &Request) -> usize {
    req.header("content-length").parse().unwrap_or(0)
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        409 => "Conflict",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_and_decode() {
        assert_eq!(
            query_param("/api/search?q=cloud+flare", "q").as_deref(),
            Some("cloud flare")
        );
        assert_eq!(percent_decode("SHACK15"), "SHACK15");
        assert_eq!(percent_decode("9Zero%20Climate%20Hub"), "9Zero Climate Hub");
    }
}
