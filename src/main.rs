mod http;
mod json;
mod page;
mod sqlite;
mod venue;

use std::env;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

use http::{
    html_response, json_response, path_only, query_param, read_request, text_response, wants_json,
    wants_markdown, write_response, Request, Response,
};
use json::{stringify, Value};
use venue::Venue;

fn db_path() -> std::path::PathBuf {
    env::var("VENUES_DB")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("data/venues.db"))
}

struct App {
    db: PathBuf,
    venues: Mutex<Vec<Venue>>,
}

fn main() {
    let db = db_path();
    let venues = venue::load_sqlite(&db).unwrap_or_else(|e| {
        eprintln!("sqlite: {e}");
        std::process::exit(1);
    });
    let n = venues.len();
    let pins = venues.iter().filter(|v| v.lat.is_some()).count();
    let app = Arc::new(App {
        db,
        venues: Mutex::new(venues),
    });
    let host = env::var("HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let port: u16 = env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(18790);
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .unwrap_or_else(|_| ([127, 0, 0, 1], port).into());
    let listener = TcpListener::bind(addr).unwrap_or_else(|e| {
        eprintln!("listen {addr}: {e}");
        std::process::exit(1);
    });
    println!("venue-directory {n} venues, {pins} pins  http://{addr}/");
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let app = Arc::clone(&app);
        thread::spawn(move || serve(stream, &app));
    }
}

fn serve(mut stream: TcpStream, app: &App) {
    let _ = stream.set_nodelay(true);
    let req = match read_request(&mut stream) {
        Ok(r) => r,
        Err(_) => return,
    };
    let res = handle(app, &req);
    let _ = write_response(&mut stream, &res);
}

fn handle(app: &App, req: &Request) -> Response {
    let path = path_only(&req.path);
    let m = req.method.as_str();
    match (m, path) {
        ("GET" | "HEAD", "/") => home(app, req),
        ("GET" | "HEAD", "/health") => json_response(
            200,
            &stringify(&Value::object(&[("ok", Value::Bool(true))])),
        ),
        ("GET" | "HEAD", "/llms.txt") => {
            text_response(200, "text/plain; charset=utf-8", llms_txt())
        }
        ("GET" | "HEAD", "/robots.txt") => text_response(
            200,
            "text/plain; charset=utf-8",
            "User-agent: *\nAllow: /\nAllow: /api/\nAllow: /llms.txt\n",
        ),
        ("GET" | "HEAD", "/api/venues" | "/api/venues.json") => list_json(app, req),
        ("GET" | "HEAD", "/api/search") => list_json(app, req),
        ("POST", "/api/venues" | "/api/venues.json") => create_venue(app, req),
        ("GET" | "HEAD", p) if p.starts_with("/api/venues/") => {
            venue_api(app, req, &p["/api/venues/".len()..])
        }
        ("POST", p) if p.starts_with("/api/venues/") && p.ends_with("/status") => {
            let slug = p
                .trim_start_matches("/api/venues/")
                .trim_end_matches("/status");
            post_status(app, slug, req)
        }
        ("PATCH" | "PUT", p) if p.starts_with("/api/venues/") => {
            patch_venue(app, &p["/api/venues/".len()..], req)
        }
        ("DELETE", p) if p.starts_with("/api/venues/") => {
            delete_venue(app, &p["/api/venues/".len()..])
        }
        ("GET" | "HEAD", p) if p.starts_with("/v/") => venue_page(app, req, &p["/v/".len()..]),
        ("GET" | "HEAD", _) => not_found(req),
        _ => json_err(405, "method not allowed"),
    }
}

fn lock_venues(app: &App) -> std::sync::MutexGuard<'_, Vec<Venue>> {
    app.venues.lock().unwrap_or_else(|e| e.into_inner())
}

fn home(app: &App, req: &Request) -> Response {
    if wants_json(req) {
        return list_json(app, req);
    }
    let venues = lock_venues(app);
    if wants_markdown(req) {
        return text_response(200, "text/markdown; charset=utf-8", index_md(&venues));
    }
    html_response(200, page::html_home(&venues))
}

