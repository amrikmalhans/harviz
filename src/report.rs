use serde::Serialize;

use crate::har::HarEntry;

#[derive(Debug, Serialize)]
pub struct Report {
    pub entries: usize,
    pub total_time_ms: f64,
    pub total_bytes: u64,
    pub top_requested: usize,
    pub top_returned: usize,
    pub top_slowest: Vec<ReportRow>,
    pub top_largest: Vec<ReportRow>,
}

#[derive(Debug, Serialize)]
pub struct ReportRow {
    pub url: String,
    pub time_ms: f64,
    pub bytes: u64,
}

pub fn pos_i64_to_u64(x: Option<i64>) -> u64 {
    match x {
        Some(v) if v > 0 => v as u64,
        _ => 0,
    }
}

pub fn entry_bytes(e: &HarEntry) -> u64 {
    let r = &e.response;
    let body = pos_i64_to_u64(r.body_size).max(pos_i64_to_u64(r.content.as_ref().and_then(|c| c.size)));
    let headers = pos_i64_to_u64(r.headers_size);
    body + headers
}

pub fn build_report(entries: &[HarEntry], top: usize) -> Report {
    let total = entries.len();
    let total_time_ms: f64 = entries.iter().map(|e| e.time).sum();
    let total_bytes: u64 = entries.iter().map(entry_bytes).sum();

    let mut by_time = entries.to_vec();
    by_time.sort_by(|a, b| {
        b.time
            .partial_cmp(&a.time)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut by_bytes = entries.to_vec();
    by_bytes.sort_by_key(|e| std::cmp::Reverse(entry_bytes(e)));

    let top_returned = top.min(total);
    let top_slowest = by_time
        .into_iter()
        .take(top_returned)
        .map(|e| {
            let bytes = entry_bytes(&e);
            ReportRow {
                url: e.request.url,
                time_ms: e.time,
                bytes,
            }
        })
        .collect();

    let top_largest = by_bytes
        .into_iter()
        .take(top_returned)
        .map(|e| {
            let bytes = entry_bytes(&e);
            ReportRow {
                url: e.request.url,
                time_ms: e.time,
                bytes,
            }
        })
        .collect();

    Report {
        entries: total,
        total_time_ms,
        total_bytes,
        top_requested: top,
        top_returned,
        top_slowest,
        top_largest,
    }
}

pub fn format_bytes(n: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;

    let nf = n as f64;

    if n < 1024 {
        format!("{} B", n)
    } else if nf < MB {
        format!("{:.2} KB", nf / KB)
    } else if nf < GB {
        format!("{:.2} MB", nf / MB)
    } else {
        format!("{:.2} GB", nf / GB)
    }
}

#[cfg(test)]
mod tests {
    use crate::har::{HarEntry, HarRequest, HarResponse, HarResponseContent};

    use super::*;

    fn mk_entry(url: &str, time: f64, body_size: Option<i64>, headers_size: Option<i64>, content_size: Option<i64>) -> HarEntry {
        HarEntry {
            time,
            request: HarRequest {
                url: url.to_string(),
            },
            response: HarResponse {
                body_size,
                headers_size,
                content: content_size.map(|size| HarResponseContent { size: Some(size) }),
            },
        }
    }

    #[test]
    fn pos_i64_conversion_handles_missing_non_positive_values() {
        assert_eq!(pos_i64_to_u64(None), 0);
        assert_eq!(pos_i64_to_u64(Some(-1)), 0);
        assert_eq!(pos_i64_to_u64(Some(0)), 0);
        assert_eq!(pos_i64_to_u64(Some(42)), 42);
    }

    #[test]
    fn entry_bytes_uses_max_body_or_content_plus_headers() {
        let e = mk_entry("https://a", 10.0, Some(100), Some(20), Some(120));
        assert_eq!(entry_bytes(&e), 140);

        let e2 = mk_entry("https://b", 10.0, Some(300), Some(50), Some(120));
        assert_eq!(entry_bytes(&e2), 350);
    }

    #[test]
    fn build_report_computes_totals_and_top_lists() {
        let entries = vec![
            mk_entry("https://a", 200.0, Some(100), Some(10), Some(90)),
            mk_entry("https://b", 50.0, Some(500), Some(5), Some(250)),
            mk_entry("https://c", 100.0, Some(-1), Some(7), Some(300)),
        ];

        let report = build_report(&entries, 2);
        assert_eq!(report.entries, 3);
        assert_eq!(report.total_time_ms, 350.0);
        assert_eq!(report.total_bytes, 922);
        assert_eq!(report.top_requested, 2);
        assert_eq!(report.top_returned, 2);

        assert_eq!(report.top_slowest[0].url, "https://a");
        assert_eq!(report.top_slowest[1].url, "https://c");

        assert_eq!(report.top_largest[0].url, "https://b");
        assert_eq!(report.top_largest[1].url, "https://c");
    }

    #[test]
    fn build_report_caps_top_to_entry_count() {
        let entries = vec![mk_entry("https://one", 1.0, Some(1), Some(1), None)];
        let report = build_report(&entries, 10);
        assert_eq!(report.top_returned, 1);
        assert_eq!(report.top_slowest.len(), 1);
        assert_eq!(report.top_largest.len(), 1);
    }
}
