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
- 🔋 **Battery Indicator** - Real-time battery percentage (Nerd Font icons)
- 🔊 **Volume Control** - PipeWire integration
  - Real-time volume slider
  - Mute/unmute toggle
  - Live event updates (no polling!)
- ⌨️ **Keyboard Layout** - Current layout indicator with real-time switching
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
  - Date/time handling
  - Keyboard layout
  - Notification system
  - StatusNotifierWatcher protocol
  - System tray management
  - Volume control
  - Workspace management
  
- **Infrastructure Layer** (`src/infrastructure/`) - External integrations
  - D-Bus services (notifications, tray)
  - Hyprland IPC
  - PipeWire audio
  - System battery access
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

Hyprline automatically reads your Hyprland configuration to display workspace hotkeys.

**How it works:**
1. Locates `hyprland.conf` in `$XDG_CONFIG_HOME/hypr/` or `~/.config/hypr/`
2. Parses lines starting with `bind` that contain `workspace`
3. Extracts keybindings in format: `bind = MODIFIERS, KEY, workspace, NUMBER`
4. Displays the key on each workspace button

**Example config:**
```conf
bind = SUPER, 1, workspace, 1
bind = SUPER, 2, workspace, 2
bind = SUPER, Q, workspace, 3
```

Result: Workspace buttons show `1`, `2`, `Q` respectively.

**Features:**
- Auto-detection of workspace keybindings
- Case-insensitive key matching
- Ignores commented lines (`#`)
- Falls back to numbers if bindings not found

### Dependencies

**System Libraries (required):**
- **GTK4** - UI framework
- **gtk4-layer-shell** - Wayland layer shell protocol
- **PipeWire** - Audio control (via `libpipewire`)
- **WirePlumber** - PipeWire session manager (provides `wpctl` utility)
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
- `futures` - Async runtime utilities
- `tokio` - Async runtime
- `pipewire` - PipeWire bindings
- `rusqlite` - SQLite database (bundled)

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
- 🔋 **Индикатор батареи** - процент заряда в реальном времени (иконки Nerd Font)
- 🔊 **Управление громкостью** - интеграция с PipeWire
  - Слайдер громкости в реальном времени
  - Переключатель mute/unmute
  - Обновления по событиям (без опроса!)
- ⌨️ **Раскладка клавиатуры** - индикатор текущей раскладки с обновлением в реальном времени
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
  - Обработка даты/времени
  - Раскладка клавиатуры
  - Система уведомлений
  - Протокол StatusNotifierWatcher
  - Управление системным треем
  - Управление громкостью
  - Управление рабочими пространствами
  
- **Слой инфраструктуры** (`src/infrastructure/`) - внешние интеграции
  - D-Bus сервисы (уведомления, трей)
  - Hyprland IPC
  - PipeWire аудио
  - Доступ к системной батарее
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

Hyprline автоматически читает конфигурацию Hyprland для отображения горячих клавиш воркспейсов.

**Как это работает:**
1. Находит `hyprland.conf` в `$XDG_CONFIG_HOME/hypr/` или `~/.config/hypr/`
2. Парсит строки, начинающиеся с `bind`, содержащие `workspace`
3. Извлекает привязки клавиш в формате: `bind = МОДИФИКАТОРЫ, КЛАВИША, workspace, НОМЕР`
4. Отображает клавишу на каждой кнопке воркспейса

**Пример конфига:**
```conf
bind = SUPER, 1, workspace, 1
bind = SUPER, 2, workspace, 2
bind = SUPER, Q, workspace, 3
```

Результат: Кнопки воркспейсов показывают `1`, `2`, `Q` соответственно.

**Возможности:**
- Автоопределение привязок клавиш воркспейсов
- Регистронезависимое сопоставление клавиш
- Игнорирование закомментированных строк (`#`)
- Откат к номерам, если привязки не найдены

### Зависимости

**Системные библиотеки (необходимые):**
- **GTK4** - UI фреймворк
- **gtk4-layer-shell** - протокол Wayland layer shell
- **PipeWire** - управление аудио (через `libpipewire`)
- **WirePlumber** - менеджер сессий PipeWire (предоставляет утилиту `wpctl`)
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
- `futures` - утилиты для асинхронного рантайма
- `tokio` - асинхронный рантайм
- `pipewire` - привязки PipeWire
- `rusqlite` - база данных SQLite (встроена)

Всё остальное встроено!

### Технологический стек

- **Язык:** Rust 🦀
- **UI:** GTK4 + Layer Shell
- **Аудио:** нативный API PipeWire
- **IPC:** сокет Hyprland + D-Bus
- **Хранилище:** SQLite (rusqlite)
- **Сборка:** Cargo со встраиванием ресурсов


