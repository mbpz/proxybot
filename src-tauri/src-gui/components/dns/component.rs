use super::types::{DnsConfig, DnsQuery, UpstreamType};
use crate::i18n::t;
use yew::prelude::*;

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
            let new_type = if *upstream_type == "udp" {
                "doh"
            } else {
                "udp"
            };
            upstream_type.set(new_type.to_string());
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
            <h2>{t("dns")}</h2>
            <div class="dns-controls">
                <button onclick={toggle_upstream}>
                    {t("toggle_upstream")}
                </button>
                <button onclick={toggle_blocklist}>
                    { if config.blocklist_enabled { t("disable_blocklist") } else { t("enable_blocklist") } }
                </button>
                <select value={(*upstream_type).clone()} onchange={Callback::from(move |e: Event| {
                    if let Some(target) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
                        upstream_type.set(target.value());
                    }
                })}>
                    <option value="udp">{t("plain_udp")}</option>
                    <option value="doh">{t("dns_over_https")}</option>
                </select>
            </div>
            <div class="dns-config">
                <p>{t("upstream")}{ match config.upstream {
                    UpstreamType::PlainUdp => t("no"),
                    UpstreamType::DoH => t("yes"),
                } }</p>
                <p>{t("blocklist")}{ if config.blocklist_enabled { t("enabled") } else { t("disabled") } }</p>
            </div>
            <div class="query-log">
                <h3>{t("dns_query_log")}</h3>
                { if queries.is_empty() {
                    html! { <p>{t("no_dns_queries")}</p> }
                } else {
                    html! {
                        <table class="query-table">
                            <thead>
                                <tr>
                                    <th>{t("name")}</th>
                                    <th>{t("timestamp")}</th>
                                    <th>{t("latency_ms")}</th>
                                    <th>{t("blocked")}</th>
                                    <th>{t("response")}</th>
                                </tr>
                            </thead>
                            <tbody>
                                { for queries.iter().map(|q| {
                                    html! {
                                        <tr>
                                            <td>{ &q.name }</td>
                                            <td>{ q.timestamp }</td>
                                            <td>{ q.latency_ms }</td>
                                            <td>{ if q.blocked { t("yes") } else { t("no") } }</td>
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
