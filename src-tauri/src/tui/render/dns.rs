//! DNS tab renderer.
//!
//! Shows DNS configuration, upstream selector, blocklist toggle, hosts entries,
//! and live DNS query log.

use ratatui::text::{Line, Span};
use ratatui::{
    layout::Rect,
    style::Color,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::tui::i18n::{t as tr, I18nKey as K};
use crate::tui::TuiApp;

fn fmt_ts_ms(ts_ms: u64) -> String {
    let total_secs = ts_ms / 1000;
    let hours = (total_secs / 3600) % 24;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;
    let ms = ts_ms % 1000;
    format!("{:02}:{:02}:{:02}.{:03}", hours, mins, secs, ms)
}

/// Render the DNS tab.
pub fn render(f: &mut Frame, area: Rect, app: &TuiApp) {
    let dns_config_title = tr(K::DnsConfiguration);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(dns_config_title.as_str());

    // Gather DNS state info
    let upstream = app.dns_state.get_upstream();
    let upstream_label = match upstream.upstream_type {
        crate::dns::DnsUpstreamType::PlainUdp => format!("Plain UDP: {}", upstream.address),
        crate::dns::DnsUpstreamType::Doh => format!("DoH: {}", upstream.address),
    };

    let dns_running = app
        .dns_state
        .running
        .load(std::sync::atomic::Ordering::SeqCst);

    // Get entries for display (cloned to avoid borrow issues)
    let entries = app.dns_state.entries.lock().unwrap();
    let recent_entries: Vec<_> = entries.iter().rev().take(20).map(|e| e.clone()).collect();
    drop(entries);

    let hosts_guard = app.dns_state.hosts.lock().unwrap();
    let hosts_count = hosts_guard.len();

    let blocklist_guard = app.dns_state.blocklist.lock().unwrap();
    let blocklist_count = blocklist_guard.len();

    // Build content
    let mut lines = Vec::new();

    // === DNS Server Status ===
    lines.push(Line::from(vec![Span::raw(tr(K::DnsServerStatus))]));
    lines.push(Line::from(vec![Span::raw("─".repeat(50))]));
    let status_str = if dns_running {
        tr(K::DnsRunning)
    } else {
        tr(K::DnsStopped)
    };
    lines.push(Line::from(vec![
        Span::raw(tr(K::DnsServer)),
        Span::raw(status_str).style(Color::Green),
    ]));
    lines.push(Line::from(vec![
        Span::raw(tr(K::DnsUpstream)),
        Span::raw(&upstream_label).style(Color::Cyan),
    ]));
    lines.push(Line::from(vec![
        Span::raw(tr(K::DnsBlocklist)),
        if blocklist_count > 0 {
            Span::raw(format!(
                "{} ({})",
                tr(K::DnsBlocklistEnabled),
                blocklist_count
            ))
            .style(Color::Green)
        } else {
            Span::raw(tr(K::DnsBlocklistDisabled)).style(Color::Gray)
        },
    ]));
    lines.push(Line::from(vec![
        Span::raw(tr(K::DnsHosts)),
        Span::raw(format!("{} {}", hosts_count, tr(K::DnsHostsEntriesCount))).style(Color::Yellow),
    ]));
    lines.push(Line::from(vec![]));

    // === Upstream Selector ===
    lines.push(Line::from(vec![Span::raw(tr(K::DnsUpstreamConfiguration))]));
    lines.push(Line::from(vec![Span::raw("─".repeat(50))]));
    lines.push(Line::from(vec![Span::raw(tr(K::DnsCycleUpstreamHint))]));
    lines.push(Line::from(vec![]));

    // === Query Log ===
    lines.push(Line::from(vec![Span::raw(tr(K::DnsQueryLogRecent))]));
    lines.push(Line::from(vec![Span::raw("─".repeat(50))]));

    if recent_entries.is_empty() {
        lines.push(Line::from(vec![
            Span::raw(tr(K::DnsNoQueriesYet)).style(Color::Gray)
        ]));
    } else {
        for entry in recent_entries.iter() {
            let ts_str = fmt_ts_ms(entry.timestamp_ms);
            let domain = entry.domain.clone();
            let ips = if entry.resolved_ips.is_empty() {
                String::from("-")
            } else {
                entry.resolved_ips.join(", ")
            };
            let app_tag = entry.app_name.as_deref().unwrap_or("-").to_string();

            // Build the line as a single formatted string
            let line_str = format!("[{}] {} -> {} ({})", ts_str, domain, ips, app_tag);
            lines.push(Line::from(vec![Span::raw(line_str)]));
        }
    }

    lines.push(Line::from(vec![]));

    // === Hosts Entries (first 10) ===
    if hosts_count > 0 {
        let showing_entries = format!(
            "{} ({}/{})",
            tr(K::DnsHostsEntries),
            10.min(hosts_count),
            hosts_count
        );
        lines.push(Line::from(vec![Span::raw(showing_entries)]));
        lines.push(Line::from(vec![Span::raw("─".repeat(50))]));
        for h in hosts_guard.iter().take(10) {
            lines.push(Line::from(vec![Span::raw(format!(
                "  {} -> {}",
                h.domain, h.ip
            ))]));
        }
        if hosts_count > 10 {
            lines.push(Line::from(vec![Span::raw(format!(
                "  {} {}",
                tr(K::DnsAndMore),
                hosts_count - 10
            ))]));
        }
        lines.push(Line::from(vec![]));
    }

    drop(hosts_guard);
    drop(blocklist_guard);

    // === Key Bindings ===
    lines.push(Line::from(vec![Span::raw("─".repeat(50))]));
    lines.push(Line::from(vec![Span::raw(tr(K::DnsKeyBindings))]));

    let content = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    f.render_widget(content, area);
}
