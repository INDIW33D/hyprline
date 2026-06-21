# Hyprline

> 🤖 **AI-Generated Project**: This entire project was developed using **Claude Sonnet 4.5** via GitHub Copilot.  
> 🤖 **Проект, созданный ИИ**: Весь проект был разработан с использованием **Claude Sonnet 4.5** через GitHub Copilot.

**English** | [Русский](#русский)

---

## English

### Why?

**"Why not?"** 🤷‍♂️

This project was created as an experiment to see what's possible when combining modern AI coding assistants with Rust and GTK4. The result is a fully functional system bar for Hyprland, built from scratch with AI assistance.

### What is Hyprline?

A lightweight, self-contained system bar for Hyprland written in Rust with GTK4. No external dependencies for core functionality - everything you need is built right in.

### Features

- 🖥️ **Hyprland Workspaces** - Visual workspace indicator with switching
- 🪟 **Active Window** - Current window title display
- 🕐 **Date & Time** - Interactive widget with popup calendar
- 🔔 **System Tray** - Built-in StatusNotifierWatcher (no waybar needed!)
  - Automatic application detection
  - Auto-removal of closed applications
  - Full StatusNotifier protocol support
- 🔔 **Notifications** - Full notification center with history
  - Popup notifications (top-right, 5s duration)
  - Persistent history stored in SQLite
  - Clear individual or all notifications
  - Multi-notification stacking
- 🔋 **Battery Indicator** - Real-time battery percentage and status
  - UPower D-Bus event monitoring (instant updates!)
  - Charging/discharging status with Nerd Font icons
  - Percentage display
  - No polling - updates on battery events only
- 🔊 **Volume Control** - PipeWire integration
  - Real-time volume slider
  - Mute/unmute toggle
  - Live event updates (no polling!)
- ⌨️ **Keyboard Layout** - Current layout indicator with real-time switching
- 💻 **System Resources** - CPU and RAM monitoring
  - Real-time CPU usage percentage
  - RAM usage in GB
  - Color-coded indicators (green/orange/red)
  - Updates every 2 seconds
  - Reads from /proc/stat and /proc/meminfo
- 🌐 **Network Manager** - WiFi and Ethernet control
  - Current connection status display
  - WiFi network scanning
  - Connect to WiFi networks
  - Signal strength indicator
  - NetworkManager integration via D-Bus
- 💡 **Brightness Control** - Display brightness management (requires Lumen)
  - Real-time brightness display with percentage
  - Interactive slider for brightness adjustment
  - Auto-adjustment toggle
  - D-Bus signal integration (instant updates!)
  - Color-coded icons based on brightness level
  - Debounced updates (200ms) for smooth transitions
- 🎨 **Custom Styling** - CSS-based theming
- 🚀 **Embedded Resources** - All SVG and CSS bundled into binary
- 📦 **Self-Contained** - No external tools required
- 🏗️ **Clean Architecture** - Domain-driven design with clear separation of concerns

### Build

```bash
cargo build --release
```

### Running

```bash
./target/release/hyprline
```

Or add to `~/.config/hypr/hyprland.conf`:
```
exec-once = /path/to/hyprline
```

**Note:** Applications launched BEFORE hyprline won't appear in tray automatically.

### Architecture

Clean architecture principles with clear layer separation:

- **Domain Layer** (`src/domain/`) - Business logic, models, and service traits
  - Battery management
  - Brightness control
  - Date/time handling
  - Keyboard layout
  - Notification system
  - StatusNotifierWatcher protocol
  - System tray management
  - Volume control
  - Workspace management
  - System resources monitoring
  - Network management
  
- **Infrastructure Layer** (`src/infrastructure/`) - External integrations
  - D-Bus services (notifications, tray)
  - Hyprland IPC
  - Lumen brightness control
  - PipeWire audio
  - System battery access
  - NetworkManager integration
  - Event listeners
  
- **UI Layer** (`src/ui/`) - GTK4 widgets and presentation
  - Bar composition
  - Individual widget components
  - User interactions

### System Tray

Built-in StatusNotifierWatcher automatically:
- Registers as `org.kde.StatusNotifierWatcher` on D-Bus
- Accepts registrations from applications
- Monitors D-Bus service lifecycle
- Removes icons of closed applications

**Note:** Applications register when they start. If an app started BEFORE hyprline, it won't appear in tray unless restarted.

### Workspace Keybindings

Hyprline reads registered workspace binds from Hyprland through its IPC socket to display workspace hotkeys.

**How it works:**
1. Reads the registered bind list from Hyprland over the socket
2. Extracts the key from binds using the `workspace` dispatcher
3. For binds with a description in the strict format `hyprline:workspace:<id>:<label>`, uses `<label>` as the workspace label
4. Displays the label on each workspace button (labels are shown uppercase; `semicolon` is shown as `;`)
5. Uses the workspace number as a fallback when no label is available

Malformed descriptions are ignored.

**Example Hyprland Lua callback binds:**
```lua
for i, key in ipairs({ "q", "w", "e", "r", "t" }) do
    hl.bind(mods, key, hl.dsp.focus({ workspace = i }), { description = "hyprline:workspace:" .. i .. ":" .. key })
end
```

**Example registered binds:**
```conf
bind = SUPER, 1, workspace, 1
bind = SUPER, 2, workspace, 2
bind = SUPER, Q, workspace, 3
```

Result: Workspace buttons show `1`, `2`, `Q` respectively.

**Features:**
- Auto-detection of workspace keybindings
- Falls back to numbers if bindings not found

### Dependencies

**System Libraries (required):**
- **GTK4** - UI framework
- **gtk4-layer-shell** - Wayland layer shell protocol
- **PipeWire** - Audio control (via `libpipewire`)
- **WirePlumber** - PipeWire session manager (provides `wpctl` utility)
- **UPower** - Battery monitoring via D-Bus
- **NetworkManager** - Network management via D-Bus
- **Lumen** - Brightness control via D-Bus (optional, required for brightness widget)
- **SQLite** - Notification history storage (bundled in binary)
- **D-Bus** - System integration (pre-installed on most systems)
- **GDK-PixBuf** - Image loading and manipulation

**Rust Crates:**
- `gtk4` - GTK4 bindings
- `gtk4-layer-shell` - Layer shell integration
- `gdk-pixbuf` - Pixbuf bindings for image handling
- `serde` / `serde_json` - JSON serialization
- `chrono` - Date and time handling
- `zbus` - D-Bus communication
- `async-channel` - Async channels for events
- `futures` / `futures-util` - Async runtime utilities and stream handling
- `tokio` - Async runtime
- `pipewire` - PipeWire bindings
- `rusqlite` - SQLite database (bundled)
- `parking_lot` - High-performance synchronization primitives

Everything else is embedded!

### Tech Stack

- **Language:** Rust 🦀
- **UI:** GTK4 + Layer Shell
- **Audio:** PipeWire native API
- **IPC:** Hyprland socket + D-Bus
- **Storage:** SQLite (rusqlite)
- **Build:** Cargo with resource embedding

---

## Русский

### Зачем?

**"А почему нет?"** 🤷‍♂️

Этот проект был создан как эксперимент, чтобы посмотреть, что возможно при сочетании современных AI-ассистентов для программирования с Rust и GTK4. Результат - полностью функциональный системный бар для Hyprland, построенный с нуля при помощи ИИ.

### Что такое Hyprline?

Легкий, автономный системный бар для Hyprland, написанный на Rust с использованием GTK4. Никаких внешних зависимостей для основного функционала - всё необходимое встроено.

### Возможности

- 🖥️ **Рабочие пространства Hyprland** - визуальный индикатор с переключением
- 🪟 **Активное окно** - отображение заголовка текущего окна
- 🕐 **Дата и время** - интерактивный виджет с всплывающим календарем
- 🔔 **Системный трей** - встроенный StatusNotifierWatcher (не нужен waybar!)
  - Автоматическое обнаружение приложений
  - Автоудаление закрытых приложений
  - Полная поддержка протокола StatusNotifier
- 🔔 **Уведомления** - полноценный центр уведомлений с историей
  - Всплывающие уведомления (справа вверху, 5 секунд)
  - Постоянная история в SQLite
  - Очистка отдельных уведомлений или всех сразу
  - Стекирование нескольких уведомлений
- 🔋 **Индикатор батареи** - процент заряда и статус в реальном времени
  - Мониторинг событий UPower D-Bus (мгновенные обновления!)
  - Статус зарядки/разрядки с иконками Nerd Font
  - Отображение процента
  - Без опроса - обновления только по событиям батареи
- 🔊 **Управление громкостью** - интеграция с PipeWire
  - Слайдер громкости в реальном времени
  - Переключатель mute/unmute
  - Обновления по событиям (без опроса!)
- ⌨️ **Раскладка клавиатуры** - индикатор текущей раскладки с обновлением в реальном времени
- 💻 **Системные ресурсы** - мониторинг CPU и RAM
  - Процент использования CPU в реальном времени
  - Использование RAM в GB
  - Цветовые индикаторы нагрузки (зелёный/оранжевый/красный)
  - Обновление каждые 2 секунды
  - Чтение из /proc/stat и /proc/meminfo
- 🌐 **Менеджер сети** - управление WiFi и Ethernet
  - Отображение статуса текущего подключения
  - Сканирование WiFi сетей
  - Подключение к WiFi сетям
  - Индикатор силы сигнала
  - Интеграция с NetworkManager через D-Bus
- 💡 **Управление яркостью** - управление яркостью экрана (требуется Lumen)
  - Отображение яркости в реальном времени с процентами
  - Интерактивный слайдер для регулировки яркости
  - Переключатель автоматической регулировки
  - Интеграция через D-Bus сигналы (мгновенные обновления!)
  - Цветные иконки в зависимости от уровня яркости
  - Debounced обновления (200ms) для плавных переходов
- 🎨 **Кастомизация** - темизация на основе CSS
- 🚀 **Встроенные ресурсы** - все SVG и CSS упакованы в бинарник
- 📦 **Автономность** - не требует внешних инструментов
- 🏗️ **Чистая архитектура** - domain-driven design с четким разделением слоев

### Сборка

```bash
cargo build --release
```

### Запуск

```bash
./target/release/hyprline
```

Или добавьте в `~/.config/hypr/hyprland.conf`:
```
exec-once = /path/to/hyprline
```

**Примечание:** Приложения, запущенные ДО hyprline, не появятся в трее автоматически.

### Архитектура

Принципы чистой архитектуры с четким разделением слоев:

- **Слой домена** (`src/domain/`) - бизнес-логика, модели и трейты сервисов
  - Управление батареей
  - Управление яркостью
  - Обработка даты/времени
  - Раскладка клавиатуры
  - Система уведомлений
  - Протокол StatusNotifierWatcher
  - Управление системным треем
  - Управление громкостью
  - Управление рабочими пространствами
  - Мониторинг системных ресурсов
  - Управление сетью
  
- **Слой инфраструктуры** (`src/infrastructure/`) - внешние интеграции
  - D-Bus сервисы (уведомления, трей)
  - Hyprland IPC
  - Управление яркостью Lumen
  - PipeWire аудио
  - Доступ к системной батарее
  - Интеграция с NetworkManager
  - Слушатели событий
  
- **UI слой** (`src/ui/`) - GTK4 виджеты и представление
  - Композиция бара
  - Отдельные компоненты виджетов
  - Пользовательские взаимодействия

### Системный трей

Встроенный StatusNotifierWatcher автоматически:
- Регистрируется как `org.kde.StatusNotifierWatcher` в D-Bus
- Принимает регистрации от приложений
- Мониторит жизненный цикл D-Bus сервисов
- Удаляет иконки закрытых приложений

**Примечание:** Приложения регистрируются при запуске. Если приложение запустилось ДО hyprline, оно не появится в трее (необходим перезапуск приложения).

### Горячие клавиши воркспейсов

Hyprline читает зарегистрированные бинды воркспейсов из Hyprland через его IPC-сокет, чтобы показывать горячие клавиши воркспейсов.

**Как это работает:**
1. Читает список зарегистрированных биндов из Hyprland по сокету
2. Извлекает клавишу из биндов с диспетчером `workspace`
3. Для биндов с описанием в строгом формате `hyprline:workspace:<id>:<label>` использует `<label>` в качестве метки воркспейса
4. Показывает метку на каждой кнопке воркспейса (метки отображаются заглавными; `semicolon` показывается как `;`)
5. Откатывается к номеру воркспейса, если метка недоступна

Некорректные описания игнорируются.

**Пример Lua callback биндов Hyprland:**
```lua
for i, key in ipairs({ "q", "w", "e", "r", "t" }) do
    hl.bind(mods, key, hl.dsp.focus({ workspace = i }), { description = "hyprline:workspace:" .. i .. ":" .. key })
end
```

**Пример зарегистрированных биндов:**
```conf
bind = SUPER, 1, workspace, 1
bind = SUPER, 2, workspace, 2
bind = SUPER, Q, workspace, 3
```

Результат: Кнопки воркспейсов показывают `1`, `2`, `Q` соответственно.

**Возможности:**
- Автоопределение привязок клавиш воркспейсов
- Откат к номерам, если привязки не найдены

### Зависимости

**Системные библиотеки (необходимые):**
- **GTK4** - UI фреймворк
- **gtk4-layer-shell** - протокол Wayland layer shell
- **PipeWire** - управление аудио (через `libpipewire`)
- **WirePlumber** - менеджер сессий PipeWire (предоставляет утилиту `wpctl`)
- **UPower** - мониторинг батареи через D-Bus
- **NetworkManager** - управление сетью через D-Bus
- **Lumen** - управление яркостью через D-Bus (опционально, требуется для виджета яркости)
- **SQLite** - хранение истории уведомлений (встроено в бинарник)
- **D-Bus** - системная интеграция (предустановлен в большинстве систем)
- **GDK-PixBuf** - загрузка и обработка изображений

**Rust крейты:**
- `gtk4` - привязки GTK4
- `gtk4-layer-shell` - интеграция layer shell
- `gdk-pixbuf` - привязки Pixbuf для работы с изображениями
- `serde` / `serde_json` - JSON сериализация
- `chrono` - обработка даты и времени
- `zbus` - коммуникация с D-Bus
- `async-channel` - асинхронные каналы для событий
- `futures` / `futures-util` - утилиты для асинхронного рантайма и обработки потоков
- `tokio` - асинхронный рантайм
- `pipewire` - привязки PipeWire
- `rusqlite` - база данных SQLite (встроена)
- `parking_lot` - высокопроизводительные примитивы синхронизации

Всё остальное встроено!

### Технологический стек

- **Язык:** Rust 🦀
- **UI:** GTK4 + Layer Shell
- **Аудио:** нативный API PipeWire
- **IPC:** сокет Hyprland + D-Bus
- **Хранилище:** SQLite (rusqlite)
- **Сборка:** Cargo со встраиванием ресурсов

