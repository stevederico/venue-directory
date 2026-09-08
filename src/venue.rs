use std::path::Path;

use crate::json::Value;
use crate::sqlite;

#[derive(Debug, Clone)]
pub struct Contact {
    pub raw: String,
    pub emails: Vec<String>,
    pub phones: Vec<String>,
    pub x: Vec<String>,
    pub urls: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Venue {
    pub slug: String,
    pub place: String,
    pub address: String,
    pub status: String,
    pub screen: String,
    pub capacity: String,
    pub notes: String,
    pub section: String,
    pub maps: String,
    pub bucket: String,
    pub lat: Option<f64>,
    pub lng: Option<f64>,
    pub contact: Contact,
}

pub fn load_sqlite(path: impl AsRef<Path>) -> Result<Vec<Venue>, String> {
    let conn = sqlite::Connection::open(path)?;
    let rows = conn.query(
        "SELECT slug, place, address, status, screen, capacity, contact, notes, section, maps, lat, lng
         FROM venues ORDER BY rowid",
        |row| {
            let status = row.text(3);
            let contact_raw = row.text(6);
            Venue {
                slug: row.text(0),
                place: row.text(1),
                address: row.text(2),
                bucket: bucket(&status),
                status,
                screen: row.text(4),
                capacity: row.text(5),
                contact: parse_contact(&contact_raw),
                notes: row.text(7),
                section: row.text(8),
                maps: row.text(9),
                lat: row.f64_opt(10),
                lng: row.f64_opt(11),
            }
        },
    )?;
    Ok(rows)
}

pub fn write_status(path: impl AsRef<Path>, slug: &str, status: &str) -> Result<(), String> {
    if !is_slug(slug) {
        return Err("bad slug".into());
    }
    if !matches!(status, "Ask" | "Out" | "Low") {
        return Err("status must be Ask, Out, or Low".into());
    }
    let conn = sqlite::Connection::open(path)?;
    conn.execute(&format!(
        "UPDATE venues SET status = {} WHERE slug = {}",
        sqlite::sql_quote(status),
        sqlite::sql_quote(slug)
    ))
}

pub fn is_slug(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

pub fn apply_status(v: &mut Venue, status: &str) {
    v.status = status.to_string();
    v.bucket = bucket(&v.status);
}

pub fn slugify(place: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in place.chars() {
        let c = c.to_ascii_lowercase();
        if c.is_ascii_alphanumeric() {
            out.push(c);
            dash = false;
        } else if !out.is_empty() && !dash {
            out.push('-');
            dash = true;
        }
    }
    let s = out.trim_end_matches('-').to_string();
    if s.is_empty() {
        "venue".into()
    } else {
        s
    }
}

#[derive(Default)]
pub struct Patch {
    pub place: Option<String>,
    pub address: Option<String>,
    pub status: Option<String>,
    pub screen: Option<String>,
    pub capacity: Option<String>,
    pub contact: Option<String>,
    pub notes: Option<String>,
    pub section: Option<String>,
    pub maps: Option<String>,
    pub lat: Option<Option<f64>>,
    pub lng: Option<Option<f64>>,
}

impl Patch {
    pub fn from_value(v: &Value) -> Result<Self, String> {
        let obj = v.as_object().ok_or("json object required")?;
        Ok(Self {
            place: take_str(obj, "place")?,
            address: take_str(obj, "address")?,
            status: take_str(obj, "status")?,
            screen: take_str(obj, "screen")?,
            capacity: take_str(obj, "capacity")?,
            contact: take_contact(obj)?,
            notes: take_str(obj, "notes")?,
            section: take_str(obj, "section")?,
            maps: take_str(obj, "maps")?,
            lat: take_coord(obj, "lat")?,
            lng: take_coord(obj, "lng")?,
        })
    }
}

fn take_str(
    obj: &std::collections::BTreeMap<String, Value>,
    key: &str,
) -> Result<Option<String>, String> {
    match obj.get(key) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(String::new())),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!("{key} must be a string")),
    }
}

fn take_contact(
    obj: &std::collections::BTreeMap<String, Value>,
) -> Result<Option<String>, String> {
    match obj.get("contact") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(String::new())),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(Value::Object(m)) => match m.get("raw") {
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(Value::Null) | None => Ok(Some(String::new())),
            Some(_) => Err("contact.raw must be a string".into()),
        },
        Some(_) => Err("contact must be a string".into()),
    }
}