fn list_json(app: &App, req: &Request) -> Response {
    let q = query_param(&req.path, "q").unwrap_or_default();
    let bucket = query_param(&req.path, "bucket").unwrap_or_default();
    let want: Vec<&str> = bucket
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty() && *s != "all")
        .collect();
    let venues = lock_venues(app);
    let items: Vec<Value> = venue::search(&venues, &q)
        .into_iter()
        .filter(|v| want.is_empty() || want.iter().any(|b| v.bucket == *b))
        .map(Venue::to_list_json)
        .collect();
    json_response(
        200,
        &stringify(&Value::object(&[
            ("count", Value::Int(items.len() as i64)),
            ("venues", Value::Array(items)),
        ])),
    )
}

fn venue_api(app: &App, _req: &Request, rest: &str) -> Response {
    let rest = rest.trim_end_matches('/');
    let (slug, contact_only) = if let Some(slug) = rest.strip_suffix("/contact") {
        (slug.trim_end_matches(".json"), true)
    } else {
        (
            rest.trim_end_matches(".json").trim_end_matches(".md"),
            false,
        )
    };
    let venues = lock_venues(app);
    let Some(v) = venue::find(&venues, slug) else {
        return json_err(404, "venue not found");
    };
    if rest.ends_with(".md") {
        return text_response(200, "text/markdown; charset=utf-8", v.to_markdown());
    }
    if contact_only {
        return json_response(
            200,
            &stringify(&Value::object(&[
                ("slug", Value::str(&v.slug)),
                ("place", Value::str(&v.place)),
                ("address", Value::str(&v.address)),
                ("maps", Value::str(&v.maps)),
                ("status", Value::str(&v.status)),
                ("contact", v.contact.to_json()),
            ])),
        );
    }
    json_response(200, &stringify(&v.to_json()))
}

fn venue_page(app: &App, req: &Request, rest: &str) -> Response {
    let slug = rest
        .trim_end_matches(".json")
        .trim_end_matches(".md")
        .trim_end_matches('/');
    let venues = lock_venues(app);
    let Some(v) = venue::find(&venues, slug) else {
        return not_found(req);
    };
    if rest.ends_with(".md") || wants_markdown(req) {
        return text_response(200, "text/markdown; charset=utf-8", v.to_markdown());
    }
    if rest.ends_with(".json") || wants_json(req) {
        return json_response(200, &stringify(&v.to_json()));
    }
    html_response(200, page::html_venue(v))
}

fn not_found(req: &Request) -> Response {
    if wants_json(req) {
        json_err(404, "not found")
    } else {
        html_response(404, "<!DOCTYPE html><title>Not Found</title><h1>Not Found</h1><p><a href=/>Venue Directory</a></p>".into())
    }
}

fn body_value(req: &Request) -> Result<Value, String> {
    let s = std::str::from_utf8(&req.body).map_err(|_| "body not utf-8")?;
    if s.trim().is_empty() {
        return Err("json object required".into());
    }
    json::parse(s)
}

fn api_slug(rest: &str) -> Result<&str, Response> {
    let rest = rest
        .trim_end_matches('/')
        .trim_end_matches(".json")
        .trim_end_matches('/');
    if rest.is_empty() || rest.contains('/') || !venue::is_slug(rest) {
        return Err(json_err(400, "bad slug"));
    }
    Ok(rest)
}

fn write_err(e: &str) -> Response {
    if e.contains("UNIQUE") {
        json_err(409, "slug exists")
    } else if e == "empty patch" || e == "place required" || e == "bad slug" || e.starts_with("status")
    {
        json_err(400, e)
    } else {
        json_err(500, e)
    }
}

fn create_venue(app: &App, req: &Request) -> Response {
    let val = match body_value(req) {
        Ok(v) => v,
        Err(e) => return json_err(400, &e),
    };
    let patch = match venue::Patch::from_value(&val) {
        Ok(p) => p,
        Err(e) => return json_err(400, &e),
    };
    let explicit = val.as_object().and_then(|o| match o.get("slug") {
        Some(Value::String(s)) => Some(s.as_str()),
        _ => None,
    });
    let mut venues = lock_venues(app);
    let slug = if let Some(s) = explicit {
        if !venue::is_slug(s) {
            return json_err(400, "bad slug");
        }
        if venue::find(&venues, s).is_some() {
            return json_err(409, "slug exists");
        }
        s.to_string()
    } else {
        let place = patch.place.clone().unwrap_or_default();
        let base = venue::slugify(&place);
        let mut slug = base.clone();
        let mut n = 2;
        while venue::find(&venues, &slug).is_some() {
            slug = format!("{base}-{n}");
            n += 1;
        }
        slug
    };
    let v = match venue::new_venue(slug, &patch) {
        Ok(v) => v,
        Err(e) => return json_err(400, &e),
    };
    if let Err(e) = venue::insert(&app.db, &v) {
        return write_err(&e);
    }
    let json = stringify(&v.to_json());
    venues.push(v);
    json_response(201, &json)
}

