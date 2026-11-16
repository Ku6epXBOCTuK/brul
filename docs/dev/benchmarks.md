# Замеры

Снимок одной машины, а не критерий приёмки: сравнивать можно только прогоны на
той же машине и toolchain. Сценарии и код — в
`crates/brul-reactive/benches/reactive.rs`, `benches/each.rs` и
`benches/read.rs`; контракт ядра — в
[architecture.md](architecture.md#реактивное-ядро).

Все снимки ниже сняты на одной машине:

- CPU: Intel Core i7-8700K, 6 ядер / 12 потоков.
- ОС: Windows 10 Home, 10.0.19045, x86_64.
- Toolchain: `nightly-2026-10-15`, rustc 1.101.0-nightly (7df81381d 2026-10-14),
  cargo 1.101.0-nightly.
- Режим Criterion: 50 сэмплов, прогрев 1 с, измерение 2 с для `brul-reactive` и
  3 с для `brul`.

Время — центральная оценка Criterion; разброс и выбросы остаются в отчёте
прогона.

## 2026-11-07 — реактивное ядро

`cargo bench -p brul-reactive`

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

`chain` — цепочка memo от одного сигнала до эффекта, `diamond` — два memo из
одного сигнала и общий join, `many_subscribers` — столько эффектов на одном
сигнале, `batched_writes` — батч из стольких записей и один эффект, читающий их
все.

## 2026-11-11 — чтение переменной и пропса, построение строки `each`

`cargo bench -p brul --bench read`

| Сценарий                  |   Время |
| ------------------------- | ------: |
| read/property_string      | 31.2 ns |
| read/property_string_id   | 19.6 ns |
| read/property_scalar      | 31.4 ns |
| read/property_scalar_id   | 19.2 ns |
| read/variable_in_frame    | 22.0 ns |
| read/variable_in_frame_id | 10.3 ns |
| binding/props_read        |  100 ns |
| binding/props_read_id     | 69.1 ns |
| binding/frame_only        | 11.3 ns |

`read/property_string` и `read/property_scalar` — чтение пропса без переменной в
окружении, `read/variable_in_frame` — переменная в кадре биндинга,
`binding/props_read` — кадр из двух переменных и три чтения пропсов внутри него,
`binding/frame_only` — только кадр, контроль шума. Строки `_id` читают по
`VariableId` — так делает сгенерированный код; строки без суффикса идут через
имя и платят за поиск в таблице имён.

`cargo bench -p brul --bench each`

| Сценарий                   |   Время |
| -------------------------- | ------: |
| each_row/scopes_6_bindings |  986 ns |
| each_row/widgets           |  693 ns |
| build_scopes/depth_1       | 8.98 ns |
| build_scopes/depth_3       | 9.41 ns |
| apply_row/declares         | 10.7 ns |

`each_row/scopes_6_bindings` — уровень `each` с двумя переменными, шесть взятий
снимка scope и снятие уровня, `each_row/widgets` — построение виджетов строки
(контроль шума), `apply_row/declares` — кадр биндинга из готового снимка.

Что дал каждый шаг:

- BRUL-222 — промах в окружении больше не собирает `Error::Undefined` с копией
  имени: `read/property_string` 203 → 105 ns, `read/property_scalar` 103 → 28.9
  ns, `binding/props_read` 1.52 → 1.25 µs.
- BRUL-227 — переменные хранятся по `VariableId`, без `String` на `declare` и
  без сравнения имён на чтении: `read/variable_in_frame_id` 17.2 → 14.0 ns.
- BRUL-229 — пропс ищется по `VariableId`, `PropBinding` больше не держит
  `String`: `read/property_scalar_id` 28.4 → 23.6 ns, `read/property_string_id`
  106 → 94.1 ns.
- BRUL-223 — кадр биндинга ссылается на снимок scope вместо копии значений:
  `binding/frame_only` 777 → 12.2 ns, `binding/props_read_id` 1.09 µs → 328 ns.
- BRUL-231 — строки, массивы и объекты в `Value` лежат под `Rc`, клон стал
  счётчиком ссылок: `read/property_string_id` 94.1 → 19.6 ns,
  `binding/props_read_id` 328 → 69.1 ns.
- BRUL-230 — кадр сборки и снимок scope делят один `Rc<[Variable]>` вместо двух
  копий: `each_row/scopes_6_bindings` 1.09 → 0.99 µs, `apply_row/declares` 11.2
  → 10.7 ns.

## Счётчик аллокаций в приложении (BRUL-232)

Счётчик включается на сборке приложения и живёт в сгенерированном модуле:

```powershell
$env:BRUL_ALLOC_COUNT = "1"
cargo build -p bropicker --release
```

Без переменной блок вырезан на `#[cfg(brul_alloc_count)]`. С включённой
переменной `alloc` и `realloc` считаются раздельно, `dealloc` только
пробрасывается, а в байты идёт запрошенный размер каждого вызова. Счётчики
обнуляются после первого кадра: инициализация окна и шрифтов не смешивается с
интерфейсом. Итог уходит в лог приложения (`~/.config/brul/<exe>.log`) строкой
такого вида:

```text
brul allocations: init <allocs> allocs, <reallocs> reallocs, <bytes> bytes; ui ...
```

`init` — старт до первого кадра, `ui` — всё после него. Строка печатается только
при выходе через цикл событий: убитый процесс отчёта не даёт.

## 2026-11-16 — bropicker: старт и закрытие (BRUL-232)

`$env:BRUL_ALLOC_COUNT = "1"; cargo build -p bropicker`, запуск окна и выход по
Esc, сборка debug.

| Фаза | allocs | reallocs |    bytes |
| ---- | -----: | -------: | -------: |
| init |  14462 |     3197 | 10187995 |
| ui   |    820 |       14 |   122312 |

`init` — окно, шрифты, GPU и конфиг до первого кадра, `ui` — построение и
обновление интерфейса после него. `dealloc` не считается, байты — сумма
запрошенных размеров, а не живая память.

## 2026-11-16 — note-obs: старт и закрытие (BRUL-232)

`$env:BRUL_ALLOC_COUNT = "1"; cargo build -p note-obs`, запуск окна, выход через
`nobs --quit`, сборка debug.

| Фаза | allocs | reallocs |   bytes |
| ---- | -----: | -------: | ------: |
| init |   9900 |     2210 | 8093519 |
| ui   |   1076 |       44 |  173252 |

## 2026-11-16 — xboct-panel: старт и закрытие (BRUL-232)

`$env:BRUL_ALLOC_COUNT = "1"; cargo build -p xboct-panel`, запуск окна и выход
через `xboct-panel --quit`, сборка debug.

| Фаза | allocs | reallocs |    bytes |
| ---- | -----: | -------: | -------: |
| init |  17193 |     2783 | 11432538 |
| ui   |    733 |       13 |    56149 |

## 2026-11-16 — холодный старт (BRUL-196)

`cargo build --release -p bropicker -p note-obs -p xboct-panel`, затем по пять
запусков каждого приложения с `$env:BRUL_STARTUP_LOG = "1"`: процесс поднимался,
через 2.5 с снимался, из `~/.config/brul/<exe>.log` бралась строка
`brul startup: first frame in ...`.

| Приложение  | Медиана |    Разброс | Масштаб | Поверхность |
| ----------- | ------: | ---------: | ------: | ----------: |
| bropicker   |  757 мс | 689–785 мс |    1.00 |     488x800 |
| note-obs    |  682 мс | 666–764 мс |    1.00 |     520x480 |
| xboct-panel |  720 мс | 652–785 мс |    1.00 |     900x560 |

Замер идёт от входа в цикл событий (`run_windows`) до первого показанного кадра:
в него входят создание окна, GPU-контекст, шрифты и построение дерева. Загрузка
exe и инициализация лога в число не входят, поэтому к «времени от клика» надо
добавить запуск процесса. Строку печатает только `BRUL_STARTUP_LOG`, по
умолчанию её нет; в сборке под администратором (xboct-panel) переменную нужно
ставить в профиле пользователя — повышенный процесс не наследует окружение
вызывающего.

## 2026-11-16 — ошибка в конфиге (BRUL-196)

Во всех трёх приложениях файл конфига заменялся на заведомо битый TOML
(`unclosed array`), приложение запускалось и снимался код возврата и хвост лога.

| Приложение  | Код возврата | Что видит пользователь     |
| ----------- | -----------: | -------------------------- |
| bropicker   |            0 | ничего, окно не появляется |
| note-obs    |            1 | ничего, окно не появляется |
| xboct-panel |            1 | ничего, окно не появляется |

Разбор и номер строки пишутся в `~/.config/brul/<exe>.log` с уровнем `ERROR`, но
из окна запуска этого не видно: процесса, консоли и диалога нет. Отдельно стоит
код 0 у bropicker — неудачный конфиг выглядит как успешный запуск.

## Как обновлять

Новый снимок заменяет старый, историю хранит git. Прогон на другой машине или
toolchain открывает свой раздел с собственной шапкой: числа из разных разделов
сравнивать нельзя.
