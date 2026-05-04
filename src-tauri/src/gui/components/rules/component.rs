use yew::prelude::*;

#[function_component(RulesTab)]
pub fn rules_tab() -> Html {
    html! {
        <div class="rules-tab">
            <h2>{"Rules"}</h2>
            <div class="rules-list">
                <p>{"Rules management - add/edit/delete rules"}</p>
            </div>
            <div class="rules-actions">
                <button>{"Add Rule"}</button>
            </div>
        </div>
    }
}