fn take_coord(
    obj: &std::collections::BTreeMap<String, Value>,
    key: &str,
) -> Result<Option<Option<f64>>, String> {
    match obj.get(key) {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::Float(n)) => Ok(Some(Some(*n))),
        Some(Value::Int(n)) => Ok(Some(Some(*n as f64))),
        Some(_) => Err(format!("{key} must be a number")),
    }
}

fn sql_coord(v: Option<f64>) -> String {
    match v {
        Some(n) if n.is_finite() => n.to_string(),
        _ => "NULL".into(),
    }
}

pub fn insert(path: impl AsRef<Path>, v: &Venue) -> Result<(), String> {
    if !is_slug(&v.slug) {
        return Err("bad slug".into());
    }
    let conn = sqlite::Connection::open(path)?;
    conn.execute(&format!(
        "INSERT INTO venues (slug, place, address, status, screen, capacity, contact, notes, section, maps, lat, lng)
         VALUES ({}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {}, {})",
        sqlite::sql_quote(&v.slug),
        sqlite::sql_quote(&v.place),
        sqlite::sql_quote(&v.address),
        sqlite::sql_quote(&v.status),
        sqlite::sql_quote(&v.screen),
        sqlite::sql_quote(&v.capacity),
        sqlite::sql_quote(&v.contact.raw),
        sqlite::sql_quote(&v.notes),
        sqlite::sql_quote(&v.section),
        sqlite::sql_quote(&v.maps),
        sql_coord(v.lat),
        sql_coord(v.lng),
    ))
}

pub fn update(path: impl AsRef<Path>, slug: &str, p: &Patch) -> Result<(), String> {
    if !is_slug(slug) {
        return Err("bad slug".into());
    }
    let mut sets = Vec::new();
    let mut add = |col: &str, val: &str| sets.push(format!("{col} = {}", sqlite::sql_quote(val)));
    if let Some(s) = &p.place {
        add("place", s);
    }
    if let Some(s) = &p.address {
        add("address", s);
    }
    if let Some(s) = &p.status {
        add("status", s);
    }
    if let Some(s) = &p.screen {
        add("screen", s);
    }
    if let Some(s) = &p.capacity {
        add("capacity", s);
    }
    if let Some(s) = &p.contact {
        add("contact", s);
    }
    if let Some(s) = &p.notes {
        add("notes", s);
    }
    if let Some(s) = &p.section {
        add("section", s);
    }
    if let Some(s) = &p.maps {
        add("maps", s);
    }
    if let Some(lat) = p.lat {
        sets.push(format!("lat = {}", sql_coord(lat)));
    }
    if let Some(lng) = p.lng {
        sets.push(format!("lng = {}", sql_coord(lng)));
    }
    if sets.is_empty() {
        return Err("empty patch".into());
    }
    let conn = sqlite::Connection::open(path)?;
    conn.execute(&format!(
        "UPDATE venues SET {} WHERE slug = {}",
        sets.join(", "),
        sqlite::sql_quote(slug)
    ))
}

pub fn delete(path: impl AsRef<Path>, slug: &str) -> Result<(), String> {
    if !is_slug(slug) {
        return Err("bad slug".into());
    }
    let conn = sqlite::Connection::open(path)?;
    conn.execute(&format!(
        "DELETE FROM venues WHERE slug = {}",
        sqlite::sql_quote(slug)
    ))
}

pub fn apply_patch(v: &mut Venue, p: &Patch) {
    if let Some(s) = &p.place {
        v.place = s.clone();
    }
    if let Some(s) = &p.address {
        v.address = s.clone();
    }
    if let Some(s) = &p.status {
        apply_status(v, s);
    }
    if let Some(s) = &p.screen {
        v.screen = s.clone();
    }
    if let Some(s) = &p.capacity {
        v.capacity = s.clone();
    }
    if let Some(s) = &p.contact {
        v.contact = parse_contact(s);
    }
    if let Some(s) = &p.notes {
        v.notes = s.clone();
    }
    if let Some(s) = &p.section {
        v.section = s.clone();
    }
    if let Some(s) = &p.maps {
        v.maps = s.clone();
    }
    if let Some(lat) = p.lat {
        v.lat = lat;
    }
    if let Some(lng) = p.lng {
        v.lng = lng;
    }
}

