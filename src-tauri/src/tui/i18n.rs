//! TUI i18n module — simple enum-based translation system.

use std::collections::HashMap;
use std::sync::Mutex;

/// Translation key enum — all keys must be listed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I18nKey {
    // Tabs
    TabTraffic, TabRules, TabDevices, TabCerts, TabDns,
    TabAlerts, TabReplay, TabGraph, TabGen,
    // Traffic
    TrafficTitle,
    TrafficCapturing, TrafficWaiting, TrafficConfigurePort,
    TrafficNoRequests, TrafficNoSelected,
    TrafficRequestHeaders, TrafficResponseHeaders,
    TrafficRequestBody, TrafficResponseBody,
    TrafficEmptyBody, TrafficNoBody,
    TrafficNotWs, TrafficNoFrames,
    TrafficSubTabsHeaders, TrafficSubTabsBody, TrafficSubTabsWsFrames, TrafficSubTabsSwitchTab,
    TrafficFilterMethod, TrafficFilterHost, TrafficFilterStatus, TrafficFilterAppTag,
    TrafficFilterHint,
    TrafficBreakpointRequest, TrafficBreakpointResponse,
    TrafficBreakpointEditHelp, TrafficBreakpointNavHelp,
    TrafficControlsRunning, TrafficControlsStopped,
    TrafficControlsCaInstalled, TrafficControlsCaNotInstalled,
    // Rules
    RulesTitle, RulesHotReload, RulesHint,
    RulesActionsDirect, RulesActionsProxy, RulesActionsReject,
    RulesActionsMapremote, RulesActionsMaplocal, RulesActionsBreakpoint,
    // Devices
    DevicesTitle, DevicesHint, DevicesOverridePrompt,
    DevicesProxyBot, DevicesThisPc, DevicesNoDevices,
    // Certs
    CertsTitle, CertsCaInfo, CertsFingerprint, CertsExpiry,
    CertsCreated, CertsStatus, CertsDaysLeft, CertsSerial,
    CertsRegenerate, CertsExport, CertsKeyBinding,
    // DNS
    DnsTitle, DnsServerStatus, DnsQueryLog, DnsUpstreamConfig, DnsHostsEntries,
    DnsRunning, DnsStopped,
    DnsToggle, DnsBlocklistToggle, DnsCycleUpstream,
    DnsNoQueries,
    // Alerts
    AlertsTitle, AlertsEmpty,
    AlertsSev1, AlertsSev2, AlertsSev3,
    // Replay
    ReplayTitle, ReplayEmpty, ReplayStatus,
    // Graph
    GraphTitle, GraphEmpty, GraphAuthEmpty,
    // Gen
    GenTitle, GenPlaceholder, GenNote,
}

