use leptos::prelude::*;

#[component]
pub fn LandingPage() -> impl IntoView {
    view! {
        <div class="landing">
            <h1>"WebRPG"</h1>
            <p>"A virtual tabletop for roleplaying games."</p>
            <p>"Host sessions with maps, dice rolling, character sheets, chat, and more."</p>
            <div class="landing-actions">
                <a href="/login">"Log In"</a>
                <span>" or "</span>
                <a href="/login?signup=true">"Sign Up"</a>
                <span>" to get started."</span>
            </div>
        </div>
    }
}
