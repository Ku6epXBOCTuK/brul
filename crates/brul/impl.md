## **AppBuilder**

**Данные:**

- `config: Config` (конфигурация приложения, может включать окна)
- `setup_hooks: Vec<SyncHook>` (синхронные setup функции)
- `async_setup_hooks: Vec<AsyncHook>` (асинхронные setup функции)
- `managed_states: TypeStateMap` (типизированные состояния для `manage()`)

**Методы:**

- `default()` → AppBuilder
- `setup(FnOnce(&AppHandle) → Result)` → AppBuilder
- `setup_async(FnOnce(AppHandle) → Future)` → AppBuilder
- `manage(T)` → AppBuilder (добавляет типизированное состояние)
- `run()` → Result (собирает App, выполняет хуки, запускает)

## **App**

**Данные:**

- `runtime: RuntimeHandle` (платформенный рантайм)
- `state: StateManager` (менеджер состояния)
- `window_manager: WindowManager` (менеджер окон)
- `command_receiver: mpsc::Receiver<AppCommand>` (приёмник команд)

**Методы:**

- `handle()` → AppHandle (создаёт лёгкий дескриптор)
- `run()` → Result (запускает основной цикл)
- `state<T>()` → Option<&T> (получает типизированное состояние)
- `create_window(config)` → WindowHandle (создаёт окно напрямую)
- `exit()` (выход напрямую через runtime)

## **AppHandle**

**Данные:**

- `command_sender: mpsc::Sender<AppCommand>` (отправитель команд)
- `state: Weak<StateManager>` (слабая ссылка на состояние)
- `window_registry: Arc<Mutex<WindowManager>>` (доступ к окнам)

**Методы:**

- `exit()` → Result (отправляет команду на выход)
- `create_window(config)` → WindowHandle (отправляет команду на создание окна)
- `state<T>()` → Option<&T> (получает состояние через Weak ссылку)
- `get_window(id)` → Option<WindowHandle> (получает окно)

## **Runtime**

**Данные:**

- `event_loop: EventLoop` (системный цикл событий)
- `platform_api: PlatformApi` (платформенные вызовы)

**Методы:**

- `poll_events()` → Vec<PlatformEvent> (опрос системных событий)
- `create_window(config)` → WindowId (создание нативного окна)
- `exit()` (выход из цикла событий)

## **StateManager**

**Данные:**

- `typed: TypeStateMap` (типизированные состояния по TypeId)
- `simple: HashMap<String, Value>` (опционально, строковые состояния)

**Методы:**

- `insert<T>(T)` (добавить типизированное состояние)
- `get<T>()` → Option<&T> (получить типизированное состояние)
- `set(key, value)` (установить строковое состояние)
- `get(key)` → Option<&Value> (получить строковое состояние)

## **WindowManager**

**Данные:**

- `windows: HashMap<WindowId, Window>` (окна)

**Методы:**

- `create_window(config)` → WindowHandle (создать окно)
- `get_window(id)` → Option<WindowHandle> (получить окно)
- `close_window(id)` (закрыть окно)

## **Config**

**Данные:**

- `window: WindowConfig` (дефолтные настройки окон)
- `app_name: String` (название приложения)

**Методы:** загрузка/сохранение (опционально)

## **Типизированное хранилище (TypeStateMap)**

**Данные:**

- `map: HashMap<TypeId, Box<Send + Sync>>`

**Методы:**

- `insert<T>(T)`
- `get<T>()` → Option<&T>

## **Поток работы:**

1. `AppBuilder` собирает конфигурацию и хуки
2. `.run()` создаёт `App`, `StateManager`, `WindowManager`, `Runtime`
3. Выполняются setup хуки (синхронные, потом async через block_on)
4. Запускается основной цикл событий
5. `AppHandle` отправляет команды через канал
6. `App` обрабатывает команды в цикле
