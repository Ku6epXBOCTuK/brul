# Замеры

Снимок одной машины, а не критерий приёмки: сравнивать можно только прогоны на
той же машине и toolchain. Сценарии и код — в
`crates/brul-reactive/benches/reactive.rs`; контракт ядра — в
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

## Как обновлять

Новый снимок заменяет старый, историю хранит git. Без даты, CPU, ОС, toolchain и
команды числа несравнимы.
