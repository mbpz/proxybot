//! TUI i18n module — simple enum-based translation system.

use std::collections::HashMap;
use std::sync::Mutex;

/// Translation key enum — all keys must be listed here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum I18nKey {
    // Tabs
    TabTraffic,
    TabRules,
    TabDevices,
    TabCerts,
    TabDns,
    TabAlerts,
    TabReplay,
    TabGraph,
    TabGen,
    // Traffic
    TrafficTitle,
    TrafficCapturing,
    TrafficWaiting,
    TrafficConfigurePort,
    TrafficNoRequests,
    TrafficNoSelected,
    TrafficRequestHeaders,
    TrafficResponseHeaders,
    TrafficRequestBody,
    TrafficResponseBody,
    TrafficEmptyBody,
    TrafficNoBody,
    TrafficNotWs,
    TrafficNoFrames,
    TrafficSubTabsHeaders,
    TrafficSubTabsBody,
    TrafficSubTabsWsFrames,
    TrafficSubTabsSwitchTab,
    TrafficFilterMethod,
    TrafficFilterHost,
    TrafficFilterStatus,
    TrafficFilterAppTag,
    TrafficFilterHint,
    TrafficBreakpointRequest,
    TrafficBreakpointResponse,
    TrafficBreakpointEditHelp,
    TrafficBreakpointNavHelp,
    TrafficControlsRunning,
    TrafficControlsStopped,
    TrafficControlsCaInstalled,
    TrafficControlsCaNotInstalled,
    // Rules
    RulesTitle,
    RulesHotReload,
    RulesHint,
    RulesActionsDirect,
    RulesActionsProxy,
    RulesActionsReject,
    RulesActionsMapremote,
    RulesActionsMaplocal,
    RulesActionsBreakpoint,
    RulesPatternDomain,
    RulesPatternDomainSuffix,
    RulesPatternDomainKeyword,
    RulesPatternIpCidr,
    RulesPatternGeoip,
    RulesPatternRuleSet,
    RulesRuleList,
    RulesAddRule,
    RulesEditRule,
    RulesPattern,
    RulesValue,
    RulesAction,
    RulesPatternTypes,
    RulesUseTab,
    RulesPressSToSave,
    RulesEscToCancel,
    RulesActive,
    RulesInactive,
    // Devices
    DevicesTitle,
    DevicesHint,
    DevicesOverridePrompt,
    DevicesProxyBot,
    DevicesThisPc,
    DevicesNoDevices,
    DevicesDeviceList,
    DevicesNetworkTopology,
    DevicesConfigureGateway,
    DevicesSetProxy,
    DevicesPort,
    DevicesInstallCa,
    DevicesOrUseUsb,
    DevicesUsbAdbDevices,
    DevicesLegend,
    DevicesName,
    DevicesApp,
    DevicesMac,
    // Certs
    CertsTitle,
    CertsCaInfo,
    CertsFingerprint,
    CertsExpiry,
    CertsCreated,
    CertsStatus,
    CertsDaysLeft,
    CertsSerial,
    CertsRegenerate,
    CertsExport,
    CertsKeyBinding,
    CertsActions,
    CertsExportPath,
    CertsRegenerateStatus,
    CertsUnknown,
    CertsExpired,
    CertsExpiringSoon,
    CertsValid,
    CertsFingerprintLabel,
    CertsExpiryLabel,
    CertsCreatedLabel,
    CertsStatusLabel,
    CertsDaysUntilExpiry,
    CertsSerialLabel,
    // DNS
    DnsTitle,
    DnsServerStatus,
    DnsQueryLog,
    DnsUpstreamConfig,
    DnsHostsEntries,
    DnsRunning,
    DnsStopped,
    DnsToggle,
    DnsBlocklistToggle,
    DnsCycleUpstream,
    DnsNoQueries,
    DnsConfiguration,
    DnsServer,
    DnsUpstream,
    DnsBlocklist,
    DnsHosts,
    DnsBlocklistEnabled,
    DnsBlocklistDisabled,
    DnsHostsEntriesCount,
    DnsUpstreamConfiguration,
    DnsCycleUpstreamHint,
    DnsQueryLogRecent,
    DnsNoQueriesYet,
    DnsShowingEntries,
    DnsAndMore,
    DnsKeyBindings,
    // Alerts
    AlertsTitle,
    AlertsEmpty,
    AlertsSev1,
    AlertsSev2,
    AlertsSev3,
    AlertsSummary,
    AlertsActive,
    AlertsBaseline,
    AlertsNewDomainAlerts,
    AlertsNavigateHint,
    AlertsAckHint,
    AlertsClearHint,
    AlertsEnterDetail,
    // Replay
    ReplayTitle,
    ReplayEmpty,
    ReplayStatus,
    ReplayTargets,
    ReplayDiffView,
    ReplayStatusIdle,
    ReplayRequests,
    ReplayPaths,
    ReplaySelectTarget,
    ReplayStart,
    ReplayStop,
    ReplayExportHar,
    ReplayNavigate,
    ReplayStartStop,
    ReplayExport,
    ReplayShowDiff,
    // Graph
    GraphTitle,
    GraphEmpty,
    GraphAuthEmpty,
    GraphTrafficDependencyGraph,
    GraphNoRequestPatterns,
    GraphTemporalEdges,
    GraphDagView,
    GraphAuthView,
    GraphKeyDagAuthRefresh,
    GraphAuthStateMachine,
    GraphNoExplicitAuth,
    GraphAuthMayBeEmbedded,
    GraphStateDiagram,
    GraphEntryVerifyCreds,
    GraphApiCallsAfterAuth,
    GraphInitial,
    GraphFinal,
    // Gen
    GenTitle,
    GenPlaceholder,
    GenNote,
    GenGenerator,
    GenMode,
    GenMockApi,
    GenFrontendScaffold,
    GenDockerBundle,
    GenActions,
    GenGenerateMockApi,
    GenGenerateFrontend,
    GenGenerateDocker,
    GenOpenOutputFolder,
    GenOutput,
    GenGenerating,
    GenPleaseWait,
    GenNoGenerationYet,
    GenSelectModeAndPress,
    GenOutputLast,
    GenNoteRequiresInferred,
    GenRunInference,
    GenMock,
    GenFrontend,
    GenDocker,
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
            I18nKey::DevicesDeviceList => "devices.device_list",
            I18nKey::DevicesNetworkTopology => "devices.network_topology",
            I18nKey::DevicesConfigureGateway => "devices.configure_gateway",
            I18nKey::DevicesSetProxy => "devices.set_proxy",
            I18nKey::DevicesPort => "devices.port",
            I18nKey::DevicesInstallCa => "devices.install_ca",
            I18nKey::DevicesOrUseUsb => "devices.or_use_usb",
            I18nKey::DevicesUsbAdbDevices => "devices.usb_adb_devices",
            I18nKey::DevicesLegend => "devices.legend",
            I18nKey::DevicesName => "devices.name",
            I18nKey::DevicesApp => "devices.app",
            I18nKey::DevicesMac => "devices.mac",
            I18nKey::RulesPatternDomain => "rules.pattern.domain",
            I18nKey::RulesPatternDomainSuffix => "rules.pattern.domain_suffix",
            I18nKey::RulesPatternDomainKeyword => "rules.pattern.domain_keyword",
            I18nKey::RulesPatternIpCidr => "rules.pattern.ip_cidr",
            I18nKey::RulesPatternGeoip => "rules.pattern.geoip",
            I18nKey::RulesPatternRuleSet => "rules.pattern.rule_set",
            I18nKey::RulesRuleList => "rules.rule_list",
            I18nKey::RulesAddRule => "rules.add_rule",
            I18nKey::RulesEditRule => "rules.edit_rule",
            I18nKey::RulesPattern => "rules.pattern",
            I18nKey::RulesValue => "rules.value",
            I18nKey::RulesAction => "rules.action",
            I18nKey::RulesPatternTypes => "rules.pattern_types",
            I18nKey::RulesUseTab => "rules.use_tab",
            I18nKey::RulesPressSToSave => "rules.press_s_to_save",
            I18nKey::RulesEscToCancel => "rules.esc_to_cancel",
            I18nKey::RulesActive => "rules.active",
            I18nKey::RulesInactive => "rules.inactive",
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
            I18nKey::CertsActions => "certs.actions",
            I18nKey::CertsExportPath => "certs.export_path",
            I18nKey::CertsRegenerateStatus => "certs.regenerate_status",
            I18nKey::CertsUnknown => "certs.unknown",
            I18nKey::CertsExpired => "certs.expired",
            I18nKey::CertsExpiringSoon => "certs.expiring_soon",
            I18nKey::CertsValid => "certs.valid",
            I18nKey::CertsFingerprintLabel => "certs.fingerprint_label",
            I18nKey::CertsExpiryLabel => "certs.expiry_label",
            I18nKey::CertsCreatedLabel => "certs.created_label",
            I18nKey::CertsStatusLabel => "certs.status_label",
            I18nKey::CertsDaysUntilExpiry => "certs.days_until_expiry",
            I18nKey::CertsSerialLabel => "certs.serial_label",
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
            I18nKey::DnsConfiguration => "dns.configuration",
            I18nKey::DnsServer => "dns.server",
            I18nKey::DnsUpstream => "dns.upstream",
            I18nKey::DnsBlocklist => "dns.blocklist",
            I18nKey::DnsHosts => "dns.hosts",
            I18nKey::DnsBlocklistEnabled => "dns.blocklist_enabled",
            I18nKey::DnsBlocklistDisabled => "dns.blocklist_disabled",
            I18nKey::DnsHostsEntriesCount => "dns.hosts_entries_count",
            I18nKey::DnsUpstreamConfiguration => "dns.upstream_configuration",
            I18nKey::DnsCycleUpstreamHint => "dns.cycle_upstream_hint",
            I18nKey::DnsQueryLogRecent => "dns.query_log_recent",
            I18nKey::DnsNoQueriesYet => "dns.no_queries_yet",
            I18nKey::DnsShowingEntries => "dns.showing_entries",
            I18nKey::DnsAndMore => "dns.and_more",
            I18nKey::DnsKeyBindings => "dns.key_bindings",
            I18nKey::AlertsTitle => "alerts.title",
            I18nKey::AlertsEmpty => "alerts.empty",
            I18nKey::AlertsSev1 => "alerts.severity.sev1",
            I18nKey::AlertsSev2 => "alerts.severity.sev2",
            I18nKey::AlertsSev3 => "alerts.severity.sev3",
            I18nKey::AlertsSummary => "alerts.summary",
            I18nKey::AlertsActive => "alerts.active",
            I18nKey::AlertsBaseline => "alerts.baseline",
            I18nKey::AlertsNewDomainAlerts => "alerts.new_domain_alerts",
            I18nKey::AlertsNavigateHint => "alerts.navigate_hint",
            I18nKey::AlertsAckHint => "alerts.ack_hint",
            I18nKey::AlertsClearHint => "alerts.clear_hint",
            I18nKey::AlertsEnterDetail => "alerts.enter_detail",
            I18nKey::ReplayTitle => "replay.title",
            I18nKey::ReplayEmpty => "replay.empty",
            I18nKey::ReplayStatus => "replay.status",
            I18nKey::ReplayTargets => "replay.targets",
            I18nKey::ReplayDiffView => "replay.diff_view",
            I18nKey::ReplayStatusIdle => "replay.status_idle",
            I18nKey::ReplayRequests => "replay.requests",
            I18nKey::ReplayPaths => "replay.paths",
            I18nKey::ReplaySelectTarget => "replay.select_target",
            I18nKey::ReplayStart => "replay.start",
            I18nKey::ReplayStop => "replay.stop",
            I18nKey::ReplayExportHar => "replay.export_har",
            I18nKey::ReplayNavigate => "replay.navigate",
            I18nKey::ReplayStartStop => "replay.start_stop",
            I18nKey::ReplayExport => "replay.export",
            I18nKey::ReplayShowDiff => "replay.show_diff",
            I18nKey::GraphTitle => "graph.title",
            I18nKey::GraphEmpty => "graph.empty",
            I18nKey::GraphAuthEmpty => "graph.auth_empty",
            I18nKey::GraphTrafficDependencyGraph => "graph.traffic_dependency_graph",
            I18nKey::GraphNoRequestPatterns => "graph.no_request_patterns",
            I18nKey::GraphTemporalEdges => "graph.temporal_edges",
            I18nKey::GraphDagView => "graph.dag_view",
            I18nKey::GraphAuthView => "graph.auth_view",
            I18nKey::GraphKeyDagAuthRefresh => "graph.key_dag_auth_refresh",
            I18nKey::GraphAuthStateMachine => "graph.auth_state_machine",
            I18nKey::GraphNoExplicitAuth => "graph.no_explicit_auth",
            I18nKey::GraphAuthMayBeEmbedded => "graph.auth_may_be_embedded",
            I18nKey::GraphStateDiagram => "graph.state_diagram",
            I18nKey::GraphEntryVerifyCreds => "graph.entry_verify_creds",
            I18nKey::GraphApiCallsAfterAuth => "graph.api_calls_after_auth",
            I18nKey::GraphInitial => "graph.initial",
            I18nKey::GraphFinal => "graph.final",
            I18nKey::GenTitle => "gen.title",
            I18nKey::GenPlaceholder => "gen.placeholder",
            I18nKey::GenNote => "gen.note",
            I18nKey::GenGenerator => "gen.generator",
            I18nKey::GenMode => "gen.mode",
            I18nKey::GenMockApi => "gen.mock_api",
            I18nKey::GenFrontendScaffold => "gen.frontend_scaffold",
            I18nKey::GenDockerBundle => "gen.docker_bundle",
            I18nKey::GenActions => "gen.actions",
            I18nKey::GenGenerateMockApi => "gen.generate_mock_api",
            I18nKey::GenGenerateFrontend => "gen.generate_frontend",
            I18nKey::GenGenerateDocker => "gen.generate_docker",
            I18nKey::GenOpenOutputFolder => "gen.open_output_folder",
            I18nKey::GenOutput => "gen.output",
            I18nKey::GenGenerating => "gen.generating",
            I18nKey::GenPleaseWait => "gen.please_wait",
            I18nKey::GenNoGenerationYet => "gen.no_generation_yet",
            I18nKey::GenSelectModeAndPress => "gen.select_mode_and_press",
            I18nKey::GenOutputLast => "gen.output_last",
            I18nKey::GenNoteRequiresInferred => "gen.note_requires_inferred",
            I18nKey::GenRunInference => "gen.run_inference",
            I18nKey::GenMock => "gen.mock",
            I18nKey::GenFrontend => "gen.frontend",
            I18nKey::GenDocker => "gen.docker",
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
    m.insert(
        "traffic.configure_port",
        "Configure your device to use proxy port 8088",
    );
    m.insert(
        "traffic.no_requests",
        "No requests captured. Start proxy to begin.",
    );
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
    m.insert(
        "traffic.filter.hint",
        "[Enter] select  [/] search  [1/2/3] detail tab  [Esc] clear filters",
    );
    m.insert("traffic.breakpoint.request", "REQUEST BREAKPOINT");
    m.insert("traffic.breakpoint.response", "RESPONSE BREAKPOINT");
    m.insert("traffic.breakpoint.edit_help", "[e]dit [g] send [c] cancel");
    m.insert(
        "traffic.breakpoint.nav_help",
        "[↑/↓] field [Enter] edit [g] send [Esc] cancel",
    );
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
    m.insert("rules.pattern.domain", "DOMAIN");
    m.insert("rules.pattern.domain_suffix", "DOMAIN-SUFFIX");
    m.insert("rules.pattern.domain_keyword", "DOMAIN-KEYWORD");
    m.insert("rules.pattern.ip_cidr", "IP-CIDR");
    m.insert("rules.pattern.geoip", "GEOIP");
    m.insert("rules.pattern.rule_set", "RULE-SET");
    m.insert("rules.rule_list", "Rule List");
    m.insert("rules.add_rule", "Add Rule");
    m.insert("rules.edit_rule", "Edit Rule");
    m.insert("rules.pattern", "Pattern");
    m.insert("rules.value", "Value");
    m.insert("rules.action", "Action");
    m.insert(
        "rules.pattern_types",
        "DOMAIN, DOMAIN-SUFFIX, DOMAIN-KEYWORD, IP-CIDR",
    );
    m.insert(
        "rules.use_tab",
        "Use Tab to cycle: Pattern -> Value -> Action",
    );
    m.insert("rules.press_s_to_save", "press s to save, Esc/q to cancel");
    m.insert("rules.esc_to_cancel", "Esc/q to cancel");
    m.insert("rules.active", "ACTIVE");
    m.insert("rules.inactive", "INACTIVE");
    // Devices
    m.insert("devices.title", "Devices");
    m.insert(
        "devices.hint",
        "[a] toggle ADB | j/k navigate [e] edit rule",
    );
    m.insert("devices.proxy_bot", "ProxyBot Server");
    m.insert("devices.this_pc", "(This PC)");
    m.insert("devices.no_devices", "(no devices connected)");
    m.insert("devices.device_list", "Device List");
    m.insert("devices.network_topology", "Network Topology");
    m.insert("devices.configure_gateway", "Configure device gateway:");
    m.insert("devices.set_proxy", "Set proxy to this PC");
    m.insert("devices.port", "Port: 8088");
    m.insert("devices.install_ca", "Install CA certificate");
    m.insert("devices.or_use_usb", "Or use USB with [a] toggle ADB");
    m.insert("devices.usb_adb_devices", "USB ADB Devices:");
    m.insert("devices.legend", "Legend:");
    m.insert("devices.name", "[name] = device name");
    m.insert("devices.app", "(app)  = detected app");
    m.insert("devices.mac", "MAC    = device address");
    // Certs
    m.insert("certs.title", "Certificates");
    m.insert("certs.ca_info", "CA Certificate Info");
    m.insert("certs.actions.regenerate", "[r] Regenerate CA");
    m.insert("certs.actions.export", "[e] Export CA PEM");
    m.insert("certs.key_binding", "r=regenerate, e=export, q=quit");
    m.insert("certs.actions", "Actions");
    m.insert("certs.export_path", "Export:");
    m.insert("certs.regenerate_status", "Regenerate:");
    m.insert("certs.unknown", "Unknown");
    m.insert("certs.expired", "Expired");
    m.insert("certs.expiring_soon", "Expiring Soon");
    m.insert("certs.valid", "Valid");
    m.insert("certs.fingerprint_label", "Fingerprint (SHA1):");
    m.insert("certs.expiry_label", "Expiry:");
    m.insert("certs.created_label", "Created:");
    m.insert("certs.status_label", "Status:");
    m.insert("certs.days_until_expiry", "Days until expiry:");
    m.insert("certs.serial_label", "Serial:");
    // DNS
    m.insert("dns.title", "DNS");
    m.insert("dns.server_status", "DNS Server Status");
    m.insert("dns.running", "Running");
    m.insert("dns.stopped", "Stopped");
    m.insert("dns.no_queries", "No DNS queries recorded");
    m.insert("dns.configuration", "DNS Configuration");
    m.insert("dns.server", "Server:");
    m.insert("dns.upstream", "Upstream:");
    m.insert("dns.blocklist", "Blocklist:");
    m.insert("dns.hosts", "Hosts:");
    m.insert("dns.blocklist_enabled", "Enabled ({} entries)");
    m.insert("dns.blocklist_disabled", "Disabled");
    m.insert("dns.hosts_entries_count", "{} entries");
    m.insert("dns.upstream_configuration", "Upstream Configuration");
    m.insert("dns.cycle_upstream_hint", "(u) cycle upstream type");
    m.insert("dns.query_log_recent", "DNS Query Log (recent)");
    m.insert("dns.no_queries_yet", "(no queries yet)");
    m.insert("dns.showing_entries", "showing {}/{})");
    m.insert("dns.and_more", "... and {} more");
    m.insert(
        "dns.key_bindings",
        "Key bindings: (s) toggle DNS, (b) toggle blocklist, (u) cycle upstream",
    );
    // Alerts
    m.insert("alerts.title", "Alerts");
    m.insert(
        "alerts.empty",
        "No alerts. New domains/IPs will trigger alerts here.",
    );
    m.insert("alerts.severity.sev1", "SEV1");
    m.insert("alerts.severity.sev2", "SEV2");
    m.insert("alerts.severity.sev3", "SEV3");
    m.insert("alerts.summary", "Alerts Summary");
    m.insert("alerts.active", "active");
    m.insert("alerts.baseline", "Baseline");
    m.insert("alerts.new_domain_alerts", "New domain alerts");
    m.insert("alerts.navigate_hint", "[j/k] up/down");
    m.insert("alerts.ack_hint", "[a] ack");
    m.insert("alerts.clear_hint", "[c] clear all");
    m.insert("alerts.enter_detail", "[Enter] view detail");
    // Replay
    m.insert("replay.title", "Replay");
    m.insert(
        "replay.empty",
        "No replay targets. Targets appear after traffic is recorded.",
    );
    m.insert("replay.status", "HAR export:");
    m.insert("replay.targets", "Replay Targets");
    m.insert("replay.diff_view", "Diff View");
    m.insert("replay.status_idle", "idle");
    m.insert("replay.requests", "requests");
    m.insert("replay.paths", "paths");
    m.insert(
        "replay.select_target",
        "Select a target and press [s] to start replay, [x] to stop, [e] to export HAR",
    );
    m.insert("replay.start", "start");
    m.insert("replay.stop", "stop");
    m.insert("replay.export_har", "export HAR");
    m.insert("replay.navigate", "[j/k] navigate");
    m.insert("replay.start_stop", "[s] start  [x] stop");
    m.insert("replay.export", "[e] export HAR");
    m.insert("replay.show_diff", "[d] show diff");
    // Graph
    m.insert("graph.title", "Graph");
    m.insert(
        "graph.empty",
        "No traffic captured yet. Start proxy to see DAG.",
    );
    m.insert(
        "graph.auth_empty",
        "No traffic captured yet. Start proxy to see auth flow.",
    );
    m.insert("graph.traffic_dependency_graph", "Traffic Dependency Graph");
    m.insert("graph.no_request_patterns", "No request patterns found.");
    m.insert("graph.temporal_edges", "Temporal edges:");
    m.insert("graph.dag_view", "DAG View");
    m.insert("graph.auth_view", "Auth View");
    m.insert(
        "graph.key_dag_auth_refresh",
        "[g] DAG  [a] Auth  [r] refresh",
    );
    m.insert("graph.auth_state_machine", "Auth State Machine");
    m.insert("graph.no_explicit_auth", "No explicit auth flow detected.");
    m.insert(
        "graph.auth_may_be_embedded",
        "Auth may be embedded in headers or first-party SDK.",
    );
    m.insert("graph.state_diagram", "stateDiagram-v2");
    m.insert("graph.entry_verify_creds", " : entry/verify creds");
    m.insert("graph.api_calls_after_auth", "--- API calls after auth ---");
    m.insert("graph.initial", "Initial");
    m.insert("graph.final", "Final");
    // Gen
    m.insert("gen.title", "Gen");
    m.insert(
        "gen.placeholder",
        "No generation yet. Select a mode and press...",
    );
    m.insert("gen.note", "Requires inferred APIs from captured traffic.");
    m.insert("gen.generator", "Generator");
    m.insert("gen.mode", "Mode:");
    m.insert("gen.mock_api", "Mock API");
    m.insert("gen.frontend_scaffold", "Frontend Scaffold");
    m.insert("gen.docker_bundle", "Docker Bundle");
    m.insert("gen.actions", "Actions:");
    m.insert(
        "gen.generate_mock_api",
        "[m] Generate Mock API     - Create FastAPI mock",
    );
    m.insert(
        "gen.generate_frontend",
        "[f] Generate Frontend     - React scaffold",
    );
    m.insert(
        "gen.generate_docker",
        "[d] Generate Docker      - Full deployment bundle",
    );
    m.insert(
        "gen.open_output_folder",
        "[o] Open Output Folder   - Open generated files",
    );
    m.insert("gen.output", "Output:");
    m.insert("gen.generating", "Generating... please wait");
    m.insert("gen.please_wait", "please wait");
    m.insert(
        "gen.no_generation_yet",
        "No generation yet. Select a mode and press",
    );
    m.insert(
        "gen.select_mode_and_press",
        "the corresponding key to generate.",
    );
    m.insert("gen.output_last", "Last output:");
    m.insert(
        "gen.note_requires_inferred",
        "Note: Requires inferred APIs from captured traffic.",
    );
    m.insert("gen.run_inference", "Run inference before generating.");
    m.insert("gen.mock", "Mock");
    m.insert("gen.frontend", "Frontend");
    m.insert("gen.docker", "Docker");
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
    m.insert(
        "traffic.filter.hint",
        "[Enter]选择 [/]搜索 [1/2/3]详情标签 [Esc]清除筛选",
    );
    m.insert("traffic.breakpoint.request", "请求断点");
    m.insert("traffic.breakpoint.response", "响应断点");
    m.insert("traffic.breakpoint.edit_help", "[e]编辑 [g]发送 [c]取消");
    m.insert(
        "traffic.breakpoint.nav_help",
        "[↑/↓]字段 [Enter]编辑 [g]发送 [Esc]取消",
    );
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
    m.insert("rules.pattern.domain", "域名");
    m.insert("rules.pattern.domain_suffix", "域名后缀");
    m.insert("rules.pattern.domain_keyword", "域名关键词");
    m.insert("rules.pattern.ip_cidr", "IP-CIDR");
    m.insert("rules.pattern.geoip", "GEOIP");
    m.insert("rules.pattern.rule_set", "规则集");
    m.insert("rules.rule_list", "规则列表");
    m.insert("rules.add_rule", "添加规则");
    m.insert("rules.edit_rule", "编辑规则");
    m.insert("rules.pattern", "模式");
    m.insert("rules.value", "值");
    m.insert("rules.action", "动作");
    m.insert("rules.pattern_types", "域名, 域名后缀, 域名关键词, IP-CIDR");
    m.insert("rules.use_tab", "使用 Tab 循环：模式 -> 值 -> 动作");
    m.insert("rules.press_s_to_save", "按 s 保存，Esc/q 取消");
    m.insert("rules.esc_to_cancel", "Esc/q 取消");
    m.insert("rules.active", "激活");
    m.insert("rules.inactive", "未激活");
    // Devices
    m.insert("devices.title", "设备");
    m.insert("devices.hint", "[a]切换 ADB | j/k 上下 [e]编辑规则");
    m.insert("devices.proxy_bot", "ProxyBot 服务器");
    m.insert("devices.this_pc", "(本机)");
    m.insert("devices.no_devices", "(无设备连接)");
    m.insert("devices.device_list", "设备列表");
    m.insert("devices.network_topology", "网络拓扑");
    m.insert("devices.configure_gateway", "配置设备网关：");
    m.insert("devices.set_proxy", "设置代理到本机");
    m.insert("devices.port", "端口：8088");
    m.insert("devices.install_ca", "安装 CA 证书");
    m.insert("devices.or_use_usb", "或使用 USB 按 [a] 切换 ADB");
    m.insert("devices.usb_adb_devices", "USB ADB 设备：");
    m.insert("devices.legend", "图例：");
    m.insert("devices.name", "[名称] = 设备名称");
    m.insert("devices.app", "(应用) = 检测到的应用");
    m.insert("devices.mac", "MAC = 设备地址");
    // Certs
    m.insert("certs.title", "证书");
    m.insert("certs.ca_info", "CA 证书信息");
    m.insert("certs.actions.regenerate", "[r] 重新生成 CA");
    m.insert("certs.actions.export", "[e] 导出 CA PEM");
    m.insert("certs.key_binding", "r=重新生成, e=导出, q=退出");
    m.insert("certs.actions", "操作");
    m.insert("certs.export_path", "导出：");
    m.insert("certs.regenerate_status", "重新生成：");
    m.insert("certs.unknown", "未知");
    m.insert("certs.expired", "已过期");
    m.insert("certs.expiring_soon", "即将过期");
    m.insert("certs.valid", "有效");
    m.insert("certs.fingerprint_label", "指纹 (SHA1)：");
    m.insert("certs.expiry_label", "到期时间：");
    m.insert("certs.created_label", "创建时间：");
    m.insert("certs.status_label", "状态：");
    m.insert("certs.days_until_expiry", "剩余天数：");
    m.insert("certs.serial_label", "序列号：");
    // DNS
    m.insert("dns.title", "DNS");
    m.insert("dns.server_status", "DNS 服务器状态");
    m.insert("dns.running", "运行中");
    m.insert("dns.stopped", "已停止");
    m.insert("dns.no_queries", "无 DNS 查询记录");
    m.insert("dns.configuration", "DNS 配置");
    m.insert("dns.server", "服务器：");
    m.insert("dns.upstream", "上游：");
    m.insert("dns.blocklist", "黑名单：");
    m.insert("dns.hosts", "Hosts：");
    m.insert("dns.blocklist_enabled", "已启用 ({} 条目)");
    m.insert("dns.blocklist_disabled", "已禁用");
    m.insert("dns.hosts_entries_count", "{} 条目");
    m.insert("dns.upstream_configuration", "上游配置");
    m.insert("dns.cycle_upstream_hint", "(u) 切换上游类型");
    m.insert("dns.query_log_recent", "DNS 查询日志（最近）");
    m.insert("dns.no_queries_yet", "（尚无查询）");
    m.insert("dns.showing_entries", "显示 {}/{})");
    m.insert("dns.and_more", "... 还有 {} 条");
    m.insert(
        "dns.key_bindings",
        "快捷键：(s) 切换 DNS，(b) 切换黑名单，(u) 切换上游",
    );
    // Alerts
    m.insert("alerts.title", "告警");
    m.insert("alerts.empty", "无告警。新域名/IP 将触发告警。");
    m.insert("alerts.severity.sev1", "严重1");
    m.insert("alerts.severity.sev2", "严重2");
    m.insert("alerts.severity.sev3", "严重3");
    m.insert("alerts.summary", "告警摘要");
    m.insert("alerts.active", "活跃");
    m.insert("alerts.baseline", "基线");
    m.insert("alerts.new_domain_alerts", "新域名告警");
    m.insert("alerts.navigate_hint", "[j/k] 上/下");
    m.insert("alerts.ack_hint", "[a] 确认");
    m.insert("alerts.clear_hint", "[c] 清除全部");
    m.insert("alerts.enter_detail", "[Enter] 查看详情");
    // Replay
    m.insert("replay.title", "回放");
    m.insert("replay.empty", "无回放目标。流量记录后将出现目标。");
    m.insert("replay.status", "HAR 导出：");
    m.insert("replay.targets", "回放目标");
    m.insert("replay.diff_view", "差异视图");
    m.insert("replay.status_idle", "空闲");
    m.insert("replay.requests", "请求");
    m.insert("replay.paths", "路径");
    m.insert(
        "replay.select_target",
        "选择目标后按 [s] 开始回放，[x] 停止，[e] 导出 HAR",
    );
    m.insert("replay.start", "开始");
    m.insert("replay.stop", "停止");
    m.insert("replay.export_har", "导出 HAR");
    m.insert("replay.navigate", "[j/k] 导航");
    m.insert("replay.start_stop", "[s] 开始  [x] 停止");
    m.insert("replay.export", "[e] 导出 HAR");
    m.insert("replay.show_diff", "[d] 显示差异");
    // Graph
    m.insert("graph.title", "图表");
    m.insert("graph.empty", "尚无流量记录。启动代理查看 DAG。");
    m.insert("graph.auth_empty", "尚无流量记录。启动代理查看认证流程。");
    m.insert("graph.traffic_dependency_graph", "流量依赖图");
    m.insert("graph.no_request_patterns", "未找到请求模式。");
    m.insert("graph.temporal_edges", "时序边：");
    m.insert("graph.dag_view", "DAG 视图");
    m.insert("graph.auth_view", "认证视图");
    m.insert("graph.key_dag_auth_refresh", "[g] DAG  [a] 认证  [r] 刷新");
    m.insert("graph.auth_state_machine", "认证状态机");
    m.insert("graph.no_explicit_auth", "未检测到明确的认证流程。");
    m.insert(
        "graph.auth_may_be_embedded",
        "认证可能嵌入在头或第一方 SDK 中。",
    );
    m.insert("graph.state_diagram", "状态图-v2");
    m.insert("graph.entry_verify_creds", " : 进入/验证凭据");
    m.insert("graph.api_calls_after_auth", "--- 认证后的 API 调用 ---");
    m.insert("graph.initial", "初始");
    m.insert("graph.final", "最终");
    // Gen
    m.insert("gen.title", "生成");
    m.insert("gen.placeholder", "尚未生成。选择模式后按...");
    m.insert("gen.note", "需要从捕获流量中推断 API。");
    m.insert("gen.generator", "生成器");
    m.insert("gen.mode", "模式：");
    m.insert("gen.mock_api", "Mock API");
    m.insert("gen.frontend_scaffold", "前端脚手架");
    m.insert("gen.docker_bundle", "Docker 部署包");
    m.insert("gen.actions", "操作：");
    m.insert(
        "gen.generate_mock_api",
        "[m] 生成 Mock API     - 创建 FastAPI mock",
    );
    m.insert("gen.generate_frontend", "[f] 生成前端     - React 脚手架");
    m.insert("gen.generate_docker", "[d] 生成 Docker      - 完整部署包");
    m.insert(
        "gen.open_output_folder",
        "[o] 打开输出文件夹   - 打开生成的文件",
    );
    m.insert("gen.output", "输出：");
    m.insert("gen.generating", "生成中... 请稍候");
    m.insert("gen.please_wait", "请稍候");
    m.insert("gen.no_generation_yet", "尚未生成。选择模式后按");
    m.insert("gen.select_mode_and_press", "对应键生成。");
    m.insert("gen.output_last", "最近输出：");
    m.insert(
        "gen.note_requires_inferred",
        "注意：需要从捕获流量中推断 API。",
    );
    m.insert("gen.run_inference", "先生成推理。");
    m.insert("gen.mock", "Mock");
    m.insert("gen.frontend", "前端");
    m.insert("gen.docker", "Docker");
    m
}

/// Thread-safe language state for TuiApp.
pub struct LocaleState {
    lang: Mutex<Language>,
}

impl LocaleState {
    pub fn new(lang: Language) -> Self {
        Self {
            lang: Mutex::new(lang),
        }
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
    static EN_MAP: std::sync::OnceLock<HashMap<&'static str, &'static str>> =
        std::sync::OnceLock::new();
    let map = EN_MAP.get_or_init(en_map);
    let s = key.as_str();
    map.get(s)
        .map(|v| (*v).to_string())
        .unwrap_or_else(|| s.to_string())
}

/// Get translated string with explicit language.
pub fn t_lang(key: I18nKey, lang: Language) -> String {
    match lang {
        Language::En => t(key),
        Language::Zh => {
            static ZH_MAP: std::sync::OnceLock<HashMap<&'static str, &'static str>> =
                std::sync::OnceLock::new();
            let map = ZH_MAP.get_or_init(zh_map);
            let s = key.as_str();
            map.get(s)
                .map(|v| (*v).to_string())
                .unwrap_or_else(|| s.to_string())
        }
    }
}
