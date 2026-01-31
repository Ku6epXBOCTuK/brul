use brul::{AppHandle, AppManager, State, util::Color};
use tokio::time::Instant;

#[derive(Debug)]
struct AppState {
    start_time: Instant,
}

#[derive(Debug)]
struct MyString(String);

fn change_background_color(app_handle: &AppHandle) {
    // let now = Instant::now();
    // let app_state = app_handle.state::<AppState>();
    // let time = now.duration_since(app_state.start_time).as_secs_f32();

    // let color = Color::rgb(
    //     (time.sin() * 0.5 + 0.5) as f32,
    //     ((time + 2.0).sin() * 0.5 + 0.5) as f32,
    //     ((time + 4.0).sin() * 0.5 + 0.5) as f32,
    // );

    // app_handle.set_background_color(color);
    tracing::info!("Changing background color");
}

#[brul::command]
fn log_app_state(state: State<AppState>) {
    dbg!(state);
}

#[brul::command]
fn log_string_state(state: State<MyString>) {
    dbg!(state);
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter("brul=debug,basic_app=debug")
        .init();
    brul::AppBuilder::new()
        .setup(|app| {
            app.manage(MyString("BRUL".into()));

            let string = app.state::<MyString>();
            tracing::debug!("String stored in state: {}", string.0);
        })
        .manage(AppState {
            start_time: Instant::now(),
        })
        .add_task(change_background_color)
        // .add_handlers(brul::generate_handlers![log_app_state, log_string_state])
        .run()
        .expect("Error while running brul application");
}
