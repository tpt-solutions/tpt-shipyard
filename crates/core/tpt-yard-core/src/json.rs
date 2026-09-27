//! Minimal JSON value, parser, and writer.
//!
//! `tpt-yard-core` deliberately has no external dependencies, so project and
//! golden-data interchange uses this small, self-contained JSON
//! implementation. It supports the full JSON grammar (objects, arrays,
//! numbers, strings with `\uXXXX` escapes including surrogate pairs,
//! booleans, null). Numbers are stored as `f64`.
//!
//! ```
//! use tpt_yard_core::json::Value;
//!
//! let v = Value::parse(r#"{"name": "Block 212", "weight_kg": 184.5}"#).unwrap();
//! assert_eq!(v.get("name").and_then(|s| s.as_str()), Some("Block 212"));
//! assert_eq!(v.get("weight_kg").and_then(|n| n.as_f64()), Some(184.5));
//! ```

use std::fmt;

/// A JSON value.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// `null`
    Null,
    /// `true` / `false`
    Bool(bool),
    /// Any JSON number (stored as `f64`).
    Number(f64),
    /// A string.
    String(String),
    /// An array.
    Array(Vec<Value>),
    /// An object; pairs keep insertion order.
    Object(Vec<(String, Value)>),
}

/// JSON parse / access errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError(pub String);

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "json error: {}", self.0)
    }
}

impl std::error::Error for JsonError {}

impl Value {
    // ---------------------------------------------------------------- access

    /// Object member lookup (linear scan; objects here are small).
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Mutable object member lookup.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Value> {
        match self {
            Value::Object(pairs) => pairs.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// Borrow the contained string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Borrow the contained number.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Number as `u64` (errors on negative or fractional values).
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Number(n) if *n >= 0.0 && n.fract() == 0.0 && n.abs() <= u64::MAX as f64 => {
                Some(*n as u64)
            }
            _ => None,
        }
    }

    /// Borrow the contained boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Borrow the contained array.
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    /// Borrow the contained object pairs.
    pub fn as_object(&self) -> Option<&[(String, Value)]> {
        match self {
            Value::Object(o) => Some(o),
            _ => None,
        }
    }

    // ------------------------------------------------------------------ parse

    /// Parses a JSON document.
    ///
    /// # Errors
    ///
    /// [`JsonError`] with position information on malformed input.
    pub fn parse(input: &str) -> Result<Value, JsonError> {
        let mut p = Parser {
            bytes: input.as_bytes(),
            pos: 0,
        };
        p.skip_ws();
        let v = p.value()?;
        p.skip_ws();
        if p.pos != p.bytes.len() {
            return Err(p.err("trailing characters after JSON value"));
        }
        Ok(v)
    }

    /// Serializes compactly (no extra whitespace).
    pub fn to_string_compact(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, None, 0);
        out
    }

    /// Serializes with two-space indentation.
    pub fn to_string_pretty(&self) -> String {
        let mut out = String::new();
        self.write(&mut out, Some(2), 0);
        out
    }

    fn write(&self, out: &mut String, indent: Option<usize>, depth: usize) {
        let pad = |out: &mut String, depth: usize| {
            if let Some(w) = indent {
                out.push('\n');
                for _ in 0..w * depth {
                    out.push(' ');
                }
            }
        };
        match self {
            Value::Null => out.push_str("null"),
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::Number(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    out.push_str(&format!("{}", *n as i64));
                } else if n.is_finite() {
                    out.push_str(&format!("{n}"));
                } else {
                    out.push_str("null"); // JSON has no Inf/NaN
                }
            }
            Value::String(s) => write_escaped(out, s),
            Value::Array(items) => {
                out.push('[');
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    pad(out, depth + 1);
                    item.write(out, indent, depth + 1);
                }
                if !items.is_empty() {
                    pad(out, depth);
                }
                out.push(']');
            }
            Value::Object(pairs) => {
                out.push('{');
                for (i, (k, v)) in pairs.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    pad(out, depth + 1);
                    write_escaped(out, k);
                    out.push(':');
                    if indent.is_some() {
                        out.push(' ');
                    }
                    v.write(out, indent, depth + 1);
                }
                if !pairs.is_empty() {
                    pad(out, depth);
                }
                out.push('}');
            }
        }
    }
}

