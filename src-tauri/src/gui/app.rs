use yew::prelude::*;
use crate::gui::components::{
    alerts::AlertsTab,
    certs::CertsTab,
    devices::DevicesTab,
    dns::DnsTab,
    gen::GenTab,
    graph::GraphTab,
    replay::ReplayTab,
    rules::RulesTab,
    traffic::TrafficTab,
};

#[derive(PartialEq, Clone, Default)]
pub enum Tab {
    #[default]
    Traffic,
    Rules,
    Devices,
    Certs,
    Dns,
    Alerts,
    Replay,
    Graph,
    Gen,
}

#[derive(Properties, PartialEq)]
pub struct AppProps {
    #[prop_or_default]
    pub active_tab: Tab,
}

#[function_component(App)]
pub fn app(props: &AppProps) -> Html {
    let active = props.active_tab.clone();

    html! {
        <div class="app-container">
            <nav class="tab-nav">
                <button class={format!("tab-btn{}", if active == Tab::Traffic { " active" } else { "" })}>
                    { "Traffic" }
                </button>
                <button class={format!("tab-btn{}", if active == Tab::Rules { " active" } else { "" })}>
                    { "Rules" }
                </button>
                <button class={format!("tab-btn{}", if active == Tab::Devices { " active" } else { "" })}>
                    { "Devices" }
                </button>
                <button class={format!("tab-btn{}", if active == Tab::Certs { " active" } else { "" })}>
                    { "Certs" }
                </button>
                <button class={format!("tab-btn{}", if active == Tab::Dns { " active" } else { "" })}>
                    { "DNS" }
                </button>
                <button class={format!("tab-btn{}", if active == Tab::Alerts { " active" } else { "" })}>
                    { "Alerts" }
                </button>
                <button class={format!("tab-btn{}", if active == Tab::Replay { " active" } else { "" })}>
                    { "Replay" }
                </button>
                <button class={format!("tab-btn{}", if active == Tab::Graph { " active" } else { "" })}>
                    { "Graph" }
                </button>
                <button class={format!("tab-btn{}", if active == Tab::Gen { " active" } else { "" })}>
                    { "Gen" }
                </button>
            </nav>
            <main class="content">
                { match props.active_tab {
                    Tab::Traffic => html! { <TrafficTab /> },
                    Tab::Rules => html! { <RulesTab /> },
                    Tab::Devices => html! { <DevicesTab /> },
                    Tab::Certs => html! { <CertsTab /> },
                    Tab::Dns => html! { <DnsTab /> },
                    Tab::Alerts => html! { <AlertsTab /> },
                    Tab::Replay => html! { <ReplayTab /> },
                    Tab::Graph => html! { <GraphTab /> },
                    Tab::Gen => html! { <GenTab /> },
                }}
            </main>
        </div>
    }
}