impl I18nKey {
    pub fn as_str(&self) -> &'static str {
        match self {
            I18nKey::TabTraffic => "tabs.traffic",
            I18nKey::TabRules => "tabs.rules",
            I18nKey::TabDevices => "tabs.devices",
            I18nKey::TabCerts => "tabs.certs",
            I18nKey::TabDns => "tabs.dns",
            I18nKey::TabAlerts => "tabs.alerts",
            I18nKey::TabReplay => "tabs.replay",
            I18nKey::TabGraph => "tabs.graph",
            I18nKey::TabGen => "tabs.gen",
            I18nKey::TrafficTitle => "traffic.title",
            I18nKey::TrafficCapturing => "traffic.capturing",
            I18nKey::TrafficWaiting => "traffic.waiting",
            I18nKey::TrafficConfigurePort => "traffic.configure_port",
            I18nKey::TrafficNoRequests => "traffic.no_requests",
            I18nKey::TrafficNoSelected => "traffic.no_selected",
            I18nKey::TrafficRequestHeaders => "traffic.request_headers",
            I18nKey::TrafficResponseHeaders => "traffic.response_headers",
            I18nKey::TrafficRequestBody => "traffic.request_body",
            I18nKey::TrafficResponseBody => "traffic.response_body",
            I18nKey::TrafficEmptyBody => "traffic.empty_body",
            I18nKey::TrafficNoBody => "traffic.no_body",
            I18nKey::TrafficNotWs => "traffic.not_ws",
            I18nKey::TrafficNoFrames => "traffic.no_frames",
            I18nKey::TrafficSubTabsHeaders => "traffic.sub_tabs.headers",
            I18nKey::TrafficSubTabsBody => "traffic.sub_tabs.body",
            I18nKey::TrafficSubTabsWsFrames => "traffic.sub_tabs.ws_frames",
            I18nKey::TrafficSubTabsSwitchTab => "traffic.sub_tabs.switch_tab",
            I18nKey::TrafficFilterMethod => "traffic.filter.method",
            I18nKey::TrafficFilterHost => "traffic.filter.host",
            I18nKey::TrafficFilterStatus => "traffic.filter.status",
            I18nKey::TrafficFilterAppTag => "traffic.filter.app_tag",
            I18nKey::TrafficFilterHint => "traffic.filter.hint",
            I18nKey::TrafficBreakpointRequest => "traffic.breakpoint.request",
            I18nKey::TrafficBreakpointResponse => "traffic.breakpoint.response",
            I18nKey::TrafficBreakpointEditHelp => "traffic.breakpoint.edit_help",
            I18nKey::TrafficBreakpointNavHelp => "traffic.breakpoint.nav_help",
            I18nKey::TrafficControlsRunning => "traffic.controls.running",
            I18nKey::TrafficControlsStopped => "traffic.controls.stopped",
            I18nKey::TrafficControlsCaInstalled => "traffic.controls.ca_installed",
            I18nKey::TrafficControlsCaNotInstalled => "traffic.controls.ca_not_installed",
            I18nKey::RulesTitle => "rules.title",
            I18nKey::RulesHotReload => "rules.hot_reload",
            I18nKey::RulesHint => "rules.hint",
            I18nKey::RulesActionsDirect => "rules.actions.direct",
            I18nKey::RulesActionsProxy => "rules.actions.proxy",
            I18nKey::RulesActionsReject => "rules.actions.reject",
            I18nKey::RulesActionsMapremote => "rules.actions.mapremote",
            I18nKey::RulesActionsMaplocal => "rules.actions.maplocal",
            I18nKey::RulesActionsBreakpoint => "rules.actions.breakpoint",
            I18nKey::DevicesTitle => "devices.title",
            I18nKey::DevicesHint => "devices.hint",
            I18nKey::DevicesOverridePrompt => "devices.override_prompt",
            I18nKey::DevicesProxyBot => "devices.proxy_bot",
            I18nKey::DevicesThisPc => "devices.this_pc",
            I18nKey::DevicesNoDevices => "devices.no_devices",
            I18nKey::CertsTitle => "certs.title",
            I18nKey::CertsCaInfo => "certs.ca_info",
            I18nKey::CertsFingerprint => "certs.fingerprint",
            I18nKey::CertsExpiry => "certs.expiry",
            I18nKey::CertsCreated => "certs.created",
            I18nKey::CertsStatus => "certs.status",
            I18nKey::CertsDaysLeft => "certs.days_left",
            I18nKey::CertsSerial => "certs.serial",
            I18nKey::CertsRegenerate => "certs.actions.regenerate",
            I18nKey::CertsExport => "certs.actions.export",
            I18nKey::CertsKeyBinding => "certs.key_binding",
            I18nKey::DnsTitle => "dns.title",
            I18nKey::DnsServerStatus => "dns.server_status",
            I18nKey::DnsQueryLog => "dns.query_log",
            I18nKey::DnsUpstreamConfig => "dns.upstream_config",
            I18nKey::DnsHostsEntries => "dns.hosts_entries",
            I18nKey::DnsRunning => "dns.running",
            I18nKey::DnsStopped => "dns.stopped",
            I18nKey::DnsToggle => "dns.actions.toggle",
            I18nKey::DnsBlocklistToggle => "dns.actions.blocklist_toggle",
            I18nKey::DnsCycleUpstream => "dns.actions.cycle_upstream",
            I18nKey::DnsNoQueries => "dns.no_queries",
            I18nKey::AlertsTitle => "alerts.title",
            I18nKey::AlertsEmpty => "alerts.empty",
            I18nKey::AlertsSev1 => "alerts.severity.sev1",
            I18nKey::AlertsSev2 => "alerts.severity.sev2",
            I18nKey::AlertsSev3 => "alerts.severity.sev3",
            I18nKey::ReplayTitle => "replay.title",
            I18nKey::ReplayEmpty => "replay.empty",
            I18nKey::ReplayStatus => "replay.status",
            I18nKey::GraphTitle => "graph.title",
            I18nKey::GraphEmpty => "graph.empty",
            I18nKey::GraphAuthEmpty => "graph.auth_empty",
            I18nKey::GenTitle => "gen.title",
            I18nKey::GenPlaceholder => "gen.placeholder",
            I18nKey::GenNote => "gen.note",
        }
    }
}