fn write_escaped(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, msg: &str) -> JsonError {
        JsonError(format!("{} at byte {}", msg, self.pos))
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, b: u8) -> Result<(), JsonError> {
        if self.peek() == Some(b) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.err(&format!("expected '{}'", b as char)))
        }
    }

    fn literal(&mut self, lit: &str, value: Value) -> Result<Value, JsonError> {
        if self.bytes[self.pos..].starts_with(lit.as_bytes()) {
            self.pos += lit.len();
            Ok(value)
        } else {
            Err(self.err("invalid literal"))
        }
    }

    fn value(&mut self) -> Result<Value, JsonError> {
        match self
            .peek()
            .ok_or_else(|| self.err("unexpected end of input"))?
        {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => self.string().map(Value::String),
            b't' => self.literal("true", Value::Bool(true)),
            b'f' => self.literal("false", Value::Bool(false)),
            b'n' => self.literal("null", Value::Null),
            _ => self.number(),
        }
    }

    fn object(&mut self) -> Result<Value, JsonError> {
        self.expect(b'{')?;
        let mut pairs = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Object(pairs));
        }
        loop {
            self.skip_ws();
            let key = self.string()?;
            self.skip_ws();
            self.expect(b':')?;
            self.skip_ws();
            let val = self.value()?;
            pairs.push((key, val));
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Object(pairs));
                }
                _ => return Err(self.err("expected ',' or '}' in object")),
            }
        }
    }

    fn array(&mut self) -> Result<Value, JsonError> {
        self.expect(b'[')?;
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.skip_ws();
            items.push(self.value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => {
                    self.pos += 1;
                }
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(self.err("expected ',' or ']' in array")),
            }
        }
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.expect(b'"')?;
        let mut s = String::new();
        loop {
            let b = self.peek().ok_or_else(|| self.err("unterminated string"))?;
            self.pos += 1;
            match b {
                b'"' => return Ok(s),
                b'\\' => {
                    let esc = self.peek().ok_or_else(|| self.err("unterminated escape"))?;
                    self.pos += 1;
                    match esc {
                        b'"' => s.push('"'),
                        b'\\' => s.push('\\'),
                        b'/' => s.push('/'),
                        b'b' => s.push('\u{08}'),
                        b'f' => s.push('\u{0C}'),
                        b'n' => s.push('\n'),
                        b'r' => s.push('\r'),
                        b't' => s.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let c = if (0xD800..0xDC00).contains(&hi) {
                                // High surrogate: expect a matching low surrogate.
                                if self.bytes[self.pos..].starts_with(b"\\u") {
                                    self.pos += 2;
                                    let lo = self.hex4()?;
                                    if (0xDC00..0xE000).contains(&lo) {
                                        let combined =
                                            0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
                                        char::from_u32(combined)
                                            .ok_or_else(|| self.err("invalid surrogate pair"))?
                                    } else {
                                        return Err(self.err("invalid low surrogate"));
                                    }
                                } else {
                                    return Err(self.err("lone high surrogate"));
                                }
                            } else {
                                char::from_u32(hi).ok_or_else(|| self.err("invalid \\u escape"))?
                            };
                            s.push(c);
                        }
                        _ => return Err(self.err("invalid escape character")),
                    }
                }
                _ => {
                    // Consume a full UTF-8 sequence.
                    let start = self.pos - 1;
                    let len = utf8_len(b);
                    if start + len > self.bytes.len() {
                        return Err(self.err("truncated UTF-8 sequence"));
                    }
                    let chunk = std::str::from_utf8(&self.bytes[start..start + len])
                        .map_err(|_| self.err("invalid UTF-8 in string"))?;
                    s.push_str(chunk);
                    self.pos = start + len;
                }
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, JsonError> {
        if self.pos + 4 > self.bytes.len() {
            return Err(self.err("truncated \\u escape"));
        }
        let hex = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4])
            .map_err(|_| self.err("invalid \\u escape"))?;
        let v = u32::from_str_radix(hex, 16).map_err(|_| self.err("invalid \\u escape"))?;
        self.pos += 4;
        Ok(v)
    }

    fn number(&mut self) -> Result<Value, JsonError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        while matches!(
            self.peek(),
            Some(b'0'..=b'9' | b'.' | b'e' | b'E' | b'+' | b'-')
        ) {
            self.pos += 1;
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos])
            .map_err(|_| self.err("invalid number"))?;
        text.parse::<f64>()
            .map(Value::Number)
            .map_err(|_| self.err(&format!("invalid number '{text}'")))
    }
}

