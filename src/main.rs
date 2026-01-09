use brul_gui::start_app;

const FOO: f32 = 1.0;

fn main() {
    start_app();

    let a: f32 = 1.0;

    let _b = match a {
        FOO => 1,
        _ => 0,
    };
}
