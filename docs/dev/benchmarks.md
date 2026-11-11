# Замеры

Снимок одной машины, а не критерий приёмки: сравнивать можно только прогоны на
той же машине и toolchain. Сценарии и код — в
`crates/brul-reactive/benches/reactive.rs`, `benches/each.rs` и
`benches/read.rs`; контракт ядра — в
[architecture.md](architecture.md#реактивное-ядро).

## Снимок 2026-11-07

- CPU: Intel Core i7-8700K, 6 ядер / 12 потоков.
- ОС: Windows 10 Home, 10.0.19045, x86_64.
- Toolchain: `nightly-2026-10-15`, rustc 1.101.0-nightly (7df81381d 2026-10-14),
  cargo 1.101.0-nightly.
- Команда: `cargo bench -p brul-reactive`.
- Режим Criterion: 50 сэмплов, прогрев 1 с, измерение 2 с.

| Сценарий         | Размер |   Время |
| ---------------- | -----: | ------: |
| chain            |     10 | 2.10 µs |
| chain            |    100 | 18.7 µs |
| chain            |   1000 |  182 µs |
| diamond          |      — | 0.99 µs |
| many_subscribers |     10 | 1.47 µs |
| many_subscribers |    100 | 18.7 µs |
| many_subscribers |   1000 |  577 µs |
| batched_writes   |     10 | 2.73 µs |
| batched_writes   |    100 | 16.5 µs |

Время — центральная оценка Criterion; разброс и выбросы остаются в отчёте
прогона. `chain` — цепочка memo от одного сигнала до эффекта, `diamond` — два
memo из одного сигнала и общий join, `many_subscribers` — столько эффектов на
одном сигнале, `batched_writes` — батч из стольких записей и один эффект,
читающий их все.

## Снимок 2026-11-11 — чтение переменной и пропса

- CPU: Intel Core i7-8700K, 6 ядер / 12 потоков.
- ОС: Windows 10 Home, 10.0.19045, x86_64.
- Toolchain: `nightly-2026-10-15`, rustc 1.101.0-nightly (7df81381d 2026-10-14).
- Команда: `cargo bench -p brul --bench read`.
- Режим Criterion: 50 сэмплов, прогрев 1 с, измерение 3 с.

| Сценарий                  |   Время |
| ------------------------- | ------: |
| read/property_string      |  111 ns |
| read/property_string_id   | 94.1 ns |
| read/property_scalar      | 36.0 ns |
| read/property_scalar_id   | 23.6 ns |
| read/variable_in_frame    | 23.6 ns |
| read/variable_in_frame_id | 14.7 ns |
| binding/props_read        | 1.19 µs |
| binding/props_read_id     | 1.09 µs |
| binding/frame_only        |  777 ns |

`read/property_string` и `read/property_scalar` — чтение пропса без переменной в
окружении, `read/variable_in_frame` — переменная в кадре биндинга,
`binding/props_read` — кадр из двух переменных и три чтения пропсов внутри него,
`binding/frame_only` — только кадр, контроль шума. Строки `_id` читают по
`VariableId` — так делает сгенерированный код; строки без суффикса идут через
имя и платят за поиск в таблице имён. До BRUL-222 чтение пропса стоило 203 ns и
103 ns: промах в окружении собирал `Error::Undefined` с копией имени. До
BRUL-227 эти же сценарии стоили 105 ns, 28.9 ns, 17.2 ns, 1.25 µs и 892 ns, но
искали переменную сравнением строк и выделяли `String` на каждый `declare`. До
BRUL-229 пропс искался сравнением строк и `PropBinding` держал `String`:
`read/property_scalar_id` стоил 28.4 ns, `read/property_string_id` — 106 ns.
`binding/props_read_id` от этого почти не изменился: время кадра из двух
переменных перекрывает чтение пропсов.

## Как обновлять

Новый снимок заменяет старый, историю хранит git. Без даты, CPU, ОС, toolchain и
команды числа несравнимы.
