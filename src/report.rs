use std::collections::HashMap;

use clap::ValueEnum;
use serde::Serialize;

use crate::har::HarEntry;

#[derive(Debug, Clone, Copy, Serialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum GroupBy {
    Host,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub entries: usize,
    pub total_time_ms: f64,
    pub total_bytes: u64,
    pub top_requested: usize,
    pub top_returned: usize,
    pub group_by: Option<GroupBy>,
    pub top_slowest: Vec<ReportRow>,
    pub top_largest: Vec<ReportRow>,
    pub top_groups: Vec<GroupRow>,
}

#[derive(Debug, Serialize)]
pub struct ReportRow {
    pub url: String,
    pub time_ms: f64,
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct GroupRow {
    pub key: String,
    pub count: usize,
    pub total_time_ms: f64,
    pub avg_time_ms: f64,
    pub p95_time_ms: f64,
    pub total_bytes: u64,
}

#[derive(Debug, Default)]
struct GroupAccumulator {
    count: usize,
    total_time_ms: f64,
    total_bytes: u64,
    times: Vec<f64>,
}

pub fn pos_i64_to_u64(x: Option<i64>) -> u64 {
    match x {
        Some(v) if v > 0 => v as u64,
        _ => 0,
    }
}

pub fn entry_bytes(e: &HarEntry) -> u64 {
    let r = &e.response;
    let body =
        pos_i64_to_u64(r.body_size).max(pos_i64_to_u64(r.content.as_ref().and_then(|c| c.size)));
    let headers = pos_i64_to_u64(r.headers_size);
    body + headers
}

fn nearest_rank_percentile(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let rank = (p * values.len() as f64).ceil() as usize;
    let idx = rank.saturating_sub(1).min(values.len() - 1);
    values[idx]
}

fn host_key(url: &str) -> String {
    let Some((_, after_scheme)) = url.split_once("://") else {
        return "<invalid-host>".to_string();
    };

    let authority = after_scheme.split('/').next().unwrap_or_default();
    if authority.is_empty() {
        return "<invalid-host>".to_string();
    }

    let host_port = authority.rsplit('@').next().unwrap_or_default();
    if host_port.is_empty() {
        return "<invalid-host>".to_string();
    }

    let host = if let Some(stripped) = host_port.strip_prefix('[') {
        let Some((ipv6, _)) = stripped.split_once(']') else {
            return "<invalid-host>".to_string();
        };
        if ipv6.is_empty() {
            return "<invalid-host>".to_string();
        }
        format!("[{}]", ipv6)
    } else {
        host_port.split(':').next().unwrap_or_default().to_string()
    };

    if host.is_empty() {
        "<invalid-host>".to_string()
    } else {
        host.to_ascii_lowercase()
    }
}

fn build_top_groups(entries: &[HarEntry], top: usize, group_by: Option<GroupBy>) -> Vec<GroupRow> {
    let Some(GroupBy::Host) = group_by else {
        return Vec::new();
    };

    let mut groups: HashMap<String, GroupAccumulator> = HashMap::new();
    for entry in entries {
        let key = host_key(&entry.request.url);
        let acc = groups.entry(key).or_default();
        acc.count += 1;
        acc.total_time_ms += entry.time;
        acc.total_bytes += entry_bytes(entry);
        acc.times.push(entry.time);
    }

    let mut rows: Vec<GroupRow> = groups
        .into_iter()
        .map(|(key, mut acc)| {
            acc.times
                .sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

            let p95_time_ms = nearest_rank_percentile(&acc.times, 0.95);
            GroupRow {
                key,
                count: acc.count,
                total_time_ms: acc.total_time_ms,
                avg_time_ms: acc.total_time_ms / acc.count as f64,
                p95_time_ms,
                total_bytes: acc.total_bytes,
            }
        })
        .collect();

    rows.sort_by(|a, b| {
        b.total_time_ms
            .partial_cmp(&a.total_time_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.key.cmp(&b.key))
    });
    rows.into_iter().take(top).collect()
}

pub fn build_report(entries: &[HarEntry], top: usize, group_by: Option<GroupBy>) -> Report {
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
    let top_groups = build_top_groups(entries, top, group_by);
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
        group_by,
        top_slowest,
        top_largest,
        top_groups,
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

    fn mk_entry(
        url: &str,
        time: f64,
        body_size: Option<i64>,
        headers_size: Option<i64>,
        content_size: Option<i64>,
    ) -> HarEntry {
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

        let report = build_report(&entries, 2, None);
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
        let report = build_report(&entries, 10, None);
        assert_eq!(report.top_returned, 1);
        assert_eq!(report.top_slowest.len(), 1);
        assert_eq!(report.top_largest.len(), 1);
    }

    #[test]
    fn nearest_rank_percentile_handles_small_samples() {
        assert_eq!(nearest_rank_percentile(&[], 0.95), 0.0);
        assert_eq!(nearest_rank_percentile(&[10.0], 0.95), 10.0);
        assert_eq!(nearest_rank_percentile(&[10.0, 20.0], 0.95), 20.0);
        assert_eq!(nearest_rank_percentile(&[10.0, 20.0, 30.0], 0.95), 30.0);
        assert_eq!(nearest_rank_percentile(&[1.0, 2.0, 3.0, 4.0], 0.5), 2.0);
    }

    #[test]
    fn build_report_with_group_by_host_computes_group_metrics() {
        let entries = vec![
            mk_entry(
                "https://api.example.com/a",
                200.0,
                Some(100),
                Some(10),
                None,
            ),
            mk_entry(
                "https://api.example.com/b",
                100.0,
                Some(200),
                Some(20),
                None,
            ),
            mk_entry(
                "https://cdn.example.com/c",
                300.0,
                Some(300),
                Some(30),
                None,
            ),
            mk_entry("https://cdn.example.com/d", 50.0, Some(50), Some(10), None),
        ];

        let report = build_report(&entries, 2, Some(GroupBy::Host));
        assert_eq!(report.top_groups.len(), 2);

        assert_eq!(report.top_groups[0].key, "cdn.example.com");
        assert_eq!(report.top_groups[0].count, 2);
        assert_eq!(report.top_groups[0].total_time_ms, 350.0);
        assert_eq!(report.top_groups[0].avg_time_ms, 175.0);
        assert_eq!(report.top_groups[0].p95_time_ms, 300.0);
        assert_eq!(report.top_groups[0].total_bytes, 390);

        assert_eq!(report.top_groups[1].key, "api.example.com");
        assert_eq!(report.top_groups[1].count, 2);
        assert_eq!(report.top_groups[1].total_time_ms, 300.0);
        assert_eq!(report.top_groups[1].avg_time_ms, 150.0);
        assert_eq!(report.top_groups[1].p95_time_ms, 200.0);
        assert_eq!(report.top_groups[1].total_bytes, 330);
    }

    #[test]
    fn build_report_group_sorting_tiebreaks_on_key() {
        let entries = vec![
            mk_entry("https://z.example.com/1", 100.0, Some(1), Some(0), None),
            mk_entry("https://a.example.com/1", 100.0, Some(1), Some(0), None),
        ];

        let report = build_report(&entries, 2, Some(GroupBy::Host));
        assert_eq!(report.top_groups.len(), 2);
        assert_eq!(report.top_groups[0].key, "a.example.com");
        assert_eq!(report.top_groups[1].key, "z.example.com");
    }

    #[test]
    fn build_report_grouping_buckets_invalid_url() {
        let entries = vec![
            mk_entry("not a url", 10.0, Some(10), Some(0), None),
            mk_entry("also bad", 20.0, Some(20), Some(0), None),
        ];

        let report = build_report(&entries, 5, Some(GroupBy::Host));
        assert_eq!(report.top_groups.len(), 1);
        assert_eq!(report.top_groups[0].key, "<invalid-host>");
        assert_eq!(report.top_groups[0].count, 2);
    }
}
