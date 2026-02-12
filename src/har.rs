use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Har {
    pub log: HarLog,
}

#[derive(Debug, Deserialize)]
pub struct HarLog {
    pub entries: Vec<HarEntry>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HarEntry {
    pub time: f64,
    pub request: HarRequest,
    pub response: HarResponse,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HarRequest {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HarResponse {
    #[serde(default)]
    pub body_size: Option<i64>,
    #[serde(default)]
    pub headers_size: Option<i64>,
    #[serde(default)]
    pub content: Option<HarResponseContent>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HarResponseContent {
    #[serde(default)]
    pub size: Option<i64>,
}

pub fn parse_har(bytes: &[u8]) -> Result<Har> {
    serde_json::from_slice(bytes).with_context(|| "failed to parse HAR JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_minimal_har() {
        let json = r#"{
          "log": {
            "entries": [
              {
                "time": 12.5,
                "request": { "url": "https://example.com" },
                "response": {}
              }
            ]
          }
        }"#;

        let har = parse_har(json.as_bytes()).expect("HAR should parse");
        assert_eq!(har.log.entries.len(), 1);
        assert_eq!(har.log.entries[0].request.url, "https://example.com");
        assert_eq!(har.log.entries[0].response.body_size, None);
        assert_eq!(har.log.entries[0].response.headers_size, None);
    }

    #[test]
    fn rejects_malformed_json() {
        let bad = b"{ this is not valid json }";
        let err = parse_har(bad).expect_err("malformed JSON should fail");
        assert!(err.to_string().contains("failed to parse HAR JSON"));
    }
}
