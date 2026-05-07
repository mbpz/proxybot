use crate::i18n::t;
use yew::prelude::*;

#[function_component(ReplayTab)]
pub fn replay_tab() -> Html {
    html! {
        <div class="replay-tab">
            <h2>{t("replay")}</h2>
            <div class="replay-list">
                <p>{t("replay_targets")}</p>
            </div>
            <div class="replay-actions">
                <button>{t("export_har")}</button>
                <button>{t("show_diff")}</button>
            </div>
        </div>
    }
}
