# Kit

Стандартные компоненты BRUL: `Button`, `Card`, `Tooltip`, `Checkbox`.

Каждый файл самодостаточен: свои `theme dark` / `theme light` и классы. Темы
между файлами не наследуются, поэтому приложение должно объявить те же имена тем
в своём `<style>`.

```brul
<use>
    import Button from "../../../kit/button.brul";
    import Card from "../../../kit/card.brul";
    import Tooltip from "../../../kit/tooltip.brul";
    import Checkbox from "../../../kit/checkbox.brul";
</use>
```

```brul
Button (label: "Открыть", variant: "primary", on clicked: open) { }
Card {
    @slot(header) { Text (title) { "Заголовок" } }
    Text { "Тело карточки" }
    @slot(footer) { Button (label: "Закрыть", variant: "ghost") { } }
}
Tooltip (text: "Подсказка") { Text { "Наведи" } }
Checkbox (label: "Запомнить", checked: always) { }
```

`variant` у `Button`: `primary`, `secondary` (по умолчанию), `ghost`. `Tooltip`
сейчас показывает пузырь в потоке, а не поверх содержимого.
