mod config;
mod domain;
mod infrastructure;
mod shared_state;
mod ui;

use domain::battery_service::BatteryService;
use domain::bluetooth_service::BluetoothService;
use domain::brightness_service::BrightnessService;
use domain::datetime_service::DateTimeService;
use domain::keyboard_layout_service::KeyboardLayoutService;
use domain::models::DateTimeConfig;
use domain::network_service::NetworkService;
use domain::notification_service::NotificationService;
use domain::status_notifier_watcher_service::StatusNotifierWatcherService;
use domain::submap_service::SubmapService;
use domain::system_resources_service::SystemResourcesService;
use domain::system_tray_service::SystemTrayService;
use domain::volume_service::VolumeService;
use domain::workspace_service::WorkspaceService;
use infrastructure::bluez_bluetooth::BluezBluetoothService;
use infrastructure::dbus_status_notifier_watcher::DbusStatusNotifierWatcher;
use infrastructure::hyprland_ipc::HyprlandIpc;
use infrastructure::hyprland_keyboard_layout::HyprlandKeyboardLayoutService;
use infrastructure::hyprland_submap::HyprlandSubmapService;
use infrastructure::lumen_brightness::LumenBrightnessService;
use infrastructure::monitor_listener::{start_monitor_listener, MonitorEvent};
use infrastructure::networkmanager::NetworkManagerService;
use infrastructure::remote_notification_service::RemoteNotificationService;
use infrastructure::status_notifier_tray::StatusNotifierTrayService;
use infrastructure::system_battery::SystemBatteryService;
use infrastructure::system_datetime::SystemDateTimeService;
use infrastructure::system_resources::LinuxSystemResources;
use shared_state::get_shared_state;
use ui::bar::Bar;
use ui::volume_osd::VolumeOsd;

