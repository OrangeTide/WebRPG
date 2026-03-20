use leptos::form::ActionForm;
use leptos::prelude::*;

use crate::server::api::{Login, Signup};

#[component]
pub fn LoginPage() -> impl IntoView {
    let (error_msg, set_error_msg) = signal(String::new());
    let login_action = ServerAction::<Login>::new();
    let navigate = leptos_router::hooks::use_navigate();

    Effect::new(move || {
        if let Some(result) = login_action.value().get() {
            match result {
                Ok(_user) => {
                    navigate("/sessions", Default::default());
                }
                Err(e) => set_error_msg.set(e.to_string()),
            }
        }
    });

    view! {
        <div class="login-page">
            <h1>"Log In"</h1>

            <Show when=move || !error_msg.get().is_empty()>
                <p class="error">{move || error_msg.get()}</p>
            </Show>

            <ActionForm action=login_action>
                <div>
                    <label for="username">"Username"</label>
                    <input type="text" name="username" required />
                </div>
                <div>
                    <label for="password">"Password"</label>
                    <input type="password" name="password" required />
                </div>
                <button type="submit">"Log In"</button>
            </ActionForm>

            <p>
                <a href="/signup">"Need an account? Sign up"</a>
            </p>
        </div>
    }
}

#[component]
pub fn SignupPage() -> impl IntoView {
    let (error_msg, set_error_msg) = signal(String::new());
    let signup_action = ServerAction::<Signup>::new();
    let navigate = leptos_router::hooks::use_navigate();

    Effect::new(move || {
        if let Some(result) = signup_action.value().get() {
            match result {
                Ok(_user) => {
                    navigate("/sessions", Default::default());
                }
                Err(e) => set_error_msg.set(e.to_string()),
            }
        }
    });

    view! {
        <div class="login-page">
            <h1>"Sign Up"</h1>

            <Show when=move || !error_msg.get().is_empty()>
                <p class="error">{move || error_msg.get()}</p>
            </Show>

            <ActionForm action=signup_action>
                <div>
                    <label for="username">"Username"</label>
                    <input type="text" name="username" required />
                </div>
                <div>
                    <label for="display_name">"Display Name"</label>
                    <input type="text" name="display_name" required />
                </div>
                <div>
                    <label for="email">"Email"</label>
                    <input type="email" name="email" required />
                </div>
                <div>
                    <label for="password">"Password"</label>
                    <input type="password" name="password" required minlength="8" />
                </div>
                <button type="submit">"Sign Up"</button>
            </ActionForm>

            <p>
                <a href="/login">"Already have an account? Log in"</a>
            </p>
        </div>
    }
}