fn patch_venue(app: &App, rest: &str, req: &Request) -> Response {
    let slug = match api_slug(rest) {
        Ok(s) => s.to_string(),
        Err(r) => return r,
    };
    let val = match body_value(req) {
        Ok(v) => v,
        Err(e) => return json_err(400, &e),
    };
    let patch = match venue::Patch::from_value(&val) {
        Ok(p) => p,
        Err(e) => return json_err(400, &e),
    };
    let mut venues = lock_venues(app);
    if venue::find(&venues, &slug).is_none() {
        return json_err(404, "venue not found");
    }
    if let Err(e) = venue::update(&app.db, &slug, &patch) {
        return write_err(&e);
    }
    let Some(v) = venues.iter_mut().find(|v| v.slug == slug) else {
        return json_err(404, "venue not found");
    };
    venue::apply_patch(v, &patch);
    json_response(200, &stringify(&v.to_json()))
}

fn delete_venue(app: &App, rest: &str) -> Response {
    let slug = match api_slug(rest) {
        Ok(s) => s.to_string(),
        Err(r) => return r,
    };
    let mut venues = lock_venues(app);
    if venue::find(&venues, &slug).is_none() {
        return json_err(404, "venue not found");
    }
    if let Err(e) = venue::delete(&app.db, &slug) {
        return write_err(&e);
    }
    venues.retain(|v| v.slug != slug);
    json_response(
        200,
        &stringify(&Value::object(&[
            ("ok", Value::Bool(true)),
            ("slug", Value::str(slug)),
        ])),
    )
}

fn post_status(app: &App, slug: &str, req: &Request) -> Response {
    let Some(status) = json::string_field(&req.body, "status") else {
        return json_err(400, "status required");
    };
    {
        let venues = lock_venues(app);
        if venue::find(&venues, slug).is_none() {
            return json_err(404, "venue not found");
        }
    }
    if let Err(e) = venue::write_status(&app.db, slug, &status) {
        let code = if e.starts_with("status") || e == "bad slug" {
            400
        } else {
            500
        };
        return json_err(code, &e);
    }
    let mut venues = lock_venues(app);
    let Some(v) = venues.iter_mut().find(|v| v.slug == slug) else {
        return json_err(404, "venue not found");
    };
    venue::apply_status(v, &status);
    json_response(200, &stringify(&v.to_json()))
}

fn json_err(status: u16, msg: &str) -> Response {
    json_response(
        status,
        &stringify(&Value::object(&[("error", Value::str(msg))])),
    )
}

fn index_md(venues: &[Venue]) -> String {
    let mut s = String::from("# Venue Directory\n\n");
    s.push_str("SF rooms. Use `/api/venues/{slug}/contact` for emails, phones, and X.\n\n");
    for v in venues {
        s.push_str(&format!(
            "- [{}](/v/{}.md) ({})\n",
            v.place, v.slug, v.bucket
        ));
    }
    s
}

fn llms_txt() -> &'static str {
    r#"# Venue Directory

SF rooms. Parsed contact. SQLite is the store.

