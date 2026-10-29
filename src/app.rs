use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    ParamSegment, StaticSegment,
    components::{Route, Router, Routes},
};

use crate::components::theme_toggle::ThemeToggle;
use crate::pages::admin::AdminPage;
use crate::pages::game::GamePage;
use crate::pages::landing::LandingPage;
use crate::pages::login::{LoginPage, SignupPage};
use crate::pages::sessions::SessionsPage;

/// Sets `data-theme` on the document root before first paint, so there is no
/// flash of the wrong theme. Uses the stored preference, falling back to the
/// OS color scheme. Kept tiny and dependency-free since it runs inline in the
/// head. The CSS prefers-color-scheme block is the fallback when JS is off.
const THEME_INIT_SCRIPT: &str = "(function(){var t;try{t=localStorage.getItem('webrpg_theme');}catch(e){}if(t!=='light'&&t!=='dark'){t=(window.matchMedia&&window.matchMedia('(prefers-color-scheme: light)').matches)?'light':'dark';}document.documentElement.setAttribute('data-theme',t);})();";

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="icon" href="/favicon.ico" />
                <script inner_html=THEME_INIT_SCRIPT></script>
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet href="/pkg/webrpg.css" />
        <Title text="WebRPG" />

        <Router>
            <main>
                <Routes fallback=|| "Page not found.".into_view()>
                    <Route path=StaticSegment("") view=LandingPage />
                    <Route path=StaticSegment("login") view=LoginPage />
                    <Route path=StaticSegment("signup") view=SignupPage />
                    <Route path=StaticSegment("sessions") view=SessionsPage />
                    <Route path=StaticSegment("admin") view=AdminPage />
                    <Route path=(StaticSegment("game"), ParamSegment("id")) view=GamePage />
                </Routes>
            </main>
            <ThemeToggle />
        </Router>
    }
}
