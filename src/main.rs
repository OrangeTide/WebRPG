use leptos::prelude::*;

#[component]
fn SimpleCounter(
    initial_value: i32,
    step: i32,
    #[prop(default = "somevalue".to_string())]
    name: String,
    ) -> impl IntoView {
    let (value, set_value) = signal(initial_value);

    view! {
        <div>
            <button on:click=move |_| set_value.set(0)>"Clear"</button>
            <button on:click=move |_| set_value.set({initial_value})>"Reset"</button>
            <button on:click=move |_| *set_value.write() -= step>"Down"</button>
            <button on:click=move |_| set_value.update(|value| *value += step)>"Up"</button>
            <span>{name} ":" {value}</span>
        </div>
    }
}

fn main() {
    _ = console_log::init_with_level(log::Level::Debug);
    console_error_panic_hook::set_once();
    mount_to_body(|| {
        view! {
            <p>"Hello, world!"</p>
            <SimpleCounter initial_value=7 step=1 name="potato".to_owned()/>
        }
    })
}
