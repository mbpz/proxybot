use yew::prelude::*;
use serde::{Deserialize, Serialize};
use super::types::Request;

#[derive(Properties, PartialEq)]
pub struct TrafficProps {}

#[function_component(TrafficTab)]
pub fn traffic_tab() -> Html {
    let requests = use_state(Vec::<Request>::new);
    let filter_text = use_state(String::new);
    let selected_request = use_state(Option::<Request>::new);

    // Filter requests based on filter_text
    let filtered_requests = {
        let requests = requests.clone();
        let filter = (*filter_text).clone();
        move || {
            if filter.is_empty() {
                requests.clone()
            } else {
                requests.iter().filter(|r| {
                    r.host.contains(&filter) || r.path.contains(&filter)
                }).cloned().collect()
            }
        }
    };

    html! {
        <div class="traffic-tab">
            <div class="filter-bar">
                <input
                    type="text"
                    placeholder="Filter by host or path..."
                    value={(*filter_text).clone()}
                    oninput={Callback::from(move |e: InputEvent| {
                        if let Some(target) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                            filter_text.set(target.value());
                        }
                    })}
                />
            </div>
            <div class="request-list">
                { for filtered_requests().iter().map(|req| {
                    let req_clone = req.clone();
                    html! {
                        <div class="request-item" onclick={Callback::from(move |_| {})}>
                            <span class="method">{ &req.method }</span>
                            <span class="host">{ &req.host }</span>
                            <span class="path">{ &req.path }</span>
                            <span class="status">{ req.status }</span>
                        </div>
                    }
                })}
            </div>
        </div>
    }
}