# Язык стилей

Статус: stable. Канон языка: парсер и кодеген проверяются на этой спеке, примеры
в конце — фикстуры тестов.

Причина ограничений — [лейаут и стили](../dev/architecture.md#лейаут-и-стили).

## Идея

Подмножество CSS по синтаксису, очищенное от наследия веба. Стили компилируются
в **неизменяемые снапшоты**; рантайм только переключает указатели на снапшоты
(состояния) — пересчётов стилей в кадре нет.

```css
.checkbox {
    width: 20px;
    radius: 6px;
    background: transparent;
    border: 2px theme.border-main;
}
.checkbox:checked {
    background: theme.accent;
    border-color: theme.accent;
}
```

`.class { ... }` — базовый снапшот класса; `.class:state` — переопределение для
состояния. Переопределять можно только **визуальные** свойства (цвет,
прозрачность, картинка), не геометрию — нарушение ловит компилятор. Исключение —
`font-weight` у `Text`: смена веса перешейпит лист, не задевая лейаут
контейнеров ([текстовые метрики](#текстовые-метрики)).

## Лексика

- Комментарии: `//` до конца строки.
- Селекторы: `.ident` + необязательный `:state`. Свойства: `kebab-case`.
- Точка с запятой обязательна после каждого свойства.
- Числа: `f32`. Единицы: `px` (длина), `%` (доля от родителя), `ms`
  (длительность перехода). Безразмерные: множители (opacity, stretch).
- Цвета: `#rgb`, `#rrggbb`, `#rrggbbaa` (альфа в конце). Ключевые слова цветов
  (`white`, `transparent`) — минимальный набор.
- Ссылки на тему: `theme.имя-ключа`.

## Грамматика (EBNF)

```ebnf
{{#include style.ebnf}}
```

## Свойства MVP

| Свойство                                               | Тип                               | Дефолт                  | Меняется по состоянию?          |
| ------------------------------------------------------ | --------------------------------- | ----------------------- | ------------------------------- |
| `width`, `height`                                      | px \| %                           | auto (от детей/лейаута) | ❌ геометрия                    |
| `stretch`                                              | число                             | 0                       | ❌ геометрия                    |
| `padding` (1–4 значения, `-left/right/top/bottom`)     | px                                | 0                       | ❌ геометрия                    |
| `spacing`                                              | px                                | 0 (только Row/Column)   | ❌ геометрия                    |
| `justify-content`                                      | start\|center\|end\|space-between | start                   | ❌ геометрия                    |
| `align-items`                                          | start\|center\|end\|stretch       | start                   | ❌ геометрия                    |
| `background`                                           | Color \| Gradient                 | transparent             | ✅                              |
| `border` (шорткат: ширина+цвет)                        | px + (Color \| theme.*)           | 0, transparent          | ❌ (только `border-color`)      |
| `border-width`                                         | px                                | 0                       | ❌ геометрия                    |
| `border-color`                                         | Color                             | transparent             | ✅                              |
| `radius`                                               | px                                | 0                       | ❌ геометрия                    |
| `color`                                                | Color                             | theme-зависимый         | ✅                              |
| `font-size`                                            | px                                | 14                      | ❌ (размер текста — компоновка) |
| `font-family`                                          | String                            | наследуется от окна     | ❌                              |
| `text-align`                                           | left\|center\|right               | left                    | ✅                              |
| `vertical-align`                                       | top\|center\|bottom               | top                     | ✅                              |
| `font-weight`                                          | число (400\|500\|600\|700)        | 400                     | ⚠️ только `Text`, см. ниже      |
| `tint`                                                 | Color                             | — (без окраски)         | ✅ (только Image)               |
| `opacity`                                              | 0.0–1.0                           | 1.0                     | ✅                              |
| `clip`                                                 | Bool                              | false                   | ✅                              |
| `scroll`                                               | Bool                              | false                   | ❌                              |
| `box-shadow` (шорткат: offset-x, offset-y, blur, цвет) | px + px + px + Color              | — (нет тени)            | ❌ (только `box-shadow-color`)  |
| `box-shadow-offset-x`, `box-shadow-offset-y`           | px (может быть отрицательным)     | 0                       | ❌ геометрия                    |
| `box-shadow-blur`                                      | px                                | 0                       | ❌ геометрия                    |
| `box-shadow-color`                                     | Color                             | transparent             | ✅                              |

_«Меняется по состоянию?» = разрешено ли свойство в `.class:state`. Остальные
там запрещены компилятором: переключение снапшота не должно вызывать релайаут.
Единственное исключение — `font-weight`: вместо релайаута он перешейпит `Text`,
правила ниже._

`padding` — шорткат как в CSS: одно значение — все стороны, два — вертикаль и
горизонталь, три — верх, горизонталь, низ, четыре — верх, право, низ, лево.
`border` — шорткат из ширины и цвета: `border: 2px theme.border-main`. Оба
раскладываются при парсинге (`padding` — в `padding-top/right/bottom/left`,
`border` — в `border-width` и `border-color`), поэтому дальше свойства
неотличимы от написанных по отдельности. Ширина — геометрия, поэтому `border` в
`.class:state` запрещён, как и `border-width`; менять в состоянии можно только
`border-color`.

`box-shadow` — шорткат из смещения по x, смещения по y, размытия и цвета:
`box-shadow: 0px 10px 15px #3b82f640`. Смещения и размытие — px, смещения могут
быть отрицательными; цвет — Color или `theme.*`. Тень рисуется под border-box
узла с тем же `radius`, без «spread» и без `inset`. Геометрия тени в
`.class:state` запрещена, как у `border`: по состоянию меняется только
`box-shadow-color`, поэтому тень появляется и исчезает без релайаута.

`linear-gradient` принимает ровно два цвета; угол — как в CSS: `0deg` смотрит
вверх, `90deg` — вправо, дальше по часовой. `to`-синтаксиса нет.

Каждый из двух цветов может быть ссылкой `theme.<ключ>`: ключ темы обязан быть
цветом, и при смене темы градиент пересобирается из новых цветов. Сами ключи
темы по-прежнему хранят только цвет или px, а не градиент.

`clip: true` режет содержимое по border-box узла; скругление — по его `radius`.

`scroll: true` делает контейнер прокручиваемым колесом: содержимое уезжает вверх
внутри его `clip`, смещение ограничено размером содержимого. Нужен `clip: true`;
из скрипта смещение не задаётся.

## Переходы

`animate <свойство>: <число>ms;` задаёт длительность перехода в базовом классе:
при смене состояния свойство интерполируется между снапшотами, а не прыгает.

```css
.button {
    background: theme.surface-item;
    animate background: 300ms;
    animate box-shadow-color: 300ms;
}
.button:hover { background: theme.surface-hover; }
```

- Анимируются только цветовые визуальные свойства: `color`, `border-color`,
  `background` (сплошной цвет), `box-shadow-color`, `tint`, `opacity`. Градиент
  сменяется мгновенно: промежуточных градиентов нет.
- У каждого свойства своя длительность: объявлений `animate` столько, сколько
  свойств. Повтор по тому же свойству в одном классе — выигрывает последний,
  между классами — как у обычных свойств, правый.
- Кривая одна — `ease-out`: быстрый старт и мягкое торможение.
- Смена состояния посреди перехода продолжает с текущего значения, без скачка;
  свойство без `animate` меняется мгновенно.
- `animate` в `.class:state`, неизвестное свойство или единица не `ms` — ошибка
  компиляции: длительность принадлежит базе.

## Текстовые метрики

`font-weight` в `.class:state` разрешён только для `Text` с заданными `width` и
`height`: при смене состояния текст перешейпится и переложится внутри своего
бокса, контейнеры остаются нетронутыми. Класс с таким переопределением без
фиксированного размера — ошибка компиляции. `font-size` и `font-family` в
состоянии запрещены: размер и гарнитура — свойство компоновки, а не состояния.

Выравнивание внутри Row/Column — имена из CSS flexbox: `justify-content` —
главная ось, `align-items` — поперечная. Значения:
`start|center|end|space-between` (justify) и `start|center|end|stretch` (align);
остальные значения flexbox отложены.

## Темы

```css
theme light {
  accent: #3b82f6;
  surface-main: #ffffff;
  text-main: #0f172a;
}
theme dark {
  accent: #3b82f6;
  surface-main: #0f172a;
  text-main: #ffffff;
}
```

- Ключи тем — произвольные `kebab-case` идентификаторы; значения — только
  `Color`. Длины и шрифты в теме — позже.
- Активная тема — рантайм-состояние; переключение — `set_theme("dark")`
  (встроенная функция скрипта). Цвета тем приходят из палитры, которую
  сгенерированный код выбирает по имени темы, поэтому переключение мгновенное.
- Дефолт — первая объявленная тема.
- Набор ключей и типы значений у всех тем одинаковые (валидация компилятора).

## Что не поддерживается

Синтаксис CSS взят целиком там, где он не тянет за собой механику веба: правила
`.class`, hex-цвета, `linear-gradient`, шорткаты, проценты.

Не поддерживается и не будет ([причина](../dev/architecture.md#лейаут-и-стили)):

- каскад, специфичность, наследование, `!important`;
- схлопывание margin — вместо него `spacing` и `padding`;
- `float` — вместо него зоны исключения;
- состояния через пересчёт каскада;
- устаревшие свойства веба.

Не поддерживается из-за единственного контекста масштабирования (px + % + DPI):
`em`, `rem`, `vw`, `vh`, `pt`, `calc()`.

Отложено (v3+, не «выброшено»):

- `transform` (scale/translate/rotate) — любые изменения размера в рантайме
  только визуальной трансформацией, без инвалидации лейаута; на нём будут
  анимации перемещения (вход и выход, FLIP);
- скругление отдельных углов, `:focus`, media-queries, CSS-переменные (замена —
  темы), `position: absolute` в стилях (в MVP абсолют — параметры `x`/`y` в
  разметке).

## Семантика выполнения

- Парсинг, разрешение `theme.*` и построение снапшотов — **один раз** при
  компиляции/запуске. Снапшот = плоский набор значений свойств.
- Несколько классов на элементе: значения мержатся слева направо, правый класс
  выигрывает при конфликте (вместо специфичности — порядок).
- `.class:state` валиден, только если есть базовый `.class`.
- Состояния накладываются на базу слоями: `:selected`, `:checked`, `:hover`,
  `:pressed`, `:disabled`. Правило состояния меняет только те свойства, которые
  объявлены в нём самом, остальные остаются от базы и нижних слоёв, поэтому
  `:hover` не стирает рамку `:selected`.
- Класс по выражению (`class: Expr`) даёт один класс поверх статических. Он тоже
  меняет только свойства своего правила, а лейаут из него требует повторной
  раскладки. Пустое значение — класса нет; неизвестное имя — ошибка в лог один
  раз, вид остаётся статическим.
- Неизвестное свойство, значение или ключ темы — **ошибка компиляции** с
  позицией.

## Тест-примеры (покрывают грамматику)

### 1. Стили CheckRow (из markup.md)

```css
.check-row { width: 100%; spacing: 12px; padding: 4px; }
.checkbox {
    width: 20px;
    height: 20px;
    radius: 6px;
    background: transparent;
    border: 2px theme.border-main;
}
.checkbox:checked {
    background: theme.accent;
    border-color: theme.accent;
}
.text-muted { color: theme.text-muted; font-size: 14px; }
```

### 2. Стили BrowserItem

```css
.browser-item { height: 84px; background: transparent; }
.item-panel {
    height: 76px;
    radius: 12px;
    background: theme.surface-item;
    border: 2px transparent;
}
.item-panel:selected {
    background: theme.surface-elevated;
    border-color: theme.accent-ring;
}
.item-panel:hover { background: theme.surface-hover; }
.item-icon { width: 48px; radius: 12px; background: theme.surface-elevated; }
.item-image { width: 30px; height: 30px; tint: theme.text-muted; }
.default-badge {
    height: 22px;
    radius: 11px;
    background: theme.accent-soft;
    padding: 0px 10px;
}
.badge-text { color: #60a5fa; font-size: 12px; vertical-align: center; }
.text-main {
    width: 100%;
    height: 20px;
    color: theme.text-main;
    font-size: 16px;
    font-weight: 500;
    vertical-align: center;
}
.text-main:hover { font-weight: 600; }
```

### 3. Темы (фрагмент bropicker)

```css
theme light {
  accent: #3b82f6;
  surface-main: #ffffff;
  surface-item: #f8fafc;
  surface-hover: #ffffff;
  border-main: #e2e8f0;
  text-main: #0f172a;
  text-muted: #64748b;
}
theme dark {
  accent: #3b82f6;
  surface-main: #0f172a;
  surface-item: #1e293b4d;
  surface-hover: #1e293b;
  border-main: #334155;
  text-main: #ffffff;
  text-muted: #94a3b8;
}
```

### 4. Градиент (иконка приложения)

```css
.app-icon {
  width: 48px;
  height: 48px;
  radius: 12px;
  background: linear-gradient(135deg, #3b82f6, #06b6d4);
}
```

### 5. Тень по наведению

```css
.raised {
  width: 44px;
  height: 44px;
  radius: 12px;
  background: #3b82f6;
  box-shadow: 0px 10px 15px transparent;
  animate background: 300ms;
  animate box-shadow-color: 300ms;
}
.raised:hover {
  background: #60a5fa;
  box-shadow-color: #3b82f640;
}
```
