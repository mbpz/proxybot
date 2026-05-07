use crate::i18n::t;
use yew::prelude::*;

#[function_component(AlertsTab)]
pub fn alerts_tab() -> Html {
    html! {
        <div class="alerts-tab">
            <h2>{t("alerts")}</h2>
            <div class="alerts-list">
                <p>{t("alert_table_placeholder")}</p>
            </div>
            <div class="alerts-actions">
                <button>{t("clear_acknowledged")}</button>
            </div>
        </div>
    }
}
