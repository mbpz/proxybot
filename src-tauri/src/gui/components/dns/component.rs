use yew::prelude::*;

use super::types::{DnsConfig, DnsQuery, UpstreamType};

#[derive(Properties, PartialEq)]
pub struct DnsTabProps {}

#[function_component(DnsTab)]
pub fn dns_tab() -> Html {
    let config = use_state(|| DnsConfig {
        upstream: UpstreamType::PlainUdp,
        upstream_host: String::from("8.8.8.8"),
        blocklist_enabled: false,
    });

    let queries = use_state(Vec::<DnsQuery>::new);
    let upstream_type = use_state(|| "udp".to_string());

    let toggle_upstream = {
        let config = config.clone();
        let upstream_type = upstream_type.clone();
        Callback::from(move |_| {
            let new_type = if *upstream_type == "udp" { "doh" } else { "udp" };
            upstream_type.set(new_type.clone());
            let new_upstream = if new_type == "udp" {
                UpstreamType::PlainUdp
            } else {
                UpstreamType::DoH
            };
            config.set(DnsConfig {
                upstream: new_upstream,
                upstream_host: config.upstream_host.clone(),
                blocklist_enabled: config.blocklist_enabled,
            });
        })
    };

    let toggle_blocklist = {
        let config = config.clone();
        Callback::from(move |_| {
            config.set(DnsConfig {
                upstream: config.upstream.clone(),
                upstream_host: config.upstream_host.clone(),
                blocklist_enabled: !config.blocklist_enabled,
            });
        })
    };

    html! {
        <div class="dns-tab">
            <h2>{"DNS"}</h2>
            <div class="dns-controls">
                <button onclick={toggle_upstream}>
                    {"Toggle Upstream"}
                </button>
                <button onclick={toggle_blocklist}>
                    { if config.blocklist_enabled { "Disable Blocklist" } else { "Enable Blocklist" } }
                </button>
                <select value={(*upstream_type).clone()} onchange={Callback::from(move |e: Event| {
                    if let Some(target) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                        upstream_type.set(target.value());
                    }
                })}>
                    <option value="udp">{"Plain UDP"}</option>
                    <option value="doh">{"DNS-over-HTTPS"}</option>
                </select>
            </div>
            <div class="dns-config">
                <p>{"Upstream: "}{ match config.upstream {
                    UpstreamType::PlainUdp => "UDP",
                    UpstreamType::DoH => "DoH",
                } }</p>
                <p>{"Blocklist: "}{ if config.blocklist_enabled { "Enabled" } else { "Disabled" } }</p>
            </div>
            <div class="query-log">
                <h3>{"DNS Query Log"}</h3>
                { if queries.is_empty() {
                    html! { <p>{"No DNS queries recorded"}</p> }
                } else {
                    html! {
                        <table class="query-table">
                            <thead>
                                <tr>
                                    <th>{"Name"}</th>
                                    <th>{"Timestamp"}</th>
                                    <th>{"Latency (ms)"}</th>
                                    <th>{"Blocked"}</th>
                                    <th>{"Response"}</th>
                                </tr>
                            </thead>
                            <tbody>
                                { for queries.iter().map(|q| {
                                    html! {
                                        <tr>
                                            <td>{ &q.name }</td>
                                            <td>{ q.timestamp }</td>
                                            <td>{ q.latency_ms }</td>
                                            <td>{ if q.blocked { "Yes" } else { "No" } }</td>
                                            <td>{ q.response.as_deref().unwrap_or("-") }</td>
                                        </tr>
                                    }
                                }) }
                            </tbody>
                        </table>
                    }
                } }
            </div>
        </div>
    }
}