pub fn new_venue(slug: String, p: &Patch) -> Result<Venue, String> {
    let place = p.place.clone().unwrap_or_default();
    if place.trim().is_empty() {
        return Err("place required".into());
    }
    let status = p.status.clone().unwrap_or_else(|| "Ask".into());
    let contact_raw = p.contact.clone().unwrap_or_default();
    Ok(Venue {
        slug,
        address: p.address.clone().unwrap_or_default(),
        screen: p.screen.clone().unwrap_or_default(),
        capacity: p.capacity.clone().unwrap_or_default(),
        notes: p.notes.clone().unwrap_or_default(),
        section: p.section.clone().unwrap_or_default(),
        maps: p.maps.clone().unwrap_or_default(),
        bucket: bucket(&status),
        status,
        contact: parse_contact(&contact_raw),
        lat: p.lat.flatten(),
        lng: p.lng.flatten(),
        place,
    })
}

pub fn find<'a>(venues: &'a [Venue], slug: &str) -> Option<&'a Venue> {
    venues.iter().find(|v| v.slug == slug)
}

pub fn search<'a>(venues: &'a [Venue], q: &str) -> Vec<&'a Venue> {
    let q = q.trim().to_ascii_lowercase();
    if q.is_empty() {
        return venues.iter().collect();
    }
    venues.iter().filter(|v| v.matches(&q)).collect()
}

impl Venue {
    pub fn matches(&self, q: &str) -> bool {
        self.place.to_ascii_lowercase().contains(q)
            || self.address.to_ascii_lowercase().contains(q)
            || self.status.to_ascii_lowercase().contains(q)
            || self.section.to_ascii_lowercase().contains(q)
            || self.notes.to_ascii_lowercase().contains(q)
            || self.contact.raw.to_ascii_lowercase().contains(q)
            || self.slug.contains(q)
            || self.bucket.to_ascii_lowercase().contains(q)
    }

    pub fn to_json(&self) -> Value {
        Value::object(&[
            ("slug", Value::str(&self.slug)),
            ("place", Value::str(&self.place)),
            ("address", Value::str(&self.address)),
            ("status", Value::str(&self.status)),
            ("bucket", Value::str(&self.bucket)),
            ("screen", Value::str(&self.screen)),
            ("capacity", Value::str(&self.capacity)),
            ("notes", Value::str(&self.notes)),
            ("section", Value::str(&self.section)),
            ("maps", Value::str(&self.maps)),
            ("lat", opt_float(self.lat)),
            ("lng", opt_float(self.lng)),
            ("contact", self.contact.to_json()),
        ])
    }

    pub fn to_list_json(&self) -> Value {
        Value::object(&[
            ("slug", Value::str(&self.slug)),
            ("place", Value::str(&self.place)),
            ("address", Value::str(&self.address)),
            ("status", Value::str(&self.status)),
            ("bucket", Value::str(&self.bucket)),
            ("lat", opt_float(self.lat)),
            ("lng", opt_float(self.lng)),
            ("has_contact", Value::Bool(self.contact.has_any())),
        ])
    }

    pub fn to_markdown(&self) -> String {
        let mut s = format!("# {}\n\n", self.place);
        s.push_str(&format!("- slug: `{}`\n", self.slug));
        line(&mut s, "address", &self.address);
        line(&mut s, "status", &self.status);
        line(&mut s, "bucket", &self.bucket);
        line(&mut s, "screen", &self.screen);
        line(&mut s, "capacity", &self.capacity);
        line(&mut s, "section", &self.section);
        line(&mut s, "maps", &self.maps);
        if let (Some(lat), Some(lng)) = (self.lat, self.lng) {
            s.push_str(&format!("- lat,lng: {lat:.6},{lng:.6}\n"));
        }
        if !self.notes.is_empty() {
            s.push_str("\n## Notes\n\n");
            s.push_str(&self.notes);
            s.push_str("\n");
        }
        s.push_str("\n## Contact\n\n");
        if self.contact.raw.is_empty() {
            s.push_str("None on file.\n");
        } else {
            s.push_str(&self.contact.raw);
            s.push_str("\n");
        }
        bullets(&mut s, "Emails", &self.contact.emails);
        bullets(&mut s, "Phones", &self.contact.phones);
        bullets(&mut s, "X", &self.contact.x);
        bullets(&mut s, "URLs", &self.contact.urls);
        s
    }
}

impl Contact {
    pub fn has_any(&self) -> bool {
        !self.raw.is_empty()
    }

    pub fn to_json(&self) -> Value {
        Value::object(&[
            ("raw", Value::str(&self.raw)),
            ("emails", Value::strings(&self.emails)),
            ("phones", Value::strings(&self.phones)),
            ("x", Value::strings(&self.x)),
            ("urls", Value::strings(&self.urls)),
        ])
    }
}

