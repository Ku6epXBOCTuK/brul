use brul_display_test::{DisplayScene, run};

macro_rules! display_cases {
    ($($name:ident),+ $(,)?) => {
        $(
            mod $name {
                include!(concat!("fixtures/display/", stringify!($name), "/scene.rs"));
            }

            #[test]
            fn $name() {
                run::<$name::Scene>(
                    stringify!($name),
                    include_str!(concat!("fixtures/display/", stringify!($name), "/meta.txt")),
                    include_bytes!(concat!("fixtures/display/", stringify!($name), "/expected.png")),
                    concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/display/", stringify!($name), "/expected.png"),
                );
            }
        )+
    };
}

display_cases! {
    button_states,
    card_shadow_clip,
    canvas_bars,
    input_caret,
}
