# custom_text_input

Пример `CustomTextInput` на `.brul` + `#[brul::attach]`: приложение меняет
политику ввода, не трогая ядро BRUL.

## Запуск

```sh
cargo run -p custom_text_input
```

## Что внутри

| Файл                    | Что делает                                                                   |
| ----------------------- | ---------------------------------------------------------------------------- |
| `src/window.brul`       | окно: состояние приложения, кнопки режимов, вывод сырого значения и каретки  |
| `src/custom_input.brul` | компонент `CustomTextInput`: пропсы `value`/`mode`, события `input`/`escape` |
| `src/main.rs`           | политики `masked_editor` и `custom_keys`, запуск и тесты                     |
| `build.rs`              | `brul::build::app("src/window.brul")`                                        |

`masked_editor` держит сырой текст в состоянии capability и сам записывает в
буфер только результат: маску, обратный порядок или хеш. Сырое значение уходит в
состояние приложения через `Event::input`, каретка переводится между raw и
display в символах. `custom_keys` на том же фокусируемом элементе перехватывает
`Tab` и `Escape`, подменяя window-политику `keyboard`.

## Тесты

```sh
cargo test -p custom_text_input
```
