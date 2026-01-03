mod dbus_service;
mod notification;
mod repository;
mod ui;

use gtk4::glib;
use gtk4::prelude::*;
use std::sync::Arc;
use tokio::sync::Mutex;

use dbus_service::NotificationDbusService;
use repository::NotificationRepository;

fn main() {
    // Инициализация логирования
    tracing_subscriber::fmt::init();

    let app = gtk4::Application::builder()
        .application_id("ru.hyprline.notifications")
        .flags(gtk4::gio::ApplicationFlags::IS_SERVICE)
        .build();

    app.connect_startup(|app| {
        // Загружаем CSS
        let provider = gtk4::CssProvider::new();
        provider.load_from_data(include_str!("styles.css"));

        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("Could not get default display"),
            &provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // Держим приложение активным (это демон, не должен завершаться)
        // Guard хранится в static для предотвращения завершения
        let _guard = app.hold();
        std::mem::forget(_guard); // Намеренно "утекаем" guard чтобы держать app активным

        setup_service(app);
    });

    app.connect_activate(|_| {
        // Ничего не делаем при активации - это демон
    });

    app.run();
}

fn setup_service(app: &gtk4::Application) {
    let app = app.clone();

    // Создаём репозиторий
    let repository = Arc::new(Mutex::new(
        NotificationRepository::new().expect("Failed to create notification repository")
    ));

    // Создаём канал для уведомлений
    let (notification_tx, notification_rx) = async_channel::unbounded();

    // Создаём канал для событий UI (показ popup, обновление истории)
    let (ui_tx, ui_rx) = async_channel::unbounded();

    // Запускаем D-Bus сервис в отдельном потоке
    let repo_for_dbus = repository.clone();
    let ui_tx_for_dbus = ui_tx.clone();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = NotificationDbusService::start(
                repo_for_dbus,
                notification_tx,
                ui_tx_for_dbus,
            ).await {
                eprintln!("[NotificationService] Failed to start D-Bus service: {}", e);
            }
        });
    });

    // Обработка новых уведомлений - показываем popup
    let app_clone = app.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        while let Ok(notification) = notification_rx.try_recv() {
            ui::popup::show_notification_popup(&app_clone, notification);
        }
        glib::ControlFlow::Continue
    });

    // Обработка UI событий (например, запрос на показ истории)
    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        while let Ok(event) = ui_rx.try_recv() {
            match event {
                UiEvent::ShowHistory => {
                    // TODO: Показать окно истории
                    eprintln!("[UI] Show history requested");
                }
                UiEvent::HideHistory => {
                    eprintln!("[UI] Hide history requested");
                }
            }
        }
        glib::ControlFlow::Continue
    });

    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    eprintln!("[NotificationDaemon] Started");
    eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

#[derive(Debug, Clone)]
pub enum UiEvent {
    ShowHistory,
    HideHistory,
}