/// Language enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    En,
    Zh,
}

impl Language {
    pub fn from_env() -> Self {
        std::env::var("PROXYBOT_LANG")
            .ok()
            .and_then(|v| match v.as_str() {
                "zh" | "ZH" | "zh-CN" | "zh_TW" => Some(Language::Zh),
                _ => Some(Language::En),
            })
            .unwrap_or(Language::En)
    }
}

fn en_map() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // Tabs
    m.insert("tabs.traffic", "Traffic");
    m.insert("tabs.rules", "Rules");
    m.insert("tabs.devices", "Devices");
    m.insert("tabs.certs", "Certs");
    m.insert("tabs.dns", "DNS");
    m.insert("tabs.alerts", "Alerts");
    m.insert("tabs.replay", "Replay");
    m.insert("tabs.graph", "Graph");
    m.insert("tabs.gen", "Gen");
    // Traffic
    m.insert("traffic.title", "Intercepted Traffic");
    m.insert("traffic.capturing", "Capturing traffic...");
    m.insert("traffic.waiting", "Waiting for requests from device...");
    m.insert("traffic.configure_port", "Configure your device to use proxy port 8088");
    m.insert("traffic.no_requests", "No requests captured. Start proxy to begin.");
    m.insert("traffic.no_selected", "No request selected.");
    m.insert("traffic.request_headers", "--- Request Headers ---");
    m.insert("traffic.response_headers", "--- Response Headers ---");
    m.insert("traffic.request_body", "--- Request Body ---");
    m.insert("traffic.response_body", "--- Response Body ---");
    m.insert("traffic.empty_body", "(empty)");
    m.insert("traffic.no_body", "(none)");
    m.insert("traffic.not_ws", "Not a WebSocket connection.");
    m.insert("traffic.no_frames", "No frames captured yet.");
    m.insert("traffic.sub_tabs.headers", "Headers");
    m.insert("traffic.sub_tabs.body", "Body");
    m.insert("traffic.sub_tabs.ws_frames", "WS Frames");
    m.insert("traffic.sub_tabs.switch_tab", "[1/2/3] switch tab");
    m.insert("traffic.filter.method", "Method");
    m.insert("traffic.filter.host", "Host");
    m.insert("traffic.filter.status", "Status");
    m.insert("traffic.filter.app_tag", "App");
    m.insert("traffic.filter.hint", "[Enter] select  [/] search  [1/2/3] detail tab  [Esc] clear filters");
    m.insert("traffic.breakpoint.request", "REQUEST BREAKPOINT");
    m.insert("traffic.breakpoint.response", "RESPONSE BREAKPOINT");
    m.insert("traffic.breakpoint.edit_help", "[e]dit [g] send [c] cancel");
    m.insert("traffic.breakpoint.nav_help", "[↑/↓] field [Enter] edit [g] send [Esc] cancel");
    m.insert("traffic.controls.running", "RUNNING");
    m.insert("traffic.controls.stopped", "STOPPED");
    m.insert("traffic.controls.ca_installed", "CA: INSTALLED");
    m.insert("traffic.controls.ca_not_installed", "CA: NOT INSTALLED");
    // Rules
    m.insert("rules.title", "Rules");
    m.insert("rules.hot_reload", "Hot-reload");
    m.insert("rules.hint", "[a]dd [e]dit [d]elete | j/k navigate");
    m.insert("rules.actions.direct", "DIRECT");
    m.insert("rules.actions.proxy", "PROXY");
    m.insert("rules.actions.reject", "REJECT");
    m.insert("rules.actions.mapremote", "MAPREMOTE");
    m.insert("rules.actions.maplocal", "MAPLOCAL");
    m.insert("rules.actions.breakpoint", "BREAKPOINT");
    // Devices
    m.insert("devices.title", "Devices");
    m.insert("devices.hint", "[a] toggle ADB | j/k navigate [e] edit rule");
    m.insert("devices.proxy_bot", "ProxyBot Server");
    m.insert("devices.this_pc", "(This PC)");
    m.insert("devices.no_devices", "(no devices connected)");
    // Certs
    m.insert("certs.title", "Certificates");
    m.insert("certs.ca_info", "CA Certificate Info");
    m.insert("certs.actions.regenerate", "[r] Regenerate CA");
    m.insert("certs.actions.export", "[e] Export CA PEM");
    m.insert("certs.key_binding", "r=regenerate, e=export, q=quit");
    // DNS
    m.insert("dns.title", "DNS");
    m.insert("dns.server_status", "DNS Server Status");
    m.insert("dns.running", "Running");
    m.insert("dns.stopped", "Stopped");
    m.insert("dns.no_queries", "No DNS queries recorded");
    // Alerts
    m.insert("alerts.title", "Alerts");
    m.insert("alerts.empty", "No alerts. New domains/IPs will trigger alerts here.");
    m.insert("alerts.severity.sev1", "SEV1");
    m.insert("alerts.severity.sev2", "SEV2");
    m.insert("alerts.severity.sev3", "SEV3");
    // Replay
    m.insert("replay.title", "Replay");
    m.insert("replay.empty", "No replay targets. Targets appear after traffic is recorded.");
    m.insert("replay.status", "HAR export:");
    // Graph
    m.insert("graph.title", "Graph");
    m.insert("graph.empty", "No traffic captured yet. Start proxy to see DAG.");
    m.insert("graph.auth_empty", "No traffic captured yet. Start proxy to see auth flow.");
    // Gen
    m.insert("gen.title", "Gen");
    m.insert("gen.placeholder", "No generation yet. Select a mode and press...");
    m.insert("gen.note", "Requires inferred APIs from captured traffic.");
    m
}

