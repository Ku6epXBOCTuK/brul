use std::hint::black_box;

use brul_gui::{TextEngine, TextStyle};
use criterion::{BatchSize, Criterion};

const SCALES: &[(&str, f32)] = &[
    ("1.00", 1.0),
    ("1.25", 1.25),
    ("1.50", 1.50),
    ("2.00", 2.00),
];

#[allow(clippy::non_ascii_literal)]
const STRINGS: &[&str] = &[
    "nobs",
    "Отправить",
    "Пресеты",
    "AFK: нет активности",
    "Скопировано в OBS",
    "bropicker",
    "Папки проекта",
    "Выбрать",
    "Настройки",
    "xboct-panel",
    "Службы остановлены",
    "Перезапустить",
    "Sublime Text",
    "Ресурсы",
    "script.brul",
    "theme",
];

const SIZES: &[f32] = &[14.0, 16.0, 18.0, 24.0];

fn lay_out(engine: &mut TextEngine, scale: f32) {
    for text in STRINGS {
        for size in SIZES {
            black_box(
                engine
                    .layout(text, &TextStyle::new(*size), None, scale)
                    .expect("bench text is valid"),
            );
        }
    }
}

fn warm_engine(scale: f32) -> TextEngine {
    let mut engine = TextEngine::new();
    lay_out(&mut engine, scale);
    engine
}

fn report_atlas(name: &str, scale: f32) {
    let engine = warm_engine(scale);
    let atlas = engine.atlas();
    let bytes = u64::from(atlas.width()) * u64::from(atlas.height()) * 4;
    println!(
        "{name}: {} entries, {}x{} atlas, {bytes} bytes",
        atlas.len(),
        atlas.width(),
        atlas.height()
    );
}

fn measure(criterion: &mut Criterion) {
    for (name, scale) in SCALES {
        report_atlas(name, *scale);
    }

    let mut cold = criterion.benchmark_group("dpi/text/cold");
    for (name, scale) in SCALES {
        cold.bench_function(*name, |bencher| {
            bencher.iter_batched(
                TextEngine::new,
                |mut engine| lay_out(&mut engine, *scale),
                BatchSize::PerIteration,
            );
        });
    }
    cold.finish();

    let mut warm = criterion.benchmark_group("dpi/text/warm");
    for (name, scale) in SCALES {
        let mut engine = warm_engine(*scale);
        warm.bench_function(*name, |bencher| {
            bencher.iter(|| lay_out(&mut engine, *scale));
        });
    }
    warm.finish();
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    measure(&mut criterion);
    criterion.final_summary();
}
