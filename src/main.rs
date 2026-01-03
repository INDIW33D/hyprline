mod config;
mod domain;
mod infrastructure;
mod shared_state;
mod ui;

use config::parse_workspace_bindings;
use domain::workspace_service::WorkspaceService;
use domain::system_tray_service::SystemTrayService;
use domain::datetime_service::DateTimeService;
use domain::battery_service::BatteryService;
use domain::volume_service::VolumeService;
use domain::notification_service::NotificationService;
use domain::keyboard_layout_service::KeyboardLayoutService;
use domain::system_resources_service::SystemResourcesService;
use domain::network_service::NetworkService;
use domain::brightness_service::BrightnessService;
use domain::status_notifier_watcher_service::StatusNotifierWatcherService;
use domain::models::DateTimeConfig;
use infrastructure::hyprland_ipc::HyprlandIpc;
use infrastructure::status_notifier_tray::StatusNotifierTrayService;
use infrastructure::system_datetime::SystemDateTimeService;
use infrastructure::system_battery::SystemBatteryService;
use infrastructure::system_resources::LinuxSystemResources;
use infrastructure::networkmanager::NetworkManagerService;
use infrastructure::dbus_status_notifier_watcher::DbusStatusNotifierWatcher;
use infrastructure::remote_notification_service::RemoteNotificationService;
use infrastructure::hyprland_keyboard_layout::HyprlandKeyboardLayoutService;
use infrastructure::lumen_brightness::LumenBrightnessService;
use infrastructure::monitor_listener::{start_monitor_listener, MonitorEvent};
use ui::bar::Bar;
use ui::volume_osd::VolumeOsd;
use shared_state::get_shared_state;

use gtk4::prelude::*;
use gtk4::{gdk, glib};
use std::sync::Arc;

