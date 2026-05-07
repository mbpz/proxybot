use crate::i18n::t;
use yew::prelude::*;

#[function_component(RulesTab)]
pub fn rules_tab() -> Html {
    html! {
        <div class="rules-tab">
            <h2>{t("rules")}</h2>
            <div class="rules-list">
                <p>{t("rules_management")}</p>
            </div>
            <div class="rules-actions">
                <button>{t("add_rule")}</button>
            </div>
        </div>
    }
}
