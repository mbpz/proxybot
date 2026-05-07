# 架构

## 概述

ProxyBot 在 Mac 上充当透明 HTTPS MITM 代理。当您的 iOS/Android 设备配置为使用 Mac 作为网关时，所有 HTTP/HTTPS 流量都会经过 ProxyBot，从而可以解密、检查和记录流量。

## 流量流向

```
手机 --[WiFi]--> Mac (pf 重定向 :80/:443) --> ProxyBot (MITM) --> 互联网
                                                            |
                                                            +--> DNS 服务器（记录查询，关联应用）
```

## 组件

### 1. 数据包过滤器（pf）

macOS 内置防火墙将手机的所有 HTTP/HTTPS 流量重定向到本地代理端口（8088）。这对手机是透明的 — 无需逐应用配置代理。

### 2. MITM 代理（Rust）

核心代理使用 Rust 编写：
- `hyper` 用于 HTTP 解析
- `rustls` 用于 TLS（通过动态生成的叶子证书实现 MITM）
- `tokio` 用于异步 I/O

### 3. 证书颁发机构（CA）

首次启动时，ProxyBot 会生成根 CA 证书。该 CA 必须安装在手机上并受其信任。对于每个 HTTPS 连接，ProxyBot 动态生成由根 CA 签名的叶子证书。

### 4. DNS 服务器

位于端口 53 的内置 DNS 服务器记录手机的所有 DNS 查询。这用于将 DNS 查询与观察到的流量进行关联以进行应用分类。

### 5. 应用分类

通过分析 TLS ClientHello 消息中的 SNI（服务器名称指示），并与 DNS 查询日志进行关联，ProxyBot 将流量按应用分组（微信、抖音、支付宝等）。

### 6. 规则引擎

域名规则决定如何处理流量：
- **DIRECT** — 不过 MITM 直接转发（用于银行应用等）
- **PROXY** — 通过上游代理路由
- **REJECT** — 丢弃连接
- **MAPREMOTE** — 映射到不同的远程主机
- **MAPLOCAL** — 映射到本地文件或模拟响应
- **BREAKPOINT** — 暂停等待检查后再继续

## 数据存储

- **SQLite**（`~/.proxybot/proxybot.db`）存储请求/响应历史、设备注册表和告警状态
- **证书存储**（`~/.proxybot/certs/`）用于 CA 和生成的叶子证书
- **规则文件**（`~/.proxybot/rules/`）采用 YAML 格式，修改时热重载