Base: the origin you fetched this from (local default http://127.0.0.1:18790).

## Read contact (start here)

GET /api/venues/{slug}/contact

Returns place, address, emails[], phones[], x[] (https://x.com/...), urls[], contact.raw.

  curl -sS http://127.0.0.1:18790/api/venues/cloudflare/contact
  curl -sS http://127.0.0.1:18790/api/venues/shack15/contact

Empty emails/phones/x: read contact.raw. Do not invent addresses.

## List

GET /api/venues
GET /api/search?q=soma
GET /api/venues?bucket=Ask,Contacted

JSON: { count, venues: [{ slug, place, address, status, bucket, lat, lng, has_contact }] }

Buckets: Ask, Contacted, Low, Out, Needs Review.
Status is one of those labels (Contacted, not a DM or email log). Optional. Empty in the shipped data.

## One venue

GET /api/venues/{slug}
GET /v/{slug}.json
GET /v/{slug}.md
HTML: GET /v/{slug}
Map + list: GET /

Accept: application/json or text/markdown on / and /v/{slug}.

## Slug

Lowercase kebab of place. Cloudflare -> cloudflare. SHACK15 -> shack15.

## Write (CRUD)

No auth. Writes SQLite and live memory. Restart not required.

POST   /api/venues                 create. place required. status defaults to Ask.
PATCH  /api/venues/{slug}          partial update. PUT is the same. null clears.
DELETE /api/venues/{slug}          delete
POST   /api/venues/{slug}/status   Ask, Out, or Low (review sheet)

  curl -sS -X POST http://127.0.0.1:18790/api/venues \
    -H 'content-type: application/json' \
    -d '{"place":"Test Loft","address":"1 Market St","status":"Ask"}'

  curl -sS -X PATCH http://127.0.0.1:18790/api/venues/test-loft \
    -H 'content-type: application/json' \
    -d '{"status":"Contacted","notes":"Reached out"}'

  curl -sS -X DELETE http://127.0.0.1:18790/api/venues/test-loft

Errors: 400 { "error": "place required" | "empty patch" | "json object required" }
        404 { "error": "venue not found" }
        409 { "error": "slug exists" }

Map tiles: OSM. Some rows have no street and no pin.
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::Request;

    fn app() -> App {
        App {
            db: PathBuf::from("data/venues.db"),
            venues: Mutex::new(venue::load_sqlite("data/venues.db").unwrap()),
        }
    }

    #[test]
    fn named_venues_load() {
        let app = app();
        let venues = lock_venues(&app);
        venue::find(&venues, "commonwealth-club").expect("commonwealth-club");
        venue::find(&venues, "canopy-jackson-square").expect("canopy-jackson-square");
        venue::find(&venues, "aws-builder-loft").expect("aws-builder-loft");
    }

    #[test]
    fn aws_luma_is_not_x() {
        let app = app();
        let venues = lock_venues(&app);
        let a = venue::find(&venues, "aws-builder-loft").expect("aws-builder-loft");
        assert!(!a.contact.x.iter().any(|u| u.contains("awsbuilderloft")));
    }

    #[test]
    fn contact_endpoint_has_cloudflare_email() {
        let app = app();
        let req = Request::new("GET", "/api/venues/cloudflare/contact");
        let res = handle(&app, &req);
        assert_eq!(res.status, 200);
        let body = String::from_utf8(res.body).unwrap();
        assert!(
            body.contains("cloudflareusergroup@cloudflare.com"),
            "{body}"
        );
        assert!(body.contains("\"emails\""));
    }

    #[test]
    fn search_finds_shack() {
        let app = app();
        let req = Request::new("GET", "/api/search?q=shack15");
        let res = handle(&app, &req);
        let body = String::from_utf8(res.body).unwrap();
        assert!(body.contains("shack15"), "{body}");
    }

    #[test]
    fn bucket_filter() {
        let app = scratch();
        handle(
            &app,
            &Request::new("POST", "/api/venues")
                .with_body(br#"{"place":"Shack","slug":"shack15","status":"Contacted"}"#),
        );
        handle(
            &app,
            &Request::new("POST", "/api/venues")
                .with_body(br#"{"place":"AWS Builder Loft","status":"Out"}"#),
        );
        let req = Request::new("GET", "/api/venues?bucket=Contacted");
        let res = handle(&app, &req);
        let body = String::from_utf8(res.body).unwrap();
        assert!(body.contains("shack15"), "{body}");
        assert!(!body.contains("\"slug\":\"aws-builder-loft\""), "{body}");
    }

    #[test]
    fn missing_is_404() {
        let app = app();
        let req = Request::new("GET", "/api/venues/no-such-place")
            .with_header("accept", "application/json");
        let res = handle(&app, &req);
        assert_eq!(res.status, 404);
    }

    #[test]
    fn post_status_sets_ask() {
        let app = scratch();
        let res = handle(
            &app,
            &Request::new("POST", "/api/venues")
                .with_body(br#"{"place":"Ask Loft","status":"Low"}"#),
        );
        assert_eq!(res.status, 201, "{}", String::from_utf8_lossy(&res.body));
        let res = handle(
            &app,
            &Request::new("POST", "/api/venues/ask-loft/status").with_body(br#"{"status":"Ask"}"#),
        );
        assert_eq!(res.status, 200, "{}", String::from_utf8_lossy(&res.body));
        let body = String::from_utf8(res.body).unwrap();
        assert!(body.contains("\"status\":\"Ask\""), "{body}");
        assert!(body.contains("\"bucket\":\"Ask\""), "{body}");
        let res = handle(
            &app,
            &Request::new("POST", "/api/venues/ask-loft/status")
                .with_body(br#"{"status":"Contacted"}"#),
        );
        assert_eq!(res.status, 400);
    }

    const SCHEMA: &str = "CREATE TABLE venues (
  slug TEXT PRIMARY KEY,
  place TEXT NOT NULL,
  address TEXT NOT NULL DEFAULT '',
  status TEXT NOT NULL DEFAULT '',
  screen TEXT NOT NULL DEFAULT '',
  capacity TEXT NOT NULL DEFAULT '',
  contact TEXT NOT NULL DEFAULT '',
  notes TEXT NOT NULL DEFAULT '',
  section TEXT NOT NULL DEFAULT '',
  maps TEXT NOT NULL DEFAULT '',
  lat REAL,
  lng REAL
);";

    fn scratch() -> App {
        let db = std::env::temp_dir().join(format!(
            "venue-crud-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let conn = sqlite::Connection::open(&db).unwrap();
        conn.execute(SCHEMA).unwrap();
        App {
            db,
            venues: Mutex::new(Vec::new()),
        }
    }

    #[test]
    fn crud_create_patch_delete() {
        let app = scratch();
        let res = handle(
            &app,
            &Request::new("POST", "/api/venues").with_body(
                br#"{"place":"Test Loft","address":"1 Market St","status":"Ask","contact":"desk@testloft.example","lat":37.79,"lng":-122.39}"#,
            ),
        );
        assert_eq!(res.status, 201, "{}", String::from_utf8_lossy(&res.body));
        let body = String::from_utf8(res.body).unwrap();
        assert!(body.contains("\"slug\":\"test-loft\""), "{body}");
        assert!(body.contains("desk@testloft.example"), "{body}");

        let res = handle(&app, &Request::new("GET", "/api/venues/test-loft"));
        assert_eq!(res.status, 200);

        let res = handle(
            &app,
            &Request::new("PATCH", "/api/venues/test-loft")
                .with_body(br#"{"status":"Contacted","notes":"Emailed Mon"}"#),
        );
        assert_eq!(res.status, 200, "{}", String::from_utf8_lossy(&res.body));
        let body = String::from_utf8(res.body).unwrap();
        assert!(body.contains("\"bucket\":\"Contacted\""), "{body}");
        assert!(body.contains("Emailed Mon"), "{body}");

        let res = handle(&app, &Request::new("DELETE", "/api/venues/test-loft"));
        assert_eq!(res.status, 200);
        let res = handle(
            &app,
            &Request::new("GET", "/api/venues/test-loft").with_header("accept", "application/json"),
        );
        assert_eq!(res.status, 404);
    }

    #[test]
    fn create_needs_place() {
        let app = scratch();
        let res = handle(
            &app,
            &Request::new("POST", "/api/venues").with_body(br#"{"address":"1 Market"}"#),
        );
        assert_eq!(res.status, 400);
    }

    #[test]
    fn create_conflict() {
        let app = scratch();
        let res = handle(
            &app,
            &Request::new("POST", "/api/venues").with_body(br#"{"place":"Dup","slug":"dup"}"#),
        );
        assert_eq!(res.status, 201, "{}", String::from_utf8_lossy(&res.body));
        let res = handle(
            &app,
            &Request::new("POST", "/api/venues").with_body(br#"{"place":"Dup 2","slug":"dup"}"#),
        );
        assert_eq!(res.status, 409);
    }
}