fn bucket(status: &str) -> String {
    let s = status.to_ascii_lowercase();
    let label = if s.contains("out") {
        "Out"
    } else if s.contains("unscored")
        || s.contains("far")
        || s.contains("mentioned")
        || s.contains("needs review")
    {
        "Needs Review"
    } else if s.contains("backup") || s.contains("low") {
        "Low"
    } else if s.contains("ask") {
        "Ask"
    } else if s.contains("contacted")
        || s.contains("waiting")
        || s.contains("emailed")
        || s.contains("dmed")
        || s.contains("submitted")
    {
        "Contacted"
    } else if s.is_empty() {
        "Other"
    } else {
        "Other"
    };
    label.into()
}

fn opt_float(v: Option<f64>) -> Value {
    match v {
        Some(n) => Value::Float(n),
        None => Value::Null,
    }
}

fn line(out: &mut String, key: &str, val: &str) {
    if !val.is_empty() {
        out.push_str(&format!("- {key}: {val}\n"));
    }
}

fn bullets(out: &mut String, title: &str, items: &[String]) {
    if items.is_empty() {
        return;
    }
    out.push_str(&format!("\n## {title}\n\n"));
    for item in items {
        out.push_str(&format!("- {item}\n"));
    }
}

pub fn parse_contact(raw: &str) -> Contact {
    let emails = extract_emails(raw);
    let phones = extract_phones(raw);
    let mut x = extract_x(raw);
    for handle in extract_mentions(raw) {
        if !x.iter().any(|u| u.eq_ignore_ascii_case(&handle)) {
            x.push(handle);
        }
    }
    let mut urls = extract_urls(raw);
    urls.retain(|u| {
        !x.iter().any(|h| {
            u.eq_ignore_ascii_case(h) || u.to_ascii_lowercase().starts_with(&h.to_ascii_lowercase())
        })
    });
    Contact {
        raw: raw.trim().to_string(),
        emails,
        phones,
        x,
        urls,
    }
}

fn extract_emails(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            let start = scan_left(s, i);
            let end = scan_right(s, i + 1);
            if start < i && end > i + 1 {
                let email: String = s[start..end].trim_matches('.').to_ascii_lowercase();
                if email.contains('.') && !out.iter().any(|e| e == &email) {
                    out.push(email);
                }
            }
        }
        i += 1;
    }
    out
}

fn is_email_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-')
}

fn scan_left(s: &str, at: usize) -> usize {
    let mut start = at;
    for (idx, c) in s[..at].char_indices().rev() {
        if is_email_char(c) {
            start = idx;
        } else {
            break;
        }
    }
    start
}

fn scan_right(s: &str, from: usize) -> usize {
    let mut end = from;
    for (idx, c) in s[from..].char_indices() {
        if is_email_char(c) {
            end = from + idx + c.len_utf8();
        } else {
            break;
        }
    }
    end
}

