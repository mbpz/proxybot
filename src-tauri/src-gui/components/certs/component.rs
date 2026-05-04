use yew::prelude::*;

#[function_component(CertsTab)]
pub fn certs_tab() -> Html {
    html! {
        <div class="certs-tab">
            <h2>{"Certificates"}</h2>
            <div class="certs-list">
                <p>{"Certificate management - view installed CA, generated leaf certs"}</p>
            </div>
        </div>
    }
}