use std::collections::HashMap;

pub fn translations() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("traffic", "Traffic");
    m.insert("rules", "Rules");
    m.insert("devices", "Devices");
    m.insert("certs", "Certs");
    m.insert("dns", "DNS");
    m.insert("alerts", "Alerts");
    m.insert("replay", "Replay");
    m.insert("graph", "Graph");
    m.insert("gen", "Gen");
    m.insert("start_proxy", "Start Proxy");
    m.insert("stop_proxy", "Stop Proxy");
    m.insert("filter", "Filter...");
    m.insert("clear", "Clear");
    m.insert("save", "Save");
    m.insert("cancel", "Cancel");
    m.insert("delete", "Delete");
    m.insert("add", "Add");
    m.insert("export", "Export");
    m
}