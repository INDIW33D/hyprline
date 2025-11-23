mod config;
mod domain;
mod infrastructure;
mod ui;

use config::parse_workspace_bindings;
use domain::workspace_service::WorkspaceService;
use domain::system_tray_service::SystemTrayService;
use domain::datetime_service::DateTimeService;
use domain::models::DateTimeConfig;
use infrastructure::hyprland_ipc::HyprlandIpc;
use infrastructure::status_notifier_tray::StatusNotifierTrayService;
use infrastructure::system_datetime::SystemDateTimeService;
use ui::bar::Bar;

use gtk4::prelude::*;
use gtk4::{gdk, glib};
use std::sync::Arc;

fn main() -> glib::ExitCode {
    let app = gtk4::Application::builder()
        .application_id("ru.hyprline")
        .build();

    app.connect_startup(|app| {
        let provider = gtk4::CssProvider::new();
        provider.load_from_path("src/styles.css");

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
    let service: Arc<dyn WorkspaceService + Send + Sync> = Arc::new(HyprlandIpc::new());
    
    // Создаём системный трей сервис
    let tray_service_impl = Arc::new(StatusNotifierTrayService::new());
    let tray_service: Arc<dyn SystemTrayService + Send + Sync> = tray_service_impl.clone();
    
    // Создаём DateTime сервис
    let datetime_service: Arc<dyn DateTimeService + Send + Sync> = Arc::new(SystemDateTimeService::new());
    let datetime_config = DateTimeConfig::default();
    
    // Создаём канал для обновлений трея
    let (tray_tx, tray_rx) = async_channel::unbounded();
    
    // Запускаем мониторинг трея
    tray_service_impl.start_monitoring(tray_tx.clone());
    
    let workspace_keys = parse_workspace_bindings();
    let monitors = service.get_monitors();

    if monitors.is_empty() {
        let bar = Bar::new(app, "default", workspace_keys, service, tray_service, datetime_service, datetime_config);
        bar.setup_event_listener(tray_rx);
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
        );
        bar.setup_event_listener(tray_rx.clone());
        bar.present();
    }
}