fn zh_map() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // Tabs
    m.insert("tabs.traffic", "流量");
    m.insert("tabs.rules", "规则");
    m.insert("tabs.devices", "设备");
    m.insert("tabs.certs", "证书");
    m.insert("tabs.dns", "DNS");
    m.insert("tabs.alerts", "告警");
    m.insert("tabs.replay", "回放");
    m.insert("tabs.graph", "图表");
    m.insert("tabs.gen", "生成");
    // Traffic
    m.insert("traffic.title", "拦截流量");
    m.insert("traffic.capturing", "正在捕获流量...");
    m.insert("traffic.waiting", "等待设备请求...");
    m.insert("traffic.configure_port", "请将设备配置为使用代理端口 8088");
    m.insert("traffic.no_requests", "无请求记录。启动代理开始捕获。");
    m.insert("traffic.no_selected", "未选择请求。");
    m.insert("traffic.request_headers", "--- 请求头 ---");
    m.insert("traffic.response_headers", "--- 响应头 ---");
    m.insert("traffic.request_body", "--- 请求体 ---");
    m.insert("traffic.response_body", "--- 响应体 ---");
    m.insert("traffic.empty_body", "(空)");
    m.insert("traffic.no_body", "(无)");
    m.insert("traffic.not_ws", "非 WebSocket 连接。");
    m.insert("traffic.no_frames", "尚无帧捕获。");
    m.insert("traffic.sub_tabs.headers", "头");
    m.insert("traffic.sub_tabs.body", "体");
    m.insert("traffic.sub_tabs.ws_frames", "WS帧");
    m.insert("traffic.sub_tabs.switch_tab", "[1/2/3]切换标签");
    m.insert("traffic.filter.method", "方法");
    m.insert("traffic.filter.host", "主机");
    m.insert("traffic.filter.status", "状态");
    m.insert("traffic.filter.app_tag", "应用");
    m.insert("traffic.filter.hint", "[Enter]选择 [/]搜索 [1/2/3]详情标签 [Esc]清除筛选");
    m.insert("traffic.breakpoint.request", "请求断点");
    m.insert("traffic.breakpoint.response", "响应断点");
    m.insert("traffic.breakpoint.edit_help", "[e]编辑 [g]发送 [c]取消");
    m.insert("traffic.breakpoint.nav_help", "[↑/↓]字段 [Enter]编辑 [g]发送 [Esc]取消");
    m.insert("traffic.controls.running", "运行中");
    m.insert("traffic.controls.stopped", "已停止");
    m.insert("traffic.controls.ca_installed", "CA: 已安装");
    m.insert("traffic.controls.ca_not_installed", "CA: 未安装");
    // Rules
    m.insert("rules.title", "规则");
    m.insert("rules.hot_reload", "热加载");
    m.insert("rules.hint", "[a]添加 [e]编辑 [d]删除 | j/k 上下");
    m.insert("rules.actions.direct", "直连");
    m.insert("rules.actions.proxy", "代理");
    m.insert("rules.actions.reject", "拒绝");
    m.insert("rules.actions.mapremote", "远程映射");
    m.insert("rules.actions.maplocal", "本地映射");
    m.insert("rules.actions.breakpoint", "断点");
    // Devices
    m.insert("devices.title", "设备");
    m.insert("devices.hint", "[a]切换 ADB | j/k 上下 [e]编辑规则");
    m.insert("devices.proxy_bot", "ProxyBot 服务器");
    m.insert("devices.this_pc", "(本机)");
    m.insert("devices.no_devices", "(无设备连接)");
    // Certs
    m.insert("certs.title", "证书");
    m.insert("certs.ca_info", "CA 证书信息");
    m.insert("certs.actions.regenerate", "[r] 重新生成 CA");
    m.insert("certs.actions.export", "[e] 导出 CA PEM");
    m.insert("certs.key_binding", "r=重新生成, e=导出, q=退出");
    // DNS
    m.insert("dns.title", "DNS");
    m.insert("dns.server_status", "DNS 服务器状态");
    m.insert("dns.running", "运行中");
    m.insert("dns.stopped", "已停止");
    m.insert("dns.no_queries", "无 DNS 查询记录");
    // Alerts
    m.insert("alerts.title", "告警");
    m.insert("alerts.empty", "无告警。新域名/IP 将触发告警。");
    m.insert("alerts.severity.sev1", "严重1");
    m.insert("alerts.severity.sev2", "严重2");
    m.insert("alerts.severity.sev3", "严重3");
    // Replay
    m.insert("replay.title", "回放");
    m.insert("replay.empty", "无回放目标。流量记录后将出现目标。");
    m.insert("replay.status", "HAR 导出:");
    // Graph
    m.insert("graph.title", "图表");
    m.insert("graph.empty", "尚无流量记录。启动代理查看 DAG。");
    m.insert("graph.auth_empty", "尚无流量记录。启动代理查看认证流程。");
    // Gen
    m.insert("gen.title", "生成");
    m.insert("gen.placeholder", "尚未生成。选择模式后按...");
    m.insert("gen.note", "需要从捕获流量中推断 API。");
    m
}

