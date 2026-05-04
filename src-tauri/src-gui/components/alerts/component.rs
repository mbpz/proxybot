use yew::prelude::*;

#[function_component(AlertsTab)]
pub fn alerts_tab() -> Html {
    html! {
        <div class="alerts-tab">
            <h2>{"Alerts"}</h2>
            <div class="alerts-list">
                <p>{"Alert table - SEV1/2/3, source, description, ACK/Clear"}</p>
            </div>
            <div class="alerts-actions">
                <button>{"Clear Acknowledged"}</button>
            </div>
        </div>
    }
}