fn utf8_len(first: u8) -> usize {
    match first {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_roundtrip_pretty() {
        let src = r#"{
            "name": "USS Example",
            "teu": 14000,
            "ratio": 0.75,
            "flag": null,
            "ok": true,
            "cog": [120.5, 0.0, -8.25],
            "nested": {"a": [1, 2, 3]}
        }"#;
        let v = Value::parse(src).unwrap();
        assert_eq!(v.get("name").unwrap().as_str().unwrap(), "USS Example");
        assert_eq!(v.get("teu").unwrap().as_u64().unwrap(), 14000);
        assert_eq!(v.get("ratio").unwrap().as_f64().unwrap(), 0.75);
        assert!(matches!(v.get("flag"), Some(Value::Null)));
        assert_eq!(v.get("ok").unwrap().as_bool(), Some(true));
        let cog = v.get("cog").unwrap().as_array().unwrap();
        assert_eq!(cog.len(), 3);
        assert_eq!(cog[2].as_f64(), Some(-8.25));
        assert_eq!(
            v.get("nested")
                .unwrap()
                .get("a")
                .unwrap()
                .as_array()
                .unwrap()[1]
                .as_f64(),
            Some(2.0)
        );

        let reparsed = Value::parse(&v.to_string_pretty()).unwrap();
        assert_eq!(v, reparsed);
        let reparsed2 = Value::parse(&v.to_string_compact()).unwrap();
        assert_eq!(v, reparsed2);
    }

    #[test]
    fn escapes_and_unicode() {
        let src = r#"{"s": "line\nbreak \"quoted\" back\\slash snow ☃"}"#;
        let v = Value::parse(src).unwrap();
        assert_eq!(
            v.get("s").unwrap().as_str().unwrap(),
            "line\nbreak \"quoted\" back\\slash snow ☃"
        );
        // Surrogate pair for U+1F680 (rocket).
        let v2 = Value::parse(r#""\ud83d\ude80""#).unwrap();
        assert_eq!(v2, Value::String("🚀".to_string()));
        // Round trip through the writer.
        let out = v2.to_string_compact();
        assert_eq!(Value::parse(&out).unwrap(), v2);
    }

    #[test]
    fn lone_surrogate_is_rejected() {
        assert!(Value::parse(r#""\ud83d""#).is_err());
        assert!(Value::parse(r#""\ud83dx""#).is_err());
    }

    #[test]
    fn malformed_inputs_error() {
        assert!(Value::parse("{").is_err());
        assert!(Value::parse("[1, 2,]").is_err());
        assert!(Value::parse("{\"a\" 1}").is_err());
        assert!(Value::parse("nul").is_err());
        assert!(Value::parse("1 2").is_err());
        assert!(Value::parse(r#""unterminated"#).is_err());
        assert!(Value::parse("").is_err());
    }

    #[test]
    fn number_formats() {
        assert_eq!(Value::parse("-12e2").unwrap().as_f64(), Some(-1200.0));
        assert_eq!(Value::parse("0.5").unwrap().as_f64(), Some(0.5));
        assert_eq!(Value::parse("1E+3").unwrap().as_f64(), Some(1000.0));
        // Integer-valued floats are written without a decimal point.
        assert_eq!(Value::Number(7.0).to_string_compact(), "7");
        assert_eq!(Value::Number(7.25).to_string_compact(), "7.25");
    }
}
