use yew::prelude::*;
use super::types::Request;

#[derive(Properties, PartialEq)]
pub struct TrafficProps {}

#[function_component(TrafficTab)]
pub fn traffic_tab() -> Html {
    let requests = use_state(Vec::<Request>::new);
    let filter_text = use_state(String::new);
    let selected_request = use_state(|| None::<Request>);

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
                <p>{"Request list - capture traffic to see requests here"}</p>
            </div>
        </div>
    }
}