use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

impl Value {
    pub fn object(pairs: &[(&str, Value)]) -> Self {
        Value::Object(
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), v.clone()))
                .collect(),
        )
    }

    pub fn str(s: impl Into<String>) -> Self {
        Value::String(s.into())
    }

    pub fn strings(items: &[String]) -> Self {
        Value::Array(items.iter().cloned().map(Value::String).collect())
    }
}

pub fn parse(input: &str) -> Result<Value, String> {
    let mut p = Parser {
        s: input.as_bytes(),
        i: 0,
    };
    let v = p.value()?;
    p.skip_ws();
    if p.i != p.s.len() {
        return Err("trailing junk".into());
    }
    Ok(v)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn skip_ws(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn bump(&mut self) -> Result<u8, String> {
        let c = self.peek().ok_or("unexpected end")?;
        self.i += 1;
        Ok(c)
    }

    fn eat(&mut self, want: u8) -> Result<(), String> {
        self.skip_ws();
        let c = self.bump()?;
        if c == want {
            Ok(())
        } else {
            Err("unexpected char".into())
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        self.skip_ws();
        match self.peek().ok_or("unexpected end")? {
            b'n' => self.lit(b"null", Value::Null),
            b't' => self.lit(b"true", Value::Bool(true)),
            b'f' => self.lit(b"false", Value::Bool(false)),
            b'"' => Ok(Value::String(self.string()?)),
            b'{' => self.object(),
            b'[' => self.array(),
            b'-' | b'0'..=b'9' => self.number(),
            _ => Err("unexpected char".into()),
        }
    }

    fn lit(&mut self, bytes: &[u8], v: Value) -> Result<Value, String> {
        for b in bytes {
            if self.bump()? != *b {
                return Err("bad literal".into());
            }
        }
        Ok(v)
    }

    fn object(&mut self) -> Result<Value, String> {
        self.eat(b'{')?;
        let mut map = BTreeMap::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Value::Object(map));
        }
        loop {
            self.skip_ws();
            let k = self.string()?;
            self.eat(b':')?;
            let v = self.value()?;
            map.insert(k, v);
            self.skip_ws();
            match self.bump()? {
                b',' => continue,
                b'}' => break,
                _ => return Err("bad object".into()),
            }
        }
        Ok(Value::Object(map))
    }

    fn array(&mut self) -> Result<Value, String> {
        self.eat(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.i += 1;
            return Ok(Value::Array(items));
        }
        loop {
            items.push(self.value()?);
            self.skip_ws();
            match self.bump()? {
                b',' => continue,
                b']' => break,
                _ => return Err("bad array".into()),
            }
        }
        Ok(Value::Array(items))
    }

    fn string(&mut self) -> Result<String, String> {
        self.skip_ws();
        if self.bump()? != b'"' {
            return Err("expected string".into());
        }
        let mut out = String::new();
        loop {
            match self.bump()? {
                b'"' => return Ok(out),
                b'\\' => match self.bump()? {
                    b'"' => out.push('"'),
                    b'\\' => out.push('\\'),
                    b'/' => out.push('/'),
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'u' => {
                        let mut n = 0u32;
                        for _ in 0..4 {
                            let h = self.bump()?;
                            n = (n << 4)
                                + match h {
                                    b'0'..=b'9' => u32::from(h - b'0'),
                                    b'a'..=b'f' => u32::from(h - b'a' + 10),
                                    b'A'..=b'F' => u32::from(h - b'A' + 10),
                                    _ => return Err("bad unicode".into()),
                                };
                        }
                        out.push(char::from_u32(n).ok_or("bad unicode")?);
                    }
                    _ => return Err("bad escape".into()),
                },
                c if c < 0x20 => return Err("raw control in string".into()),
                _c => {
                    // UTF-8: put this byte back into a char
                    self.i -= 1;
                    let rest = std::str::from_utf8(&self.s[self.i..]).map_err(|_| "bad utf-8")?;
                    let ch = rest.chars().next().ok_or("bad utf-8")?;
                    out.push(ch);
                    self.i += ch.len_utf8();
                }
            }
        }
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        let mut float = false;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.i += 1;
        }
        if self.peek() == Some(b'.') {
            float = true;
            self.i += 1;
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            float = true;
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        let s = std::str::from_utf8(&self.s[start..self.i]).map_err(|_| "bad number")?;
        if float {
            Ok(Value::Float(s.parse().map_err(|_| "bad number")?))
        } else {
            Ok(Value::Int(s.parse().map_err(|_| "bad number")?))
        }
    }
}

impl Value {
    pub fn as_object(&self) -> Option<&BTreeMap<String, Value>> {
        match self {
            Value::Object(m) => Some(m),
            _ => None,
        }
    }
}

pub fn stringify(value: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, value);
    out
}

fn write_value(out: &mut String, value: &Value) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Int(n) => out.push_str(&n.to_string()),
        Value::Float(n) => write_float(out, *n),
        Value::String(s) => write_string(out, s),
        Value::Array(items) => {
            out.push('[');
            for (i, v) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(out, v);
            }
            out.push(']');
        }
        Value::Object(map) => {
            out.push('{');
            for (i, (k, v)) in map.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(out, k);
                out.push(':');
                write_value(out, v);
            }
            out.push('}');
        }
    }
}

fn write_float(out: &mut String, n: f64) {
    if !n.is_finite() {
        out.push_str("null");
        return;
    }
    let s = format!("{n:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    out.push_str(s);
}

pub fn string_field(body: &[u8], key: &str) -> Option<String> {
    let s = std::str::from_utf8(body).ok()?;
    let needle = format!("\"{key}\"");
    let i = s.find(&needle)?;
    let rest = s[i + needle.len()..].trim_start().strip_prefix(':')?;
    let rest = rest.trim_start().strip_prefix('"')?;
    let mut out = String::new();
    let mut escaped = false;
    for c in rest.chars() {
        if escaped {
            out.push(c);
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '"' {
            return Some(out);
        }
        out.push(c);
    }
    None
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_and_floats() {
        let v = Value::object(&[
            ("place", Value::str("A \"Loft\"")),
            ("lat", Value::Float(37.7749)),
        ]);
        let s = stringify(&v);
        assert!(s.contains(r#"\"Loft\""#));
        assert!(s.contains("37.7749"));
        assert!(!s.contains("37.774900"));
    }

    #[test]
    fn reads_status_field() {
        assert_eq!(
            string_field(br#"{"status":"Out"}"#, "status").as_deref(),
            Some("Out")
        );
    }

    #[test]
    fn parse_object_and_coords() {
        let v = parse(r#"{"place":"Test Loft","lat":37.77,"lng":-122.4,"notes":null}"#).unwrap();
        let o = v.as_object().unwrap();
        assert_eq!(o.get("place"), Some(&Value::str("Test Loft")));
        match o.get("lat") {
            Some(Value::Float(n)) => assert!((n - 37.77).abs() < 1e-9),
            other => panic!("{other:?}"),
        }
        assert_eq!(o.get("notes"), Some(&Value::Null));
    }
}
