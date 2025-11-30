mod config;
mod domain;
mod infrastructure;
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
use domain::status_notifier_watcher_service::StatusNotifierWatcherService;
use domain::models::DateTimeConfig;
use infrastructure::hyprland_ipc::HyprlandIpc;
use infrastructure::status_notifier_tray::StatusNotifierTrayService;
use infrastructure::system_datetime::SystemDateTimeService;
use infrastructure::system_battery::SystemBatteryService;
use infrastructure::system_resources::LinuxSystemResources;
use infrastructure::networkmanager::NetworkManagerService;
use infrastructure::dbus_status_notifier_watcher::DbusStatusNotifierWatcher;
use infrastructure::dbus_notification_service::DbusNotificationService;
use infrastructure::hyprland_keyboard_layout::HyprlandKeyboardLayoutService;
use ui::bar::Bar;
use ui::volume_osd::VolumeOsd;

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

    // Подписываемся на события громкости для отображения OSD
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

    // Создаём Notification сервис с мониторингом
    let (notification_tx, notification_rx) = infrastructure::dbus_notification_service::create_notification_channel();
    let notification_service_impl = Arc::new(DbusNotificationService::new());
    if let Err(e) = notification_service_impl.start(notification_tx) {
        eprintln!("[Main] Warning: Failed to start NotificationService: {}", e);
    }
    let notification_service: Arc<dyn NotificationService + Send + Sync> = notification_service_impl;

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
    
    let workspace_keys = parse_workspace_bindings();
    let monitors = service.get_monitors();

    if monitors.is_empty() {
        let bar = Bar::new(
            app, 
            "default", 
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
        );
        bar.setup_event_listener(tray_rx, volume_rx, notification_rx, keyboard_layout_rx, battery_rx);
        bar.present();
        return;
    }

    for monitor in &monitors {
        let bar = Bar::new(
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
        );
        bar.setup_event_listener(
            tray_rx.clone(), 
            volume_rx.clone(), 
            notification_rx.clone(),
            keyboard_layout_rx.clone(),
            battery_rx.clone(),
        );
        bar.present();
    }
}
