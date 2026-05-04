use yew::prelude::*;

#[function_component(GraphTab)]
pub fn graph_tab() -> Html {
    html! {
        <div class="graph-tab">
            <h2>{"Graph"}</h2>
            <div class="graph-controls">
                <button>{"DAG View"}</button>
                <button>{"Auth State"}</button>
                <button>{"Refresh"}</button>
            </div>
            <div class="graph-display">
                <p>{"ASCII DAG visualization or auth state machine"}</p>
            </div>
        </div>
    }
}