use gtk4::prelude::*;
use gtk4::{gdk, glib};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExistingBarState {
    monitor_name: String,
    is_visible: bool,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct BarReconciliationPlan {
    create: Vec<String>,
    hide: Vec<String>,
    show: Vec<String>,
    rebuild: Vec<String>,
    close_removed: Vec<String>,
}

struct ManagedBar<T> {
    bar: T,
    visible: bool,
    listeners_setup: bool,
}

impl<T> ManagedBar<T> {
    const fn new(bar: T, visible: bool, listeners_setup: bool) -> Self {
        Self {
            bar,
            visible,
            listeners_setup,
        }
    }
}

struct ManagedBars<T> {
    order: Vec<String>,
    by_monitor: HashMap<String, ManagedBar<T>>,
}

impl<T> Default for ManagedBars<T> {
    fn default() -> Self {
        Self {
            order: Vec::new(),
            by_monitor: HashMap::new(),
        }
    }
}

impl<T> ManagedBars<T> {
    fn insert(&mut self, monitor_name: String, managed_bar: ManagedBar<T>) {
        if !self.by_monitor.contains_key(&monitor_name) {
            self.order.push(monitor_name.clone());
        }

        self.by_monitor.insert(monitor_name, managed_bar);
    }

    fn get_mut(&mut self, monitor_name: &str) -> Option<&mut ManagedBar<T>> {
        self.by_monitor.get_mut(monitor_name)
    }

    fn remove(&mut self, monitor_name: &str) -> Option<ManagedBar<T>> {
        self.order.retain(|name| name != monitor_name);
        self.by_monitor.remove(monitor_name)
    }

    fn existing_states(&self) -> Vec<ExistingBarState> {
        self.order
            .iter()
            .filter_map(|monitor_name| {
                self.by_monitor.get(monitor_name).map(|managed_bar| ExistingBarState {
                    monitor_name: monitor_name.clone(),
                    is_visible: managed_bar.visible,
                })
            })
            .collect()
    }
}

trait BarLifecycle {
    fn setup_event_listener(&mut self);
    fn present(&mut self);
    fn hide(&mut self);
    fn rebuild_widgets(&mut self);
    fn close(self);
}

impl BarLifecycle for Bar {
    fn setup_event_listener(&mut self) {
        Bar::setup_event_listener(self);
    }

    fn present(&mut self) {
        Bar::present(self);
    }

    fn hide(&mut self) {
        Bar::hide(self);
    }

    fn rebuild_widgets(&mut self) {
        Bar::rebuild_widgets(self);
    }

    fn close(self) {
        Bar::close(&self);
    }
}

fn apply_bar_reconciliation_plan<T, F>(
    managed_bars: &mut ManagedBars<T>,
    plan: &BarReconciliationPlan,
    mut create_bar: F,
) where
    T: BarLifecycle,
    F: FnMut(&str) -> T,
{
    for monitor_name in &plan.close_removed {
        if let Some(managed_bar) = managed_bars.remove(monitor_name) {
            managed_bar.bar.close();
        }
    }

    for monitor_name in &plan.hide {
        if let Some(managed_bar) = managed_bars.get_mut(monitor_name) {
            managed_bar.bar.hide();
            managed_bar.visible = false;
        }
    }

    for monitor_name in &plan.show {
        if let Some(managed_bar) = managed_bars.get_mut(monitor_name) {
            if !managed_bar.listeners_setup {
                managed_bar.bar.setup_event_listener();
                managed_bar.listeners_setup = true;
            }

            managed_bar.bar.present();
            managed_bar.visible = true;
        }
    }

    for monitor_name in &plan.rebuild {
        if let Some(managed_bar) = managed_bars.get_mut(monitor_name) {
            managed_bar.bar.rebuild_widgets();
        }
    }

    for monitor_name in &plan.create {
        let mut bar = create_bar(monitor_name);
        bar.setup_event_listener();
        bar.present();
        managed_bars.insert(monitor_name.clone(), ManagedBar::new(bar, true, true));
    }
}

fn reconcile_managed_bars<T, F>(
    managed_bars: &mut ManagedBars<T>,
    monitors: &[domain::models::Monitor],
    config: &config::HyprlineConfig,
    create_bar: F,
) where
    T: BarLifecycle,
    F: FnMut(&str) -> T,
{
    let existing_bar_states = managed_bars.existing_states();
    let plan = plan_bar_reconciliation(&existing_bar_states, monitors, config);
    apply_bar_reconciliation_plan(managed_bars, &plan, create_bar);
}

fn clone_runtime_config() -> config::HyprlineConfig {
    config::get_config().read().unwrap().clone()
}

fn log_startup_disabled_monitors(
    monitors: &[domain::models::Monitor],
    config: &config::HyprlineConfig,
) {
    if monitors.is_empty() {
        if !config.is_bar_enabled_for_monitor("default") {
            eprintln!("[Main] Skipping disabled fallback bar for monitor: default");
        }
        return;
    }

    for monitor in monitors {
        if !config.is_bar_enabled_for_monitor(&monitor.name) {
            eprintln!("[Main] Skipping disabled bar for monitor: {}", monitor.name);
        }
    }
}

fn desired_bar_monitor_names(
    monitors: &[domain::models::Monitor],
    config: &config::HyprlineConfig,
) -> Vec<String> {
    if monitors.is_empty() {
        return if config.is_bar_enabled_for_monitor("default") {
            vec!["default".to_string()]
        } else {
            Vec::new()
        };
    }

    monitors
        .iter()
        .filter(|monitor| config.is_bar_enabled_for_monitor(&monitor.name))
        .map(|monitor| monitor.name.clone())
        .collect()
}

fn plan_bar_reconciliation(
    existing_bar_states: &[ExistingBarState],
    monitors: &[domain::models::Monitor],
    config: &config::HyprlineConfig,
) -> BarReconciliationPlan {
    let mut plan = BarReconciliationPlan::default();
    let desired_monitor_names = desired_bar_monitor_names(monitors, config);

    for desired_monitor_name in &desired_monitor_names {
        match existing_bar_states
            .iter()
            .find(|bar| bar.monitor_name == *desired_monitor_name)
        {
            Some(existing_bar) if existing_bar.is_visible => {
                plan.rebuild.push(desired_monitor_name.clone());
            }
            Some(existing_bar) if !existing_bar.is_visible => {
                plan.show.push(desired_monitor_name.clone());
            }
            None => {
                plan.create.push(desired_monitor_name.clone());
            }
            Some(_) => unreachable!("visible state handled exhaustively"),
        }
    }

    for monitor in monitors {
        if config.is_bar_enabled_for_monitor(&monitor.name) {
            continue;
        }

        if let Some(existing_bar) = existing_bar_states
            .iter()
            .find(|bar| bar.monitor_name == monitor.name && bar.is_visible)
        {
            plan.hide.push(existing_bar.monitor_name.clone());
        }
    }

    for existing_bar in existing_bar_states {
        let monitor_still_exists = monitors
            .iter()
            .any(|monitor| monitor.name == existing_bar.monitor_name);

        if !monitor_still_exists {
            plan.close_removed.push(existing_bar.monitor_name.clone());
        }
    }

    plan
}

fn main() -> glib::ExitCode {
    let app = gtk4::Application::builder()
        .application_id("ru.hyprline.bar")
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();

    // CSS загружаем в startup (один раз)
    app.connect_startup(|_app| {
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(include_str!("styles.css"));

        gtk4::style_context_add_provider_for_display(
            &gdk::Display::default().expect("error initializing gtk4 style context"),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    // UI создаём в activate (вызывается при каждом запуске/активации)
    app.connect_activate(|app| {
        // Проверяем, есть ли уже окна — если да, просто активируем
        if !app.windows().is_empty() {
            for window in app.windows() {
                if window.is_visible() {
                    window.present();
                }
            }
            return;
        }

        build_ui(app);
    });

    app.run()
}

/// Преобразует имя раскладки в короткое представление
fn get_layout_full_name(short_name: &str) -> String {
    match short_name.to_lowercase().as_str() {
        "russian" | "ru" => "RU".to_string(),
        "english (us)" | "us" | "english" => "US".to_string(),
        "german" | "de" => "DE".to_string(),
        "french" | "fr" => "FR".to_string(),
        "spanish" | "es" => "ES".to_string(),
        "italian" | "it" => "IT".to_string(),
        "portuguese" | "pt" => "PT".to_string(),
        "polish" | "pl" => "PL".to_string(),
        "ukrainian" | "ua" => "UA".to_string(),
        "japanese" | "jp" => "JP".to_string(),
        "korean" | "kr" => "KR".to_string(),
        "chinese" | "cn" => "CN".to_string(),
        _ => short_name
            .chars()
            .take(2)
            .collect::<String>()
            .to_uppercase(),
    }
}

fn build_ui(app: &gtk4::Application) {
    // Запускаем свой StatusNotifierWatcher D-Bus сервис
    let watcher_service = Arc::new(DbusStatusNotifierWatcher::new());
    if let Err(e) = watcher_service.start() {
        eprintln!(
            "[Main] Warning: Failed to start StatusNotifierWatcher: {}",
            e
        );
    }

    // Даём время сервису зарегистрироваться в D-Bus
    std::thread::sleep(std::time::Duration::from_millis(200));

    let hyprland_ipc_impl = Arc::new(HyprlandIpc::new());
    let service: Arc<dyn WorkspaceService + Send + Sync> = hyprland_ipc_impl.clone();

    // Создаём системный трей сервис
    let tray_service_impl = Arc::new(StatusNotifierTrayService::new());
    let tray_service: Arc<dyn SystemTrayService + Send + Sync> = tray_service_impl.clone();

    // Создаём DateTime сервис
    let datetime_service: Arc<dyn DateTimeService + Send + Sync> =
        Arc::new(SystemDateTimeService::new());
    let datetime_config = DateTimeConfig::default();

    // Создаём Battery сервис с мониторингом событий
    let (battery_tx, battery_rx) = async_channel::unbounded();
    let battery_service_impl = Arc::new(SystemBatteryService::new());
    battery_service_impl.start_monitoring(battery_tx);
    let battery_service: Arc<dyn BatteryService + Send + Sync> = battery_service_impl;

    // Создаём Volume сервис с мониторингом
    let (volume_tx, volume_rx) = infrastructure::pipewire_volume::create_volume_channel();
    let (volume_osd_tx, volume_osd_rx) = async_channel::unbounded();

    let mut volume_service_impl = infrastructure::pipewire_volume::PipewireVolume::new();
    volume_service_impl.start_monitoring(volume_tx);
    volume_service_impl.start_monitoring(volume_osd_tx);
    let volume_service: Arc<dyn VolumeService + Send + Sync> = Arc::new(volume_service_impl);

    // Создаём Volume OSD (On-Screen Display)
    let volume_osd = Arc::new(VolumeOsd::new(app));

    // Создаём Notification сервис (подключается к hyprline-notifications через D-Bus)
    let notification_service: Arc<dyn NotificationService + Send + Sync> =
        Arc::new(RemoteNotificationService::new());

    // Создаём KeyboardLayout сервис
    let keyboard_layout_service: Arc<dyn KeyboardLayoutService + Send + Sync> =
        Arc::new(HyprlandKeyboardLayoutService::new());

    // Создаём канал для событий смены раскладки
    let (keyboard_layout_tx, keyboard_layout_rx) =
        infrastructure::keyboard_layout_listener::create_keyboard_layout_channel();

    // Запускаем мониторинг событий раскладки
    infrastructure::keyboard_layout_listener::start_keyboard_layout_listener(keyboard_layout_tx);

    // Создаём SystemResources сервис
    let system_resources_service: Arc<dyn SystemResourcesService + Send + Sync> =
        Arc::new(LinuxSystemResources::new());

    // Создаём Network сервис
    let network_service: Arc<dyn NetworkService + Send + Sync> =
        Arc::new(NetworkManagerService::new());

    // Создаём Bluetooth сервис
    let bluetooth_service: Arc<dyn BluetoothService + Send + Sync> =
        Arc::new(BluezBluetoothService::new());

    // Создаём Brightness сервис
    eprintln!("[Main] Creating brightness service...");
    let brightness_service_impl = match LumenBrightnessService::new() {
        Ok(service) => {
            eprintln!("[Main] Brightness service created");
            Arc::new(service)
        }
        Err(e) => {
            eprintln!("[Brightness] ✗ Failed to connect: {}", e);
            eprintln!("[Brightness] Make sure Lumen service is running");
            panic!("Cannot create brightness service");
        }
    };
    let brightness_service: Arc<dyn BrightnessService + Send + Sync> =
        brightness_service_impl.clone();

    // Сначала подписываемся на изменения яркости
    let shared_state_brightness = get_shared_state();
    brightness_service.subscribe_brightness_changed(Arc::new(move |value| {
        shared_state_brightness.update_brightness(value);
    }));

    // Затем запускаем мониторинг (который также получит начальное значение)
    brightness_service_impl.clone().start_signal_monitoring();

    // Создаём Submap сервис
    let submap_service_impl = Arc::new(HyprlandSubmapService::new());
    let submap_service: Arc<dyn SubmapService + Send + Sync> = submap_service_impl.clone();

    // Создаём канал для событий submap
    let (submap_tx, submap_rx) = infrastructure::submap_listener::create_submap_channel();

    // Запускаем мониторинг событий submap
    infrastructure::submap_listener::start_submap_listener(submap_tx);

    // Запускаем мониторинг изменений конфига Hyprland для обновления названий биндингов
    let (config_change_tx, config_change_rx) = async_channel::unbounded::<()>();
    submap_service_impl
        .clone()
        .start_config_monitoring(config_change_tx);

    // Создаём канал для обновлений трея
    let (tray_tx, tray_rx) = async_channel::unbounded();

    // Запускаем мониторинг трея
    tray_service_impl.start_monitoring(tray_tx.clone());

    // Подключаем обработчик завершения приложения
    let watcher_service_cleanup = watcher_service.clone();
    let tray_service_cleanup = tray_service_impl.clone();
    app.connect_shutdown(move |_| {
        eprintln!("[Main] Application shutting down...");

        // Останавливаем мониторинг трея
        tray_service_cleanup.stop();

        // Останавливаем StatusNotifierWatcher
        if let Err(e) = watcher_service_cleanup.stop() {
            eprintln!(
                "[Main] Warning: Failed to stop StatusNotifierWatcher: {}",
                e
            );
        }

        eprintln!("[Main] Cleanup completed");
    });

    // === ЦЕНТРАЛИЗОВАННАЯ ОБРАБОТКА СОБЫТИЙ ===
    let shared_state = get_shared_state();

    // Обработка событий громкости
    {
        let shared_state = shared_state.clone();
        let volume_service = volume_service.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok(_) = volume_rx.try_recv() {
                if let Some(info) = volume_service.get_volume_info() {
                    shared_state.update_volume(Some(info));
                }
            }
            glib::ControlFlow::Continue
        });
    }

    // Обработка событий батареи
    {
        let shared_state = shared_state.clone();
        let battery_service = battery_service.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok(_) = battery_rx.try_recv() {
                let info = battery_service.get_battery_info();
                shared_state.update_battery(info);
            }
            glib::ControlFlow::Continue
        });
    }

    // Обработка событий трея
    {
        let shared_state = shared_state.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok(items) = tray_rx.try_recv() {
                shared_state.update_tray(items);
            }
            glib::ControlFlow::Continue
        });
    }

    // Обработка событий раскладки клавиатуры
    {
        let shared_state = shared_state.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while let Ok(layout_name) = keyboard_layout_rx.try_recv() {
                let full_name = get_layout_full_name(&layout_name);
                let layout = domain::models::KeyboardLayout {
                    short_name: layout_name,
                    full_name,
                };
                shared_state.update_keyboard_layout(layout);
            }
            glib::ControlFlow::Continue
        });
    }

    // Обработка событий submap
    {
        let shared_state = shared_state.clone();
        let submap_service_clone = submap_service.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while let Ok(submap_name) = submap_rx.try_recv() {
                eprintln!(
                    "[Main] Submap changed: '{}' (active: {})",
                    submap_name,
                    !submap_name.is_empty()
                );
                let bindings = submap_service_clone.get_submap_bindings(&submap_name);
                eprintln!("[Main] Found {} bindings for submap", bindings.len());
                let submap = domain::models::SubmapInfo {
                    name: submap_name,
                    bindings,
                };
                shared_state.update_submap(submap);
            }
            glib::ControlFlow::Continue
        });
    }

    // Обработка изменений конфига Hyprland (обновление названий биндингов)
    {
        let shared_state = shared_state.clone();
        let submap_service_clone = submap_service.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while config_change_rx.try_recv().is_ok() {
                // Если мы сейчас в submap - обновляем его с новыми данными
                let current_submap = shared_state.get_submap();
                if current_submap.is_active() {
                    let bindings = submap_service_clone.get_submap_bindings(&current_submap.name);
                    let updated_submap = domain::models::SubmapInfo {
                        name: current_submap.name,
                        bindings,
                    };
                    shared_state.update_submap(updated_submap);
                    eprintln!("[Main] Submap bindings updated from config change");
                }
            }
            glib::ControlFlow::Continue
        });
    }

    // Volume OSD обработка
    {
        let volume_osd_clone = volume_osd.clone();
        let volume_service_clone = volume_service.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok(_) = volume_osd_rx.try_recv() {
                if let Some(info) = volume_service_clone.get_volume_info() {
                    volume_osd_clone.show_volume(info.volume, info.muted);
                }
            }
            glib::ControlFlow::Continue
        });
    }

    // Централизованное обновление системных ресурсов каждые 2 секунды
    {
        let shared_state = shared_state.clone();
        let system_resources_service = system_resources_service.clone();
        glib::timeout_add_local(std::time::Duration::from_secs(2), move || {
            let resources = system_resources_service.get_resources();
            shared_state.update_system_resources(resources);
            glib::ControlFlow::Continue
        });
    }

    // Централизованное обновление сети каждые 2 секунды
    {
        let shared_state = shared_state.clone();
        let network_service = network_service.clone();
        glib::timeout_add_local(std::time::Duration::from_secs(2), move || {
            let connection = network_service.get_current_connection();
            shared_state.update_network(connection);
            glib::ControlFlow::Continue
        });
    }

    // Централизованное обновление Bluetooth каждые 3 секунды
    {
        let shared_state = shared_state.clone();
        let bluetooth_service = bluetooth_service.clone();
        glib::timeout_add_local(std::time::Duration::from_secs(3), move || {
            let info = bluetooth_service.get_bluetooth_info();
            shared_state.update_bluetooth(info);
            glib::ControlFlow::Continue
        });
    }

    // Инициализация начального состояния
    if let Some(info) = battery_service.get_battery_info() {
        shared_state.update_battery(Some(info));
    }
    if let Some(info) = volume_service.get_volume_info() {
        shared_state.update_volume(Some(info));
    }
    if let Some(layout) = keyboard_layout_service.get_current_layout() {
        shared_state.update_keyboard_layout(layout);
    }
    // Инициализация уведомлений (если сервис доступен)
    if notification_service.is_connected() {
        shared_state.update_notifications(notification_service.get_count());
    }
    // Яркость уже инициализирована в отдельном потоке ранее
    // Инициализация системных ресурсов
    shared_state.update_system_resources(system_resources_service.get_resources());
    // Инициализация сети
    shared_state.update_network(network_service.get_current_connection());
    // Инициализация Bluetooth
    shared_state.update_bluetooth(bluetooth_service.get_bluetooth_info());

    // Подписка на события сервиса уведомлений в реальном времени
    {
        use infrastructure::notification_client::NotificationEvent;

        let shared_state_for_listener = shared_state.clone();
        let (tx, rx) = async_channel::unbounded::<NotificationEvent>();

        // Запускаем listener в отдельном потоке
        infrastructure::notification_client::start_notification_listener(Arc::new(move |event| {
            let _ = tx.send_blocking(event);
        }));

        // Обрабатываем события в главном потоке GTK
        let prev_count = std::rc::Rc::new(std::cell::RefCell::new(0usize));
        let prev_count_clone = prev_count.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while let Ok(event) = rx.try_recv() {
                match event {
                    NotificationEvent::CountChanged(count) => {
                        let new_count = count as usize;
                        let old_count = *prev_count_clone.borrow();

                        // Если количество увеличилось - пришло новое уведомление
                        if new_count > old_count && old_count > 0 {
                            shared_state_for_listener.trigger_notification_alert();
                        }

                        *prev_count_clone.borrow_mut() = new_count;
                        shared_state_for_listener.update_notifications(new_count);
                    }
                    NotificationEvent::ServiceAvailable => {
                        eprintln!("[Main] Notification service connected");
                        shared_state_for_listener.set_notification_service_available(true);
                    }
                    NotificationEvent::ServiceUnavailable => {
                        eprintln!("[Main] Notification service disconnected");
                        shared_state_for_listener.set_notification_service_available(false);
                        shared_state_for_listener.update_notifications(0);
                        *prev_count_clone.borrow_mut() = 0;
                    }
                }
            }
            glib::ControlFlow::Continue
        });
    }

    let workspace_keys = hyprland_ipc_impl.get_workspace_key_labels();
    let monitors = service.get_monitors();
    let config = clone_runtime_config();
    eprintln!(
        "[Main] Found {} monitors: {:?}",
        monitors.len(),
        monitors.iter().map(|m| &m.name).collect::<Vec<_>>()
    );
    log_startup_disabled_monitors(&monitors, &config);

    let create_bar = {
        let app = app.clone();
        let workspace_keys = workspace_keys.clone();
        let service = service.clone();
        let tray_service = tray_service.clone();
        let datetime_service = datetime_service.clone();
        let datetime_config = datetime_config.clone();
        let battery_service = battery_service.clone();
        let volume_service = volume_service.clone();
        let notification_service = notification_service.clone();
        let keyboard_layout_service = keyboard_layout_service.clone();
        let system_resources_service = system_resources_service.clone();
        let network_service = network_service.clone();
        let bluetooth_service = bluetooth_service.clone();
        let brightness_service = brightness_service.clone();
        let submap_service = submap_service.clone();
        let shared_state = shared_state.clone();

        std::rc::Rc::new(move |monitor_name: &str| {
            eprintln!("[Main] Creating bar for monitor: {}", monitor_name);
            Bar::new(
                &app,
                monitor_name,
                workspace_keys.clone(),
                service.clone(),
                tray_service.clone(),
                datetime_service.clone(),
                datetime_config.clone(),
                battery_service.clone(),
                volume_service.clone(),
                notification_service.clone(),
                keyboard_layout_service.clone(),
                system_resources_service.clone(),
                network_service.clone(),
                bluetooth_service.clone(),
                brightness_service.clone(),
                submap_service.clone(),
                shared_state.clone(),
            )
        })
    };

    let bars: Arc<std::sync::Mutex<ManagedBars<Bar>>> =
        Arc::new(std::sync::Mutex::new(ManagedBars::default()));

    {
        let mut bars = bars.lock().unwrap();
        reconcile_managed_bars(&mut bars, &monitors, &config, |monitor_name| {
            create_bar(monitor_name)
        });
    }

    // Подписка на изменения конфигурации для hot reload
    {
        let bars_for_config = bars.clone();
        let service_for_config = service.clone();
        let create_bar_for_config = create_bar.clone();
        let (config_tx, config_rx) = async_channel::unbounded::<()>();

        config::subscribe_config_changes(move || {
            let _ = config_tx.send_blocking(());
        });

        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while config_rx.try_recv().is_ok() {
                eprintln!("[Main] Config changed, reconciling bars...");
                let monitors = service_for_config.get_monitors();
                let config = clone_runtime_config();
                let mut bars = bars_for_config.lock().unwrap();
                reconcile_managed_bars(&mut bars, &monitors, &config, |monitor_name| {
                    create_bar_for_config(monitor_name)
                });
            }
            glib::ControlFlow::Continue
        });
    }

    // Подписка на события мониторов (добавление/удаление)
    {
        let (monitor_tx, monitor_rx) = async_channel::unbounded::<MonitorEvent>();

        start_monitor_listener(move |event| {
            let _ = monitor_tx.send_blocking(event);
        });

        let bars_for_monitors = bars.clone();
        let service_clone = service.clone();
        let create_bar_for_monitors = create_bar.clone();

        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while let Ok(event) = monitor_rx.try_recv() {
                match event {
                    MonitorEvent::Added(monitor_name) => {
                        // Задержка, чтобы GDK успел зарегистрировать новый монитор
                        let bars_clone = bars_for_monitors.clone();
                        let service = service_clone.clone();
                        let create_bar = create_bar_for_monitors.clone();

                        glib::timeout_add_local_once(
                            std::time::Duration::from_millis(300),
                            move || {
                                eprintln!(
                                    "[Main] Monitor added event settled, reconciling snapshot after: {}",
                                    monitor_name
                                );
                                let monitors = service.get_monitors();
                                let config = clone_runtime_config();
                                let mut bars = bars_clone.lock().unwrap();
                                reconcile_managed_bars(&mut bars, &monitors, &config, |name| {
                                    create_bar(name)
                                });
                            },
                        );
                    }
                    MonitorEvent::Removed(monitor_name) => {
                        let mut bars = bars_for_monitors.lock().unwrap();

                        // Находим и удаляем бар для этого монитора
                        if let Some(managed_bar) = bars.remove(&monitor_name) {
                            eprintln!("[Main] Removing bar for monitor: {}", monitor_name);
                            managed_bar.bar.close();
                            eprintln!("[Main] ✓ Bar removed for monitor: {}", monitor_name);
                        } else {
                            eprintln!("[Main] No bar found for monitor: {}", monitor_name);
                        }
                    }
                }
            }
            glib::ControlFlow::Continue
        });
    }

    // Setup и present для всех баров
    {
        let bars = bars.lock().unwrap();
        let visible_count = bars
            .existing_states()
            .into_iter()
            .filter(|bar| bar.is_visible)
            .count();
        eprintln!("[Main] Startup reconciliation presented {} bars", visible_count);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        apply_bar_reconciliation_plan, desired_bar_monitor_names, plan_bar_reconciliation,
        BarLifecycle, BarReconciliationPlan, ExistingBarState, ManagedBar, ManagedBars,
    };
    use crate::config::HyprlineConfig;
    use crate::domain::models::Monitor;
    use std::sync::{Arc, Mutex};

    fn monitor(name: &str, id: i32) -> Monitor {
        Monitor {
            name: name.to_string(),
            id,
        }
    }

    fn visible_bar(monitor_name: &str) -> ExistingBarState {
        ExistingBarState {
            monitor_name: monitor_name.to_string(),
            is_visible: true,
        }
    }

    fn hidden_bar(monitor_name: &str) -> ExistingBarState {
        ExistingBarState {
            monitor_name: monitor_name.to_string(),
            is_visible: false,
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum RecordedAction {
        Create(String),
        Setup(String),
        Present(String),
        Hide(String),
        Rebuild(String),
        Close(String),
    }

    #[derive(Clone)]
    struct FakeBar {
        monitor_name: String,
        actions: Arc<Mutex<Vec<RecordedAction>>>,
    }

    impl FakeBar {
        fn new(monitor_name: &str, actions: Arc<Mutex<Vec<RecordedAction>>>) -> Self {
            actions
                .lock()
                .unwrap()
                .push(RecordedAction::Create(monitor_name.to_string()));

            Self {
                monitor_name: monitor_name.to_string(),
                actions,
            }
        }

        fn record(&self, action: RecordedAction) {
            self.actions.lock().unwrap().push(action);
        }
    }

    impl BarLifecycle for FakeBar {
        fn setup_event_listener(&mut self) {
            self.record(RecordedAction::Setup(self.monitor_name.clone()));
        }

        fn present(&mut self) {
            self.record(RecordedAction::Present(self.monitor_name.clone()));
        }

        fn hide(&mut self) {
            self.record(RecordedAction::Hide(self.monitor_name.clone()));
        }

        fn rebuild_widgets(&mut self) {
            self.record(RecordedAction::Rebuild(self.monitor_name.clone()));
        }

        fn close(self) {
            self.actions
                .lock()
                .unwrap()
                .push(RecordedAction::Close(self.monitor_name));
        }
    }

    fn recorder() -> Arc<Mutex<Vec<RecordedAction>>> {
        Arc::new(Mutex::new(Vec::new()))
    }

    fn managed_fake_bar(
        monitor_name: &str,
        visible: bool,
        listeners_setup: bool,
        actions: Arc<Mutex<Vec<RecordedAction>>>,
    ) -> ManagedBar<FakeBar> {
        ManagedBar::new(
            FakeBar {
                monitor_name: monitor_name.to_string(),
                actions,
            },
            visible,
            listeners_setup,
        )
    }

    #[test]
    fn bar_reconciliation_defaults_all_monitors_enabled() {
        // Given
        let config = HyprlineConfig::default();
        let monitors = vec![monitor("DP-1", 1), monitor("HDMI-A-1", 2)];

        // When
        let desired = desired_bar_monitor_names(&monitors, &config);
        let plan = plan_bar_reconciliation(&[], &monitors, &config);

        // Then
        assert_eq!(desired, vec!["DP-1", "HDMI-A-1"]);
        assert_eq!(
            plan,
            BarReconciliationPlan {
                create: vec!["DP-1".to_string(), "HDMI-A-1".to_string()],
                ..BarReconciliationPlan::default()
            }
        );
    }

    #[test]
    fn bar_reconciliation_excludes_disabled_monitor() {
        // Given
        let mut config = HyprlineConfig::default();
        config.set_monitor_bar_enabled("HDMI-A-1", false);
        let monitors = vec![monitor("DP-1", 1), monitor("HDMI-A-1", 2)];

        // When
        let desired = desired_bar_monitor_names(&monitors, &config);
        let plan = plan_bar_reconciliation(&[], &monitors, &config);

        // Then
        assert_eq!(desired, vec!["DP-1"]);
        assert_eq!(plan.create, vec!["DP-1"]);
        assert!(plan.hide.is_empty());
        assert!(plan.show.is_empty());
        assert!(plan.rebuild.is_empty());
        assert!(plan.close_removed.is_empty());
    }

    #[test]
    fn bar_reconciliation_hides_existing_disabled_bar() {
        // Given
        let mut config = HyprlineConfig::default();
        config.set_monitor_bar_enabled("HDMI-A-1", false);
        let monitors = vec![monitor("DP-1", 1), monitor("HDMI-A-1", 2)];
        let existing = vec![visible_bar("DP-1"), visible_bar("HDMI-A-1")];

        // When
        let plan = plan_bar_reconciliation(&existing, &monitors, &config);

        // Then
        assert_eq!(plan.hide, vec!["HDMI-A-1"]);
        assert_eq!(plan.rebuild, vec!["DP-1"]);
        assert!(plan.create.is_empty());
        assert!(plan.show.is_empty());
        assert!(plan.close_removed.is_empty());
    }

    #[test]
    fn bar_reconciliation_creates_newly_enabled_missing_bar() {
        // Given
        let config = HyprlineConfig::default();
        let monitors = vec![monitor("DP-1", 1), monitor("HDMI-A-1", 2)];
        let existing = vec![visible_bar("DP-1")];

        // When
        let plan = plan_bar_reconciliation(&existing, &monitors, &config);

        // Then
        assert_eq!(plan.create, vec!["HDMI-A-1"]);
        assert_eq!(plan.rebuild, vec!["DP-1"]);
        assert!(plan.hide.is_empty());
        assert!(plan.show.is_empty());
        assert!(plan.close_removed.is_empty());
    }

    #[test]
    fn bar_reconciliation_default_fallback_respects_config() {
        // Given
        let mut disabled_default_config = HyprlineConfig::default();
        disabled_default_config.set_monitor_bar_enabled("default", false);
        let enabled_default_config = HyprlineConfig::default();
        let no_monitors = Vec::<Monitor>::new();

        // When
        let enabled_desired = desired_bar_monitor_names(&no_monitors, &enabled_default_config);
        let enabled_plan = plan_bar_reconciliation(
            &[hidden_bar("default")],
            &no_monitors,
            &enabled_default_config,
        );
        let disabled_desired = desired_bar_monitor_names(&no_monitors, &disabled_default_config);
        let disabled_plan = plan_bar_reconciliation(
            &[visible_bar("default")],
            &no_monitors,
            &disabled_default_config,
        );

        // Then
        assert_eq!(enabled_desired, vec!["default"]);
        assert_eq!(enabled_plan.show, vec!["default"]);
        assert!(enabled_plan.create.is_empty());
        assert_eq!(disabled_desired, Vec::<String>::new());
        assert_eq!(disabled_plan.close_removed, vec!["default"]);
        assert!(disabled_plan.hide.is_empty());
    }

    #[test]
    fn bar_reconciliation_monitor_added_preserves_existing_visible_bar() {
        // Given
        let config = HyprlineConfig::default();
        let monitors = vec![monitor("DP-1", 1), monitor("HDMI-A-1", 2)];
        let existing = vec![visible_bar("DP-1")];

        // When
        let plan = plan_bar_reconciliation(&existing, &monitors, &config);

        // Then
        assert_eq!(plan.rebuild, vec!["DP-1"]);
        assert_eq!(plan.create, vec!["HDMI-A-1"]);
        assert!(plan.hide.is_empty());
        assert!(plan.show.is_empty());
        assert!(plan.close_removed.is_empty());
    }

    #[test]
    fn bar_reconciliation_apply_records_hide_rebuild_create_actions() {
        // Given
        let actions = recorder();
        let mut managed_bars = ManagedBars::default();
        managed_bars.insert(
            "DP-1".to_string(),
            managed_fake_bar("DP-1", true, true, actions.clone()),
        );
        managed_bars.insert(
            "HDMI-A-1".to_string(),
            managed_fake_bar("HDMI-A-1", true, true, actions.clone()),
        );

        let mut config = HyprlineConfig::default();
        config.set_monitor_bar_enabled("HDMI-A-1", false);
        let monitors = vec![
            monitor("DP-1", 1),
            monitor("HDMI-A-1", 2),
            monitor("eDP-1", 3),
        ];
        let plan = plan_bar_reconciliation(&managed_bars.existing_states(), &monitors, &config);

        // When
        apply_bar_reconciliation_plan(&mut managed_bars, &plan, |monitor_name| {
            FakeBar::new(monitor_name, actions.clone())
        });

        // Then
        assert_eq!(
            *actions.lock().unwrap(),
            vec![
                RecordedAction::Hide("HDMI-A-1".to_string()),
                RecordedAction::Rebuild("DP-1".to_string()),
                RecordedAction::Create("eDP-1".to_string()),
                RecordedAction::Setup("eDP-1".to_string()),
                RecordedAction::Present("eDP-1".to_string()),
            ]
        );
    }

    #[test]
    fn bar_reconciliation_disable_enable_disable_no_duplicate_setup() {
        // Given
        let actions = recorder();
        let mut managed_bars = ManagedBars::default();
        let monitors = vec![monitor("HDMI-A-1", 1)];
        let enabled_config = HyprlineConfig::default();

        // When
        let create_plan =
            plan_bar_reconciliation(&managed_bars.existing_states(), &monitors, &enabled_config);
        apply_bar_reconciliation_plan(&mut managed_bars, &create_plan, |monitor_name| {
            FakeBar::new(monitor_name, actions.clone())
        });

        let mut disabled_config = enabled_config.clone();
        disabled_config.set_monitor_bar_enabled("HDMI-A-1", false);
        let disable_plan =
            plan_bar_reconciliation(&managed_bars.existing_states(), &monitors, &disabled_config);
        apply_bar_reconciliation_plan(&mut managed_bars, &disable_plan, |monitor_name| {
            FakeBar::new(monitor_name, actions.clone())
        });

        let enable_plan =
            plan_bar_reconciliation(&managed_bars.existing_states(), &monitors, &enabled_config);
        apply_bar_reconciliation_plan(&mut managed_bars, &enable_plan, |monitor_name| {
            FakeBar::new(monitor_name, actions.clone())
        });

        let disable_again_plan =
            plan_bar_reconciliation(&managed_bars.existing_states(), &monitors, &disabled_config);
        apply_bar_reconciliation_plan(&mut managed_bars, &disable_again_plan, |monitor_name| {
            FakeBar::new(monitor_name, actions.clone())
        });

        // Then
        let recorded_actions = actions.lock().unwrap().clone();
        assert_eq!(
            recorded_actions,
            vec![
                RecordedAction::Create("HDMI-A-1".to_string()),
                RecordedAction::Setup("HDMI-A-1".to_string()),
                RecordedAction::Present("HDMI-A-1".to_string()),
                RecordedAction::Hide("HDMI-A-1".to_string()),
                RecordedAction::Present("HDMI-A-1".to_string()),
                RecordedAction::Hide("HDMI-A-1".to_string()),
            ]
        );
        assert_eq!(
            recorded_actions
                .iter()
                .filter(|action| *action == &RecordedAction::Setup("HDMI-A-1".to_string()))
                .count(),
            1
        );
    }
}
