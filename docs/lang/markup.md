# Язык разметки

Статус: stable. Канон языка: парсер и кодеген проверяются на этой спеке, примеры
в конце — фикстуры тестов.

Принципы — [architecture.md §Решения](../dev/architecture.md#решения).

## Идея

Скобочный DSL вместо HTML-тегов. Элемент — имя, скобки с классами и параметрами,
фигурные скобки с содержимым:

```brul
View (item-panel, selected: selected, on click: panelClicked) {
    Text (text-main) { name }
}
```

- имя — встроенный виджет или компонент;
- в скобках: голые идентификаторы = **классы стилей**, `имя: выражение` =
  **параметры**, `on событие: функция` = **обработчик** (только по ссылке, без
  инлайн-тел);
- в фигурных скобках: структура — дочерние элементы, `if`/`each`; у листовых
  виджетов (`Text`) — одно выражение-содержимое. Блок `component` — известное
  исключение: в нём рядом со структурой объявляются `prop` и `event`.

Внешний вид задаётся классами в стилях ([style.md](style.md)), а не
инлайн-параметрами. Параметры в скобках — только поведение и данные.

## Лексика

- Комментарии: `//` до конца строки. (`/* */` — отложено.)
- Имена встроенных виджетов и компонентов: `PascalCase`.
- Классы: `kebab-case`. Свойства/события: `camelCase`. Ключи тем: `kebab-case`.
- Пробелы и переводы строк незначимы (кроме тел строк-литералов).
- Ключевые слова: `component`, `window`, `prop`, `event`, `on`, `if`, `else`,
  `each`, `in`, `import`, `from`; типы пропов — `Bool`, `Number`, `String`,
  `Image`.
- Ключевые слова не бывают идентификаторами: `on` в скобках начинает обработчик,
  классом или параметром с этим именем быть нельзя.

## Грамматика (EBNF)

Выражения `Expr` — общие со скриптом, см. [script.md](script.md) §«Грамматика».

```ebnf
{{#include markup.ebnf}}
```

Исключение — **листовые виджеты** (`Text` в MVP): их блок — одно выражение
(содержимое), а не список детей. Правило семантическое, не синтаксическое:
парсер знает класс виджета.

```brul
Text (text-main) { name }
Text (badge-text) { "По умолчанию" }
```

## Семантика

### Окно

`window` — корневой компонент окна. Параметры окна — не стили, а атрибуты
оконной системы. `frameless`/`transparent` — булевы флаги в параметрах.

```brul
window Picker (title: "bropicker", width: 488, height: 800,
               frameless: true, transparent: true) {
    // ...
}
```

Встроенные события:

- `on key: fn` (окно) — клавиатура; функция получает событие с полем `code`
  (строка: `"Escape"`, `"Enter"`, …); проверка кода — в скрипте, без
  спец-грамматики:
  `function handleKey(ev) { if (ev.code == "Escape") { exit() } }`
- `on drag: dragWindow` (любой элемент) — начало системного перетаскивания окна;
  `dragWindow` — встроенная функция. Своя функция может добавить условия и
  вызвать `dragWindow()` сама.

### Классы и состояния

Голый идентификатор в скобках — класс стилей. Один элемент — несколько классов:
`View (item-panel, rounded, elevated) { ... }`.

Данные-зависимые состояния — не через смену класса (динамические классы
отложены), а через **встроенные параметры-состояния**, переключающие снапшоты
([лейаут и стили](../dev/architecture.md#лейаут-и-стили)):

| Параметр         | Включаемое состояние | Кто меняет        |
| ---------------- | -------------------- | ----------------- |
| `checked: Expr`  | `:checked`           | логика компонента |
| `selected: Expr` | `:selected`          | логика компонента |
| `disabled: Expr` | `:disabled`          | логика компонента |
| —                | `:hover`, `:pressed` | input-менеджер    |

Стили для состояний — в CSS-подобных правилах ([style.md](style.md)),
компилируются в снапшоты; переключение — замена снапшота, без пересчёта каскада.

### Обработчики

Обработчик — только именованная функция, присоединяется в скобках:

```brul
View (checkbox, checked: checked, on click: toggle) { }
```

Правила:

- имя обязано резолвиться в функцию скрипт-блока этого компонента или во
  встроенную функцию — иначе ошибка компиляции с позицией;
- один обработчик на событие; дубликат — ошибка;
- аргументы события передаются функции параметрами: `event clicked(i: Number);`
  → `function launch(i) { ... }`;
- инлайн-тела (`on click { ... }`) в MVP **запрещены**: поведение живёт в
  скрипте, разметка — только структура и объявления компонента
  ([причина](../dev/architecture.md#ограничения-dsl)).

### Параметры

`имя: выражение` в скобках — по порядку приоритета:

1. параметр-состояние (`checked`/`selected`/`disabled`);
2. встроенная привязка виджета: `visible`, `x`, `y` (абсолютное позиционирование
   внутри родителя — поверх лейаута);
3. свойство компонента (при инстанцировании компонента).

Выражение параметра — реактивная привязка: изменение зависимых сигналов вызывает
точечное обновление
([обновления без VDOM](../dev/architecture.md#обновления-без-vdom)).

### Свойства и события компонента

```brul
prop checked: Bool = false;     // тип обязателен (нужен кодегену для сигнала)
event clicked(i: Number);       // событие с типизированными аргументами
```

- Декларация `prop` или `event` заканчивается `;`; пропуск — ошибка с позицией.
- Объявления стоят в блоке `component` рядом со структурой, порядок не важен.
- Значение по умолчанию обязательно для `Bool`/`Number`/`String`; `Image` — по
  умолчанию пустой (`= null`).
- `prop … : Image` принимает `null` (пустая картинка) или значение `Image`:
  ресурс из `<use>` либо путь, который приходит из данных приложения.
- Присваивание свойству из обработчика — запись в сигнал.
- Вызов события: `clicked(index)` — как вызов функции.

### Файлы и импорты

Один `.brul`-файл — один SFC. Блоки верхнего уровня в тегах: `<markup>`
обязателен и единственный, `<use>`, `<style>` и `<script>` опциональны и тоже
единственны. Порядок блоков свободен, текст вне блоков запрещён. Содержимое
блока принадлежит своему языку, сплиттер его не разбирает.

```brul
<use>
    import BrowserItem from "./browser_item";
    import Footer from "./footer";
</use>

<markup>
    // разметка
</markup>

<style>
    // стили
</style>

<script>
    // скрипт
</script>
```

- `<use>` — зависимости файла: грамматика и семантика — в [use.md](use.md).
- Инстанцирование: `BrowserItem (index: 0, name: "Chrome") { }` — компоненты в
  MVP **без детей** (слоты отложены).

### Управление детьми

```brul
if (isDefault) {
    View (default-badge) { Text (badge-text) { "По умолчанию" } }
} else {
    Text (hint) { "—" }
}

each (browsers as item, idx (item.index)) {
    BrowserItem (index: idx, name: item.name, emoji: item.emoji) { }
}
```

- `if`/`else` — условное включение поддерева.
- `each` — повторение по массиву; заголовок в скобках: коллекция, `as` и имя
  элемента, необязательные имя индекса и выражение ключа.
- Пересоздание или переиспользование элементов при изменении модели — деталь
  компилятора (keyed-reuse отложено; MVP — перестроение диапазона).

## Встроенные виджеты MVP

| Виджет          | Дети      | Назначение                                              |
| --------------- | --------- | ------------------------------------------------------- |
| `View`          | контейнер | универсальный контейнер (панели, кнопки, карточки)      |
| `Row`, `Column` | контейнер | лейаут-строка/столбец (spacing/padding — из CSS)        |
| `Text`          | выражение | текст (стили: цвет, размер, выравнивание)               |
| `Image`         | выражение | картинка (источник — ресурс или `Image`; tint — из CSS) |

`Text`/`Image` — «листовые»: текст и ресурсы не участвуют в лейауте
([лейаут и стили](../dev/architecture.md#лейаут-и-стили) — контейнер не содержит
сырого текста).

## MVP-подмножество — итог

- Всё в «Грамматике» выше + 4 встроенных виджета + 3 параметра-состояния +
  `visible`/`x`/`y`.
- Импорт: `import Name from "./path";`; ресурс:
  `resource Name from "./path.png";`.

## Отложено (v3+)

- Слоты/дети компонентов, `inherit`-наследование компонентов.
- Динамические классы (`class: cond ? "a" : "b"`) — закрыто состояниями.
- `each`-ключи для keyed-reuse, анимации, фокусы/Tab-порядок.
- Инлайн-переопределение стилей в разметке (осознанно запрещено —
  [лейаут и стили](../dev/architecture.md#лейаут-и-стили)).
- Свои виджеты на Rust как элементы разметки.
- Вызов функций компонента из выражений разметки.

## Тест-примеры (покрывают грамматику; bropicker, упрощённо)

### 1. CheckRow (чекбокс-строка)

Скрипт-блок файла:

```js
function toggle() {
  checked = !checked;
  toggled();
}
```

Разметка:

```brul
component CheckRow {
    prop label: String = "";
    prop checked: Bool = false;
    event toggled();

    Row (check-row) {
        View (checkbox, checked: checked, on click: toggle) { }
        Text (text-muted) { label }
    }
}
```

### 2. BrowserItem (строка списка браузеров)

Скрипт-блок файла:

```js
function itemClicked() {
  clicked(index);
}
```

Разметка:

```brul
component BrowserItem {
    prop index: Number = 0;
    prop icon: Image = null;
    prop emoji: String = "";
    prop name: String = "";
    prop isDefault: Bool = false;
    prop selected: Bool = false;
    event clicked(i: Number);

    View (browser-item) {
        View (item-panel, selected: selected, on click: itemClicked) {

            View (item-icon) {
                if (emoji == "") {
                    Image (item-image) { icon }
                }
            }

            Column (item-body) {
                Row (item-title) {
                    Text (text-main) { name }
                    if (isDefault) {
                        View (default-badge) {
                            Text (badge-text) { "По умолчанию" }
                        }
                    }
                }
            }

            Column (item-tail) {
                Image (check-icon, visible: selected) { checkSvg }
                Text (arrow, visible: !selected) { "›" }
            }
        }
    }
}
```

### 3. Header (шапка пикера)

```brul
component Header {
    prop url: String = "";

    Column (header, on drag: dragWindow) {

        Row (header-top) {
            View (app-icon) { }
            Column (header-titles) {
                Text (title) { "Выберите браузер" }
                Text (subtitle) { "Для открытия ссылки" }
            }
        }

        View (url-field) {
            Row (url-row) {
                Image (url-icon) { externalLinkSvg }
                Text (url-text) { url }
            }
        }
    }
}
```

### 4. Окно пикера (сборка)

```brul
window Picker (title: "bropicker", width: 488, height: 800,
               frameless: true, transparent: true, on key: handleKey) {
    Column (root) {
        Header (url: currentUrl) { }
        each (browsers as item, idx (item.index)) {
            BrowserItem (index: idx, name: item.name, icon: item.icon,
                         emoji: item.emoji, isDefault: item.isDefault,
                         selected: idx == selectedIndex,
                         on clicked: launch) { }
        }
        Footer (remember: rememberChoice, always: alwaysAsk) { }
    }
}
```

Соответствующий скрипт:
`function handleKey(ev) { if (ev.code == "Escape") { exit() } }`,
`function launch(i) { ... }`.

Свойства `checkSvg`, `currentUrl` и т.п. — входные данные примера.
