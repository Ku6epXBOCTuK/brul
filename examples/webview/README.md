# webview

Демонстрация нативного WebView как платформенного вью внутри окна BRUL.

Запуск на Windows:

```powershell
just webview
```

`window.brul` объявляет элемент `WebView` с адресом, видимостью и обработчиком
`message`. `brul::webview::factory()` создаёт бэкенд wry, который встраивает
WebView2 как дочернее окно и переводит геометрию из snapshot BRUL в физические
пиксели. Сообщения `window.ipc.postMessage(...)` из JavaScript приходят в
обработчик `message` через `EventSink` и `Handle::widget`.

Ограничения первой итерации: только Windows и непрозрачный GPU-бэкенд. Дочернее
окно WebView всегда лежит над пикселями BRUL: его нельзя обрезать, скрыть под
другими виджетами, наложить эффект или нарисовать поверх него.
software/layered-бэкенд и Wayland не поддерживаются.
