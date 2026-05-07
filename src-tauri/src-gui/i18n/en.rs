use std::collections::HashMap;

pub fn translations() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // Tab labels
    m.insert("traffic", "Traffic");
    m.insert("rules", "Rules");
    m.insert("devices", "Devices");
    m.insert("certs", "Certs");
    m.insert("dns", "DNS");
    m.insert("alerts", "Alerts");
    m.insert("replay", "Replay");
    m.insert("graph", "Graph");
    m.insert("gen", "Gen");
    // Common actions
    m.insert("start_proxy", "Start Proxy");
    m.insert("stop_proxy", "Stop Proxy");
    m.insert("filter", "Filter...");
    m.insert("filter_placeholder", "Filter by host or path...");
    m.insert("clear", "Clear");
    m.insert("save", "Save");
    m.insert("cancel", "Cancel");
    m.insert("delete", "Delete");
    m.insert("add", "Add");
    m.insert("export", "Export");
    m.insert("refresh", "Refresh");
    // Traffic tab
    m.insert(
        "request_list_placeholder",
        "Request list - capture traffic to see requests here",
    );
    // Rules tab
    m.insert(
        "rules_management",
        "Rules management - add/edit/delete rules",
    );
    m.insert("add_rule", "Add Rule");
    // Devices tab
    m.insert(
        "devices_table_placeholder",
        "Devices table - MAC, last seen, bytes up/down, app, rule",
    );
    // Certs tab
    m.insert(
        "cert_management",
        "Certificate management - view installed CA, generated leaf certs",
    );
    // DNS tab
    m.insert("toggle_upstream", "Toggle Upstream");
    m.insert("disable_blocklist", "Disable Blocklist");
    m.insert("enable_blocklist", "Enable Blocklist");
    m.insert("plain_udp", "Plain UDP");
    m.insert("dns_over_https", "DNS-over-HTTPS");
    m.insert("dns_query_log", "DNS Query Log");
    m.insert("no_dns_queries", "No DNS queries recorded");
    m.insert("name", "Name");
    m.insert("timestamp", "Timestamp");
    m.insert("latency_ms", "Latency (ms)");
    m.insert("blocked", "Blocked");
    m.insert("response", "Response");
    m.insert("upstream", "Upstream:");
    m.insert("blocklist", "Blocklist:");
    m.insert("enabled", "Enabled");
    m.insert("disabled", "Disabled");
    m.insert("yes", "Yes");
    m.insert("no", "No");
    // Alerts tab
    m.insert(
        "alert_table_placeholder",
        "Alert table - SEV1/2/3, source, description, ACK/Clear",
    );
    m.insert("clear_acknowledged", "Clear Acknowledged");
    // Replay tab
    m.insert(
        "replay_targets",
        "Replay targets - start/stop, HAR export, diff view",
    );
    m.insert("export_har", "Export HAR");
    m.insert("show_diff", "Show Diff");
    // Graph tab
    m.insert("dag_view", "DAG View");
    m.insert("auth_state", "Auth State");
    m.insert(
        "graph_placeholder",
        "ASCII DAG visualization or auth state machine",
    );
    // Gen tab
    m.insert("mock_api", "Mock API");
    m.insert("frontend_scaffold", "Frontend Scaffold");
    m.insert("docker_bundle", "Docker Bundle");
    m.insert("generator_output", "Generator output");
    m.insert("open_output_folder", "Open Output Folder");
    m
}
