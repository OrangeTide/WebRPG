use leptos::prelude::*;
use leptos_meta::{MetaTags, Stylesheet, Title, provide_meta_context};
use leptos_router::{
    ParamSegment, StaticSegment,
    components::{Route, Router, Routes},
};

use crate::pages::game::GamePage;
use crate::pages::landing::LandingPage;
use crate::pages::login::LoginPage;
use crate::pages::sessions::SessionsPage;

pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
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
                    <Route path=StaticSegment("sessions") view=SessionsPage />
                    <Route path=(StaticSegment("game"), ParamSegment("id")) view=GamePage />
                </Routes>
            </main>
        </Router>
    }
}