fn extract_phones(s: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '+' || chars[i].is_ascii_digit() || chars[i] == '(' {
            if let Some((end, phone)) = take_phone(&chars, i) {
                if !out.iter().any(|p| digits_only(p) == digits_only(&phone)) {
                    out.push(phone);
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn take_phone(chars: &[char], start: usize) -> Option<(usize, String)> {
    let mut i = start;
    let mut buf = String::new();
    let mut digits = 0u32;
    if chars[i] == '+' {
        buf.push('+');
        i += 1;
    }
    if i < chars.len() && chars[i] == '(' {
        buf.push('(');
        i += 1;
    }
    while i < chars.len() {
        let c = chars[i];
        if c.is_ascii_digit() {
            buf.push(c);
            digits += 1;
            i += 1;
        } else if matches!(c, ' ' | '-' | '.' | '(' | ')') && digits > 0 && digits < 15 {
            buf.push(c);
            i += 1;
        } else if c == 'x' || c == 'X' {
            // extension: x124
            if i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                buf.push('x');
                i += 1;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    buf.push(chars[i]);
                    i += 1;
                }
            }
            break;
        } else {
            break;
        }
    }
    if digits >= 10 && digits <= 15 {
        let phone = buf
            .trim()
            .trim_end_matches(['-', '.', ' ', '('])
            .to_string();
        Some((i, phone))
    } else {
        None
    }
}

fn digits_only(s: &str) -> String {
    s.chars().filter(|c| c.is_ascii_digit()).collect()
}

fn extract_urls(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let lower = s.to_ascii_lowercase();
    let mut from = 0;
    while let Some(rel) = find_url_start(&lower[from..]) {
        let start = from + rel;
        let rest = &s[start..];
        let end_rel = rest
            .find(|c: char| c.is_ascii_whitespace() || matches!(c, ',' | ';' | ')' | '·'))
            .unwrap_or(rest.len());
        let mut url = rest[..end_rel].trim_end_matches('.').to_string();
        url = url.trim_end_matches('/').to_string();
        if !out.iter().any(|u| u == &url) {
            out.push(url);
        }
        from = start + end_rel;
    }
    out
}

fn find_url_start(s: &str) -> Option<usize> {
    let http = s.find("http://");
    let https = s.find("https://");
    match (http, https) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn extract_mentions(s: &str) -> Vec<String> {
    let chars: Vec<char> = s.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '@' && i + 1 < chars.len() && is_handle_char(chars[i + 1]) {
            if i > 0 && is_email_char(chars[i - 1]) {
                i += 1;
                continue;
            }
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && is_handle_char(chars[j]) {
                j += 1;
            }
            if j < chars.len() && chars[j] == '.' {
                if j + 1 < chars.len() && chars[j + 1].is_ascii_alphabetic() {
                    i = j;
                    continue;
                }
            }
            let handle: String = chars[start..j]
                .iter()
                .collect::<String>()
                .to_ascii_lowercase();
            if handle.len() >= 2 && !mention_is_not_x(&chars, i, j) {
                let url = format!("https://x.com/{handle}");
                if !out.iter().any(|u| u == &url) {
                    out.push(url);
                }
            }
            i = j;
            continue;
        }
        i += 1;
    }
    out
}

fn is_handle_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn mention_is_not_x(chars: &[char], at: usize, end: usize) -> bool {
    let mut start = 0;
    let mut i = at;
    while i > 0 {
        i -= 1;
        if chars[i] == '(' {
            start = i;
            break;
        }
        if chars[i] == '.' || chars[i] == ';' {
            start = i + 1;
            break;
        }
    }
    let mut stop = chars.len();
    i = end;
    while i < chars.len() {
        if matches!(chars[i], ')' | '.' | ';') {
            stop = i;
            break;
        }
        i += 1;
    }
    let w: String = chars[start..stop].iter().collect::<String>().to_ascii_lowercase();
    w.contains("not x") || w.contains("is luma") || w.contains("luma @")
}

fn extract_x(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    for url in extract_urls(s) {
        let lower = url.to_ascii_lowercase();
        let handle = if let Some(rest) = lower.strip_prefix("https://x.com/") {
            Some(rest)
        } else if let Some(rest) = lower.strip_prefix("http://x.com/") {
            Some(rest)
        } else if let Some(rest) = lower.strip_prefix("https://twitter.com/") {
            Some(rest)
        } else if let Some(rest) = lower.strip_prefix("http://twitter.com/") {
            Some(rest)
        } else {
            None
        };
        let Some(handle) = handle else { continue };
        let handle = handle.split('/').next().unwrap_or(handle);
        let handle = handle.trim_start_matches('@');
        if handle.is_empty() {
            continue;
        }
        let canon = format!("https://x.com/{handle}");
        if !out.iter().any(|u| u == &canon) {
            out.push(canon);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_contact() {
        let c = parse_contact(
            "cloudflareusergroup@cloudflare.com (Fri). DMs: https://x.com/lucataco · (415) 555-0100",
        );
        assert_eq!(c.emails, ["cloudflareusergroup@cloudflare.com"]);
        assert!(c.x.contains(&"https://x.com/lucataco".into()));
        assert_eq!(c.phones, ["(415) 555-0100"]);
    }

    #[test]
    fn mention_before_sentence_period() {
        let c = parse_contact("DMs: @lucataco @craigsdennis. Kristian Freeman");
        assert!(c.x.contains(&"https://x.com/lucataco".into()));
        assert!(c.x.contains(&"https://x.com/craigsdennis".into()));
    }

    #[test]
    fn slug_from_punctuation() {
        assert_eq!(slugify("SHACK15"), "shack15");
        assert_eq!(slugify("Founders, Inc."), "founders-inc");
    }

    #[test]
    fn luma_handle_is_not_x() {
        let c = parse_contact(
            "awsbuilderloft@amazon.com. No official X (Luma @awsbuilderloft is Luma, not X).",
        );
        assert_eq!(c.emails, ["awsbuilderloft@amazon.com"]);
        assert!(!c.x.iter().any(|u| u.contains("awsbuilderloft")));
    }

    #[test]
    fn unscored_maps_to_needs_review() {
        assert_eq!(bucket("Unscored"), "Needs Review");
        assert_eq!(bucket("Needs Review"), "Needs Review");
        assert_eq!(bucket("Mentioned"), "Needs Review");
    }
}
