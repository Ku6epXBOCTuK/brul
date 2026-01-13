use std::thread::panicking;

use brul::State;
use brul_gui::App;
use tokio::time::Instant;

struct AppState {
    start_time: Instant,
}
struct MyString(String);

fn change_background_color(app_handle: AppHandle) {
    let now = Instant::now();
    let app_state: AppState = app_handle.state::<AppState>();
    let time = now.duration_since(app_state.start_time).as_secs_f32();

    let color = Color::rgb(
        (time.sin() * 0.5 + 0.5) as f32,
        ((time + 2.0).sin() * 0.5 + 0.5) as f32,
        ((time + 4.0).sin() * 0.5 + 0.5) as f32,
    );

    app_handle.set_background_color(color);
}

#[brul::command]
fn log_app_state(state: State<AppState>) {
    dbg!(state);
}

#[brul::command]
fn log_string_state(state: State<MyString>) {
    dbg!(state);
}

#[brul::widget]
fn my_button(emit: brul::Emit) -> Widget {
    Widget {
        r#type: "button",
        text: "click me",
        click: move || {
            emit("log_app_state");
            emit("log_string_state");
        },
    }
}

#[tokio::main]
async fn main() {
    brul::Builder::default()
        .setup(|app| {
            app.manage(MyString("BRUL".into()));

            let string = app.state::<MyString>();
            println!("String stored in state: {}", string.0);
        })
        .manage(AppState {
            start_time: Instant::now(),
        })
        .add_task(change_background_color)
        .add_widgets([my_button])
        .invoke_handlers(brul::generate_handlers![log_app_state, log_string_state])
        .run()
        .expect("Error while running brul application");
}
