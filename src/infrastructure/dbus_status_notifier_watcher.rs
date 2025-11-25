use crate::domain::status_notifier_watcher_service::StatusNotifierWatcherService;
use zbus::{ConnectionBuilder, interface};
use std::sync::{Arc, Mutex};
use async_channel::Sender;
use futures::stream::StreamExt;

/// Внутреннее состояние StatusNotifierWatcher
#[derive(Clone)]
struct WatcherState {
    registered_items: Arc<Mutex<Vec<String>>>,
    item_registered_tx: Option<Sender<String>>,
    item_unregistered_tx: Option<Sender<String>>,
    shutdown_tx: Option<Sender<()>>,
}

impl WatcherState {
    fn new() -> Self {
        Self {
            registered_items: Arc::new(Mutex::new(Vec::new())),
            item_registered_tx: None,
            item_unregistered_tx: None,
            shutdown_tx: None,
        }
    }
}

/// D-Bus интерфейс для StatusNotifierWatcher
struct StatusNotifierWatcherInterface {
    state: WatcherState,
}

#[interface(name = "org.kde.StatusNotifierWatcher")]
impl StatusNotifierWatcherInterface {
    /// Регистрирует новый элемент системного трея
    async fn register_status_notifier_item(&mut self, #[zbus(signal_context)] ctx: zbus::SignalContext<'_>, service: String) -> zbus::fdo::Result<()> {
        let should_register = {
            let mut items = self.state.registered_items.lock().unwrap();

            // Добавляем только если ещё не зарегистрирован
            if !items.contains(&service) {
                items.push(service.clone());
                eprintln!("[StatusNotifierWatcher] Registered: {}", service);
                true
            } else {
                false
            }
        };

        if should_register {
            // Отправляем D-Bus сигнал
            self.status_notifier_item_registered(&ctx, &service).await?;
        }

        Ok(())
    }

    /// Отменяет регистрацию элемента системного трея
    async fn unregister_status_notifier_item(&mut self, #[zbus(signal_context)] ctx: zbus::SignalContext<'_>, service: String) -> zbus::fdo::Result<()> {
        let was_removed = {
            let mut items = self.state.registered_items.lock().unwrap();
            let before = items.len();
            items.retain(|item| item != &service);
            let after = items.len();

            if before != after {
                eprintln!("[StatusNotifierWatcher] Unregistered: {}", service);
            }
            before != after
        };

        if was_removed {
            // Отправляем D-Bus сигнал
            self.status_notifier_item_unregistered(&ctx, &service).await?;
        }

        Ok(())
    }

    /// Регистрирует новый StatusNotifierHost
    async fn register_status_notifier_host(&mut self, _service: String) -> zbus::fdo::Result<()> {
        eprintln!("[StatusNotifierWatcher] Host registered");
        Ok(())
    }

    /// Возвращает список зарегистрированных элементов
    #[zbus(property)]
    async fn registered_status_notifier_items(&self) -> Vec<String> {
        self.state.registered_items.lock().unwrap().clone()
    }

    /// Возвращает, является ли хост зарегистрированным
    #[zbus(property)]
    async fn is_status_notifier_host_registered(&self) -> bool {
        true
    }

    /// Версия протокола
    #[zbus(property)]
    async fn protocol_version(&self) -> i32 {
        0
    }

    /// Сигнал: новый элемент зарегистрирован
    #[zbus(signal)]
    async fn status_notifier_item_registered(
        &self,
        ctx: &zbus::SignalContext<'_>,
        service: &str,
    ) -> zbus::Result<()> {}

    /// Сигнал: элемент удалён
    #[zbus(signal)]
    async fn status_notifier_item_unregistered(
        &self,
        ctx: &zbus::SignalContext<'_>,
        service: &str,
    ) -> zbus::Result<()> {}

    /// Сигнал: хост зарегистрирован
    #[zbus(signal)]
    async fn status_notifier_host_registered(
        &self,
        ctx: &zbus::SignalContext<'_>,
    ) -> zbus::Result<()> {}
}

pub struct DbusStatusNotifierWatcher {
    state: Arc<Mutex<WatcherState>>,
    handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
}

impl DbusStatusNotifierWatcher {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(WatcherState::new())),
            handle: Arc::new(Mutex::new(None)),
        }
    }
}

