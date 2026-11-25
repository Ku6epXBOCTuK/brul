# Kit

Стандартные компоненты BRUL: `Button`, `Card`, `Tooltip`, `Checkbox`, `Dialog`,
`Input`.

Каждый файл несёт только классы, а палитру объявляет приложение в `<style>`
корневого окна: компонент наследует её и берёт оттуда `theme.surface`,
`theme.accent` и остальные ключи. Своих `theme`-блоков в файлах кита нет,
поэтому неизвестный ключ — ошибка сборки с файлом и позицией.

```brul
<use>
    import Button from "brul:button";
    import Card from "brul:card";
    import Tooltip from "brul:tooltip";
    import Checkbox from "brul:checkbox";
    import Dialog from "brul:dialog";
    import Input from "brul:input";
</use>
```

`brul:` — имя файла кита, а не путь: исходник зашит в `brul-codegen` из каталога
`kit/` этого репозитория.

```brul
Button (label: "Открыть", variant: "primary", on clicked: open) { }
Card {
    @slot(header) { Text (title) { "Заголовок" } }
    Text { "Тело карточки" }
    @slot(footer) { Button (label: "Закрыть", variant: "ghost") { } }
}
Tooltip (text: "Подсказка") { Text { "Наведи" } }
Checkbox (label: "Запомнить", checked: always) { }
Dialog (open: shown, title: "Правка", on closed: finish) {
    Input (value: draft) { }
}
```

`variant` у `Button`: `primary`, `secondary` (по умолчанию), `ghost`. `Tooltip`
сейчас показывает пузырь в потоке, а не поверх содержимого. `Input` —
однострочный компонент над примитивом `TextInput`: `value: String` задаёт текст,
события `input(ev)` и `key(ev)` пробрасывают события поля; слотов и своей логики
ввода у него нет — клавиши обрабатывают скрытые window-политики.

`Dialog` живёт на модальном `Layer`: тело — слот, внизу свои «Отмена» и
подтверждение (`confirm`, по умолчанию «Сохранить»), клик по затемнению и Escape
закрывают его. Видимостью владеет вызывающий: `open` — его сигнал, а
`closed(result: Bool)` сообщает `true` только при подтверждении (обработчик
обычно пишет `open = false`). Пока диалог открыт, Tab не выходит из него.

Классы кита переопределяются из приложения: блок
`override <Имя> { .класс { … } }` в `<style>` корневого окна дописывает правила
после правил компонента, поэтому выигрывает более позднее объявление. `Button`,
`Card`, `Tooltip`, `Checkbox` и `Input` — кит, а не жёсткая тема: `.button`,
`.button-label`, состояния и геометрия доступны снаружи.
