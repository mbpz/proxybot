use crate::i18n::t;
use yew::prelude::*;

#[function_component(DevicesTab)]
pub fn devices_tab() -> Html {
    html! {
        <div class="devices-tab">
            <h2>{t("devices")}</h2>
            <div class="devices-list">
                <p>{t("devices_table_placeholder")}</p>
            </div>
        </div>
    }
}