/// Thread-safe language state for TuiApp.
pub struct LocaleState {
    lang: Mutex<Language>,
}

impl LocaleState {
    pub fn new(lang: Language) -> Self {
        Self { lang: Mutex::new(lang) }
    }

    pub fn get(&self) -> Language {
        *self.lang.lock().unwrap()
    }

    pub fn set(&self, lang: Language) {
        *self.lang.lock().unwrap() = lang;
    }

    pub fn toggle(&self) {
        let mut guard = self.lang.lock().unwrap();
        let new = match *guard {
            Language::En => Language::Zh,
            Language::Zh => Language::En,
        };
        *guard = new;
    }
}

/// Get a translated string by key — uses English by default.
pub fn t(key: I18nKey) -> String {
    static EN_MAP: std::sync::OnceLock<HashMap<&'static str, &'static str>> = std::sync::OnceLock::new();
    let map = EN_MAP.get_or_init(en_map);
    let s = key.as_str();
    map.get(s).map(|v| (*v).to_string()).unwrap_or_else(|| s.to_string())
}

/// Get translated string with explicit language.
pub fn t_lang(key: I18nKey, lang: Language) -> String {
    match lang {
        Language::En => t(key),
        Language::Zh => {
            static ZH_MAP: std::sync::OnceLock<HashMap<&'static str, &'static str>> = std::sync::OnceLock::new();
            let map = ZH_MAP.get_or_init(zh_map);
            let s = key.as_str();
            map.get(s).map(|v| (*v).to_string()).unwrap_or_else(|| s.to_string())
        }
    }
}