use leptos::prelude::*;
use leptos::form::ActionForm;

use crate::server::api::{Login, Signup};

#[component]
pub fn LoginPage() -> impl IntoView {
    let (is_signup, set_is_signup) = signal(false);
    let (error_msg, set_error_msg) = signal(String::new());

    let login_action = ServerAction::<Login>::new();
    let signup_action = ServerAction::<Signup>::new();

    let navigate = leptos_router::hooks::use_navigate();
    let navigate2 = navigate.clone();

    // Watch login result
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

    // Watch signup result
    Effect::new(move || {
        if let Some(result) = signup_action.value().get() {
            match result {
                Ok(_user) => {
                    navigate2("/sessions", Default::default());
                }
                Err(e) => set_error_msg.set(e.to_string()),
            }
        }
    });

    view! {
        <div class="login-page">
            <h1>{move || if is_signup.get() { "Sign Up" } else { "Log In" }}</h1>

            <Show when=move || !error_msg.get().is_empty()>
                <p class="error">{move || error_msg.get()}</p>
            </Show>

            <Show
                when=move || !is_signup.get()
                fallback=move || {
                    view! {
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
                    }
                }
            >
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
            </Show>

            <p>
                <a
                    href="#"
                    on:click=move |e| {
                        e.prevent_default();
                        set_is_signup.update(|v| *v = !*v);
                        set_error_msg.set(String::new());
                    }
                >
                    {move || {
                        if is_signup.get() {
                            "Already have an account? Log in"
                        } else {
                            "Need an account? Sign up"
                        }
                    }}
                </a>
            </p>
        </div>
    }
}
