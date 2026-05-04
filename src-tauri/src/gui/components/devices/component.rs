use yew::prelude::*;

#[function_component(DevicesTab)]
pub fn devices_tab() -> Html {
    html! {
        <div class="devices-tab">
            <h2>{"Devices"}</h2>
            <div class="devices-list">
                <p>{"Devices table - MAC, last seen, bytes up/down, app, rule"}</p>
            </div>
        </div>
    }
}