impl StatusNotifierWatcherService for DbusStatusNotifierWatcher {
    fn start(&self) -> Result<(), String> {
        eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        eprintln!("[StatusNotifierWatcher] Starting built-in D-Bus service...");
        eprintln!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

        let (shutdown_tx, shutdown_rx) = async_channel::bounded(1);

        // Сохраняем канал для shutdown в оригинальном state
        {
            let mut state = self.state.lock().unwrap();
            state.shutdown_tx = Some(shutdown_tx);
        }

        let state = self.state.clone();

        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async move {
                let state_clone = state.lock().unwrap().clone();
                match start_watcher_async(state_clone, shutdown_rx).await {
                    Ok(_) => eprintln!("[StatusNotifierWatcher] Service stopped"),
                    Err(e) => eprintln!("[StatusNotifierWatcher] ERROR: {}", e),
                }
            });
        });

        *self.handle.lock().unwrap() = Some(handle);

        Ok(())
    }

    fn get_registered_items(&self) -> Vec<String> {
        self.state.lock().unwrap().registered_items.lock().unwrap().clone()
    }

    fn stop(&self) -> Result<(), String> {
        eprintln!("[StatusNotifierWatcher] Stopping service...");

        // Отправляем сигнал shutdown
        {
            let state = self.state.lock().unwrap();
            if let Some(tx) = &state.shutdown_tx {
                let _ = tx.try_send(());
            }
        }

        // Ждём завершения потока
        if let Some(handle) = self.handle.lock().unwrap().take() {
            let _ = handle.join();
        }

        eprintln!("[StatusNotifierWatcher] Service stopped");
        Ok(())
    }
}

async fn start_watcher_async(state: WatcherState, shutdown_rx: async_channel::Receiver<()>) -> Result<(), Box<dyn std::error::Error>> {
    // Создаём интерфейс
    let interface = StatusNotifierWatcherInterface {
        state: state.clone(),
    };

    // Регистрируем интерфейс на D-Bus
    let conn = ConnectionBuilder::session()?
        .name("org.kde.StatusNotifierWatcher")?
        .serve_at("/StatusNotifierWatcher", interface)?
        .build()
        .await?;

    eprintln!("[StatusNotifierWatcher] ✓ D-Bus service registered");
    eprintln!("[StatusNotifierWatcher] ✓ Service: org.kde.StatusNotifierWatcher");
    eprintln!("[StatusNotifierWatcher] ✓ Path: /StatusNotifierWatcher");
    eprintln!("[StatusNotifierWatcher] ✓ Ready to accept tray items");

    // Подписываемся на сигналы NameOwnerChanged для отслеживания исчезновения сервисов
    let dbus_proxy = zbus::fdo::DBusProxy::new(&conn).await?;
    let mut name_owner_changed_stream = dbus_proxy.receive_name_owner_changed().await?;

    eprintln!("[StatusNotifierWatcher] ✓ Monitoring D-Bus service lifecycle");

    // Слушаем сигналы об исчезновении сервисов
    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                eprintln!("[StatusNotifierWatcher] Received shutdown signal");
                break;
            }

            Some(signal) = name_owner_changed_stream.next() => {
                if let Ok(args) = signal.args() {
                    let name = args.name();
                    let old_owner = args.old_owner();
                    let new_owner = args.new_owner();

                    // Если сервис исчез (old_owner не пустой, new_owner пустой)
                    if old_owner.is_some() && new_owner.is_none() {
                        // Проверяем, был ли это один из наших зарегистрированных элементов
                        let mut items = state.registered_items.lock().unwrap();

                        // Ищем элементы, которые начинаются с этого имени сервиса
                        let mut removed_items = Vec::new();
                        items.retain(|item| {
                            // item может быть в формате "service" или "service/path"
                            let service_name = item.split('/').next().unwrap_or(item);
                            if service_name == name.as_str() {
                                removed_items.push(item.clone());
                                false
                            } else {
                                true
                            }
                        });

                        drop(items);

                        // Уведомляем об удалении каждого элемента
                        for removed_item in removed_items {
                            eprintln!("[StatusNotifierWatcher] Auto-removed: {}", removed_item);

                            // Получаем доступ к ObjectServer для отправки сигнала
                            let object_server = conn.object_server();
                            let iface_ref = object_server
                                .interface::<_, StatusNotifierWatcherInterface>("/StatusNotifierWatcher")
                                .await;

                            if let Ok(iface) = iface_ref {
                                // Отправляем D-Bus сигнал через интерфейс
                                let signal_ctx = iface.signal_context();
                                let _ = iface.get()
                                    .await
                                    .status_notifier_item_unregistered(signal_ctx, &removed_item)
                                    .await;
                            }
                        }
                    }
                }
            }
        }
    }

    // Освобождаем имя в D-Bus
    drop(conn);
    eprintln!("[StatusNotifierWatcher] D-Bus connection closed");

    Ok(())
}
