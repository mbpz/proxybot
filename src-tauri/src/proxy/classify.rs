//! Capture Event Adapter for core Application Attribution.

use crate::dns::DnsState;

pub fn classify_captured_request(
    host: &str,
    scheme: &str,
    client_ip: &str,
    upstream_ip: Option<&str>,
    timestamp: &str,
    dns_state: &DnsState,
) -> Option<(String, String)> {
    let captured_at_ms = parse_capture_timestamp_ms(timestamp)?;
    dns_state
        .attribute_connection(
            host,
            (scheme == "https").then_some(host),
            client_ip,
            upstream_ip,
            captured_at_ms,
        )
        .map(|attribution| {
            (
                attribution.app_name,
                attribution.app_icon.unwrap_or_default(),
            )
        })
}

fn parse_capture_timestamp_ms(timestamp: &str) -> Option<u64> {
    let (seconds, millis) = timestamp.split_once('.').unwrap_or((timestamp, "0"));
    let seconds = seconds.parse::<u64>().ok()?;
    let mut millis = millis.chars().take(3).collect::<String>();
    while millis.len() < 3 {
        millis.push('0');
    }
    seconds
        .checked_mul(1_000)?
        .checked_add(millis.parse::<u64>().ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_runtime_timestamp_without_float_rounding() {
        assert_eq!(parse_capture_timestamp_ms("123.4"), Some(123_400));
        assert_eq!(parse_capture_timestamp_ms("123.4567"), Some(123_456));
    }
}
