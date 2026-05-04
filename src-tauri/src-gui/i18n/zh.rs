use std::collections::HashMap;

pub fn translations() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("traffic", "流量");
    m.insert("rules", "规则");
    m.insert("devices", "设备");
    m.insert("certs", "证书");
    m.insert("dns", "DNS");
    m.insert("alerts", "告警");
    m.insert("replay", "回放");
    m.insert("graph", "图表");
    m.insert("gen", "生成");
    m.insert("start_proxy", "启动代理");
    m.insert("stop_proxy", "停止代理");
    m.insert("filter", "搜索...");
    m.insert("clear", "清除");
    m.insert("save", "保存");
    m.insert("cancel", "取消");
    m.insert("delete", "删除");
    m.insert("add", "添加");
    m.insert("export", "导出");
    m
}