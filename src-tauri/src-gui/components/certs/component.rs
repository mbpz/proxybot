use crate::i18n::t;
use yew::prelude::*;

#[function_component(CertsTab)]
pub fn certs_tab() -> Html {
    html! {
        <div class="certs-tab">
            <h2>{t("certs")}</h2>
            <div class="certs-list">
                <p>{t("cert_management")}</p>
            </div>
        </div>
    }
}
