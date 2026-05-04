use yew::prelude::*;

#[function_component(GenTab)]
pub fn gen_tab() -> Html {
    html! {
        <div class="gen-tab">
            <h2>{"Gen"}</h2>
            <div class="gen-options">
                <button>{"Mock API"}</button>
                <button>{"Frontend Scaffold"}</button>
                <button>{"Docker Bundle"}</button>
            </div>
            <div class="gen-output">
                <p>{"Generator output"}</p>
            </div>
            <button>{"Open Output Folder"}</button>
        </div>
    }
}