fn main() -> glib::ExitCode {
    let app = gtk4::Application::builder()
        .application_id("ru.hyprline")
        .build();

    app.connect_startup(|app| {
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(include_str!("styles.css"));

        gtk4::style_context_add_provider_for_display(
            &gdk::Display::default().expect("error initializing gtk4 style context"),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

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
        _ => short_name.chars().take(2).collect::<String>().to_uppercase(),
    }
}

fn build_ui(app: &gtk4::Application) {
    // Запускаем свой StatusNotifierWatcher D-Bus сервис
    let watcher_service = Arc::new(DbusStatusNotifierWatcher::new());
    if let Err(e) = watcher_service.start() {
        eprintln!("[Main] Warning: Failed to start StatusNotifierWatcher: {}", e);
    }
    
    // Даём время сервису зарегистрироваться в D-Bus
    std::thread::sleep(std::time::Duration::from_millis(200));
    
    let service: Arc<dyn WorkspaceService + Send + Sync> = Arc::new(HyprlandIpc::new());
    
    // Создаём системный трей сервис
    let tray_service_impl = Arc::new(StatusNotifierTrayService::new());
    let tray_service: Arc<dyn SystemTrayService + Send + Sync> = tray_service_impl.clone();
    
    // Создаём DateTime сервис
    let datetime_service: Arc<dyn DateTimeService + Send + Sync> = Arc::new(SystemDateTimeService::new());
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
    let (keyboard_layout_tx, keyboard_layout_rx) = infrastructure::keyboard_layout_listener::create_keyboard_layout_channel();
    
    // Запускаем мониторинг событий раскладки
    infrastructure::keyboard_layout_listener::start_keyboard_layout_listener(keyboard_layout_tx);

    // Создаём SystemResources сервис
    let system_resources_service: Arc<dyn SystemResourcesService + Send + Sync> =
        Arc::new(LinuxSystemResources::new());

    // Создаём Network сервис
    let network_service: Arc<dyn NetworkService + Send + Sync> =
        Arc::new(NetworkManagerService::new());

    // Создаём Brightness сервис
    let brightness_service: Arc<dyn BrightnessService + Send + Sync> = match LumenBrightnessService::new() {
        Ok(service) => {
            let service_arc = Arc::new(service);

            // Пробуем получить текущую яркость для проверки
            if let Ok(brightness) = service_arc.get_brightness() {
                eprintln!("[Brightness] ✓ Connected ({}%)", brightness);
            }

            // Запускаем мониторинг сигналов яркости
            service_arc.clone().start_signal_monitoring();
            service_arc
        }
        Err(e) => {
            eprintln!("[Brightness] ✗ Failed to connect: {}", e);
            eprintln!("[Brightness] Make sure Lumen service is running");
            panic!("Cannot create brightness service");
        }
    };

    // Подписываемся на изменения яркости и будем обновлять SharedState
    let shared_state_brightness = get_shared_state();
    brightness_service.subscribe_brightness_changed(Arc::new(move |value| {
        shared_state_brightness.update_brightness(value);
    }));

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
            eprintln!("[Main] Warning: Failed to stop StatusNotifierWatcher: {}", e);
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
    if let Ok(brightness) = brightness_service.get_brightness() {
        shared_state.update_brightness(brightness);
    }
    // Инициализация системных ресурсов
    shared_state.update_system_resources(system_resources_service.get_resources());
    // Инициализация сети
    shared_state.update_network(network_service.get_current_connection());

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
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while let Ok(event) = rx.try_recv() {
                match event {
                    NotificationEvent::CountChanged(count) => {
                        shared_state_for_listener.update_notifications(count as usize);
                    }
                    NotificationEvent::ServiceAvailable => {
                        eprintln!("[Main] Notification service connected");
                        shared_state_for_listener.set_notification_service_available(true);
                    }
                    NotificationEvent::ServiceUnavailable => {
                        eprintln!("[Main] Notification service disconnected");
                        shared_state_for_listener.set_notification_service_available(false);
                        shared_state_for_listener.update_notifications(0);
                    }
                }
            }
            glib::ControlFlow::Continue
        });
    }

    let workspace_keys = parse_workspace_bindings();
    let monitors = service.get_monitors();

    // Создаём bars и храним их для hot reload и динамического управления
    let bars: Arc<std::sync::Mutex<Vec<Bar>>> = Arc::new(std::sync::Mutex::new(
        if monitors.is_empty() {
            vec![Bar::new(
                app,
                "default",
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
                brightness_service.clone(),
                shared_state.clone(),
            )]
        } else {
            monitors.iter().map(|monitor| {
                Bar::new(
                    app,
                    &monitor.name,
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
                    brightness_service.clone(),
                    shared_state.clone(),
                )
            }).collect()
        }
    ));

    // Подписка на изменения конфигурации для hot reload
    {
        let bars_for_config = bars.clone();
        let (config_tx, config_rx) = async_channel::unbounded::<()>();

        config::subscribe_config_changes(move || {
            let _ = config_tx.send_blocking(());
        });

        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while config_rx.try_recv().is_ok() {
                eprintln!("[Main] Config changed, rebuilding widgets...");
                let mut bars = bars_for_config.lock().unwrap();
                for bar in bars.iter_mut() {
                    bar.rebuild_widgets();
                }
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
        let app_clone = app.clone();
        let workspace_keys_clone = workspace_keys.clone();
        let service_clone = service.clone();
        let tray_service_clone = tray_service.clone();
        let datetime_service_clone = datetime_service.clone();
        let datetime_config_clone = datetime_config.clone();
        let battery_service_clone = battery_service.clone();
        let volume_service_clone = volume_service.clone();
        let notification_service_clone = notification_service.clone();
        let keyboard_layout_service_clone = keyboard_layout_service.clone();
        let system_resources_service_clone = system_resources_service.clone();
        let network_service_clone = network_service.clone();
        let brightness_service_clone = brightness_service.clone();
        let shared_state_clone = shared_state.clone();

        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while let Ok(event) = monitor_rx.try_recv() {
                match event {
                    MonitorEvent::Added(monitor_name) => {
                        // Задержка, чтобы GDK успел зарегистрировать новый монитор
                        let bars_clone = bars_for_monitors.clone();
                        let app = app_clone.clone();
                        let workspace_keys = workspace_keys_clone.clone();
                        let service = service_clone.clone();
                        let tray_service = tray_service_clone.clone();
                        let datetime_service = datetime_service_clone.clone();
                        let datetime_config = datetime_config_clone.clone();
                        let battery_service = battery_service_clone.clone();
                        let volume_service = volume_service_clone.clone();
                        let notification_service = notification_service_clone.clone();
                        let keyboard_layout_service = keyboard_layout_service_clone.clone();
                        let system_resources_service = system_resources_service_clone.clone();
                        let network_service = network_service_clone.clone();
                        let brightness_service = brightness_service_clone.clone();
                        let shared_state = shared_state_clone.clone();

                        glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
                            let mut bars = bars_clone.lock().unwrap();

                            // Проверяем, нет ли уже бара для этого монитора
                            if bars.iter().any(|b| b.monitor_name() == monitor_name) {
                                eprintln!("[Main] Bar for monitor {} already exists", monitor_name);
                                return;
                            }

                            eprintln!("[Main] Creating bar for new monitor: {}", monitor_name);

                            let bar = Bar::new(
                                &app,
                                &monitor_name,
                                workspace_keys,
                                service,
                                tray_service,
                                datetime_service,
                                datetime_config,
                                battery_service,
                                volume_service,
                                notification_service,
                                keyboard_layout_service,
                                system_resources_service,
                                network_service,
                                brightness_service,
                                shared_state,
                            );

                            bar.setup_event_listener();
                            bar.present();
                            bars.push(bar);

                            eprintln!("[Main] ✓ Bar created for monitor: {}", monitor_name);
                        });
                    }
                    MonitorEvent::Removed(monitor_name) => {
                        let mut bars = bars_for_monitors.lock().unwrap();

                        // Находим и удаляем бар для этого монитора
                        if let Some(pos) = bars.iter().position(|b| b.monitor_name() == monitor_name) {
                            eprintln!("[Main] Removing bar for monitor: {}", monitor_name);
                            let bar = bars.remove(pos);
                            bar.close();
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
        for bar in bars.iter() {
            bar.setup_event_listener();
            bar.present();
        }
    }
}
