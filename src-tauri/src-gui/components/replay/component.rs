use yew::prelude::*;

#[function_component(ReplayTab)]
pub fn replay_tab() -> Html {
    html! {
        <div class="replay-tab">
            <h2>{"Replay"}</h2>
            <div class="replay-list">
                <p>{"Replay targets - start/stop, HAR export, diff view"}</p>
            </div>
            <div class="replay-actions">
                <button>{"Export HAR"}</button>
                <button>{"Show Diff"}</button>
            </div>
        </div>
    }
}