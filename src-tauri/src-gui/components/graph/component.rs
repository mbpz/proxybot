use crate::i18n::t;
use yew::prelude::*;

#[function_component(GraphTab)]
pub fn graph_tab() -> Html {
    html! {
        <div class="graph-tab">
            <h2>{t("graph")}</h2>
            <div class="graph-controls">
                <button>{t("dag_view")}</button>
                <button>{t("auth_state")}</button>
                <button>{t("refresh")}</button>
            </div>
            <div class="graph-display">
                <p>{t("graph_placeholder")}</p>
            </div>
        </div>
    }
}
