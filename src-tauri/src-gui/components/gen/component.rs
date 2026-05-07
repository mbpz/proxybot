use crate::i18n::t;
use yew::prelude::*;

#[function_component(GenTab)]
pub fn gen_tab() -> Html {
    html! {
        <div class="gen-tab">
            <h2>{t("gen")}</h2>
            <div class="gen-options">
                <button>{t("mock_api")}</button>
                <button>{t("frontend_scaffold")}</button>
                <button>{t("docker_bundle")}</button>
            </div>
            <div class="gen-output">
                <p>{t("generator_output")}</p>
            </div>
            <button>{t("open_output_folder")}</button>
        </div>
    }
}
