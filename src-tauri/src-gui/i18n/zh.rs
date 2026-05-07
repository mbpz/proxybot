use std::collections::HashMap;

pub fn translations() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // Tab labels
    m.insert("traffic", "流量");
    m.insert("rules", "规则");
    m.insert("devices", "设备");
    m.insert("certs", "证书");
    m.insert("dns", "DNS");
    m.insert("alerts", "告警");
    m.insert("replay", "回放");
    m.insert("graph", "图表");
    m.insert("gen", "生成");
    // Common actions
    m.insert("start_proxy", "启动代理");
    m.insert("stop_proxy", "停止代理");
    m.insert("filter", "搜索...");
    m.insert("filter_placeholder", "按主机或路径筛选...");
    m.insert("clear", "清除");
    m.insert("save", "保存");
    m.insert("cancel", "取消");
    m.insert("delete", "删除");
    m.insert("add", "添加");
    m.insert("export", "导出");
    m.insert("refresh", "刷新");
    // Traffic tab
    m.insert(
        "request_list_placeholder",
        "请求列表 - 捕获流量后在此查看请求",
    );
    // Rules tab
    m.insert("rules_management", "规则管理 - 添加/编辑/删除规则");
    m.insert("add_rule", "添加规则");
    // Devices tab
    m.insert(
        "devices_table_placeholder",
        "设备表 - MAC, 最后活动, 上行/下行流量, 应用, 规则",
    );
    // Certs tab
    m.insert("cert_management", "证书管理 - 查看已安装CA, 生成的叶子证书");
    // DNS tab
    m.insert("toggle_upstream", "切换上游");
    m.insert("disable_blocklist", "禁用黑名单");
    m.insert("enable_blocklist", "启用黑名单");
    m.insert("plain_udp", "纯UDP");
    m.insert("dns_over_https", "DNS-over-HTTPS");
    m.insert("dns_query_log", "DNS查询日志");
    m.insert("no_dns_queries", "暂无DNS查询记录");
    m.insert("name", "名称");
    m.insert("timestamp", "时间戳");
    m.insert("latency_ms", "延迟 (ms)");
    m.insert("blocked", "拦截");
    m.insert("response", "响应");
    m.insert("upstream", "上游:");
    m.insert("blocklist", "黑名单:");
    m.insert("enabled", "已启用");
    m.insert("disabled", "已禁用");
    m.insert("yes", "是");
    m.insert("no", "否");
    // Alerts tab
    m.insert(
        "alert_table_placeholder",
        "告警表 - 严重级别, 来源, 描述, 确认/清除",
    );
    m.insert("clear_acknowledged", "清除已确认");
    // Replay tab
    m.insert("replay_targets", "回放目标 - 开始/停止, HAR导出, 差异视图");
    m.insert("export_har", "导出HAR");
    m.insert("show_diff", "显示差异");
    // Graph tab
    m.insert("dag_view", "DAG视图");
    m.insert("auth_state", "认证状态");
    m.insert("graph_placeholder", "ASCII DAG可视化或认证状态机");
    // Gen tab
    m.insert("mock_api", "模拟API");
    m.insert("frontend_scaffold", "前端脚手架");
    m.insert("docker_bundle", "Docker包");
    m.insert("generator_output", "生成器输出");
    m.insert("open_output_folder", "打开输出文件夹");
    m
}
