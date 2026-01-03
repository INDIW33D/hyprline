use crate::domain::models::{TrayItem, TrayStatus};
use crate::domain::system_tray_service::{SystemTrayService, TrayUpdate};
use std::sync::{Arc, Mutex};
use zbus::{proxy, Connection};
use async_channel::Sender;
use futures::stream::StreamExt;

// DBus proxy для StatusNotifierWatcher
#[proxy(
    interface = "org.kde.StatusNotifierWatcher",
    default_service = "org.kde.StatusNotifierWatcher",
    default_path = "/StatusNotifierWatcher"
)]
trait StatusNotifierWatcher {
    #[zbus(property)]
    fn registered_status_notifier_items(&self) -> zbus::Result<Vec<String>>;

    /// Сигнал: новый элемент зарегистрирован
    #[zbus(signal)]
    fn status_notifier_item_registered(&self, service: &str) -> zbus::Result<()>;

    /// Сигнал: элемент удалён
    #[zbus(signal)]
    fn status_notifier_item_unregistered(&self, service: &str) -> zbus::Result<()>;
}

// DBus proxy для StatusNotifierItem
#[proxy(
    interface = "org.kde.StatusNotifierItem",
    assume_defaults = true
)]
trait StatusNotifierItem {
    #[zbus(property)]
    fn icon_name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn icon_pixmap(&self) -> zbus::Result<Vec<(i32, i32, Vec<u8>)>>;

    #[zbus(property)]
    fn icon_theme_path(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn attention_icon_name(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn title(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn status(&self) -> zbus::Result<String>;

    #[zbus(property)]
    fn menu(&self) -> zbus::Result<zbus::zvariant::OwnedObjectPath>;

    fn activate(&self, x: i32, y: i32) -> zbus::Result<()>;

    fn secondary_activate(&self, x: i32, y: i32) -> zbus::Result<()>;
}

// DBus proxy для DBusMenu
#[proxy(
    interface = "com.canonical.dbusmenu",
    assume_defaults = true
)]
trait DBusMenu {
    /// Получить layout меню
    /// Возвращает: (revision, layout) где layout = (id, properties, children)
    fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: Vec<&str>,
    ) -> zbus::Result<(u32, (i32, std::collections::HashMap<String, zbus::zvariant::OwnedValue>, Vec<zbus::zvariant::OwnedValue>))>;

    /// Вызвать событие на элементе меню
    fn event(&self, id: i32, event_id: &str, data: zbus::zvariant::Value<'_>, timestamp: u32) -> zbus::Result<()>;
}

pub struct StatusNotifierTrayService {
    items: Arc<Mutex<Vec<TrayItem>>>,
    handle: Arc<Mutex<Option<std::thread::JoinHandle<()>>>>,
    shutdown_tx: Arc<Mutex<Option<async_channel::Sender<()>>>>,
}

impl StatusNotifierTrayService {
    pub fn new() -> Self {
        Self {
            items: Arc::new(Mutex::new(Vec::new())),
            handle: Arc::new(Mutex::new(None)),
            shutdown_tx: Arc::new(Mutex::new(None)),
        }
    }

    async fn monitor_tray_async(
        items: Arc<std::sync::Mutex<Vec<TrayItem>>>,
        tx: Sender<TrayUpdate>,
        shutdown_rx: async_channel::Receiver<()>,
    ) -> zbus::Result<()> {
        eprintln!("[Tray] Starting system tray monitoring...");

        // Подключаемся к session bus
        let connection = match Connection::session().await {
            Ok(conn) => {
                eprintln!("[Tray] ✓ Connected to D-Bus session bus");
                conn
            }
            Err(e) => {
                eprintln!("[Tray] ERROR: Failed to connect to D-Bus: {}", e);
                return Err(e);
            }
        };

        // Создаём proxy для StatusNotifierWatcher
        let watcher = match StatusNotifierWatcherProxy::new(&connection).await {
            Ok(w) => {
                eprintln!("[Tray] ✓ Connected to StatusNotifierWatcher");
                w
            }
            Err(e) => {
                eprintln!("[Tray] ERROR: StatusNotifierWatcher not available: {}", e);
                eprintln!("[Tray] This should not happen with built-in watcher!");
                return Err(e);
            }
        };

        // Получаем начальный список зарегистрированных элементов
        let registered_items = match watcher.registered_status_notifier_items().await {
            Ok(items) => {
                eprintln!("[Tray] Found {} registered tray items", items.len());
                for item in &items {
                    eprintln!("[Tray]   - {}", item);
                }
                items
            }
            Err(e) => {
                eprintln!("[Tray] ERROR: Failed to get registered items: {}", e);
                Vec::new()
            }
        };

        let mut tray_items = Vec::new();

        for service in registered_items {
            if let Ok(item) = Self::fetch_tray_item(&connection, &service).await {
                eprintln!("[Tray] Found existing item: {} ({})", item.title, item.service);
                tray_items.push(item);
            }
        }

        // Сохраняем элементы
        *items.lock().unwrap() = tray_items.clone();

        // Отправляем в UI поток начальный список (даже если пустой)
        eprintln!("[Tray] Sending {} initial items to UI", tray_items.len());
        let _ = tx.send(tray_items.clone()).await;

        // Подписываемся на сигналы
        let mut registered_stream = watcher.receive_status_notifier_item_registered().await?;
        let mut unregistered_stream = watcher.receive_status_notifier_item_unregistered().await?;

        // Слушаем сигналы в бесконечном цикле
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    eprintln!("[Tray] Received shutdown signal");
                    break;
                }

                // Новый элемент зарегистрирован
                Some(signal) = registered_stream.next() => {
                    if let Ok(args) = signal.args() {
                        let service = args.service;

                        // Добавляем новый элемент
                        if let Ok(item) = Self::fetch_tray_item(&connection, service).await {
                            let mut items_guard = items.lock().unwrap();

                            // Проверяем, нет ли уже такого элемента
                            if !items_guard.iter().any(|i| i.service == item.service) {
                                eprintln!("[Tray] Added: {} ({})", item.title, item.service);
                                items_guard.push(item);
                                let updated = items_guard.clone();
                                drop(items_guard);

                                // Отправляем обновлённый список
                                let _ = tx.send(updated).await;
                            }
                        }
                    }
                }

                // Элемент удалён
                Some(signal) = unregistered_stream.next() => {
                    if let Ok(args) = signal.args() {
                        let service = args.service;

                        // Удаляем элемент
                        let mut items_guard = items.lock().unwrap();
                        items_guard.retain(|i| {
                            let keep = i.service != service;
                            if !keep {
                                eprintln!("[Tray] Removed: {} ({})", i.title, i.service);
                            }
                            keep
                        });
                        let updated = items_guard.clone();
                        drop(items_guard);

                        // Отправляем обновлённый список
                        let _ = tx.send(updated).await;
                    }
                }
            }
        }

        Ok(())
    }

    async fn fetch_tray_item(
        connection: &Connection,
        service: &str,
    ) -> zbus::Result<TrayItem> {
        // Парсим service (формат: "service_name" или "service_name/path")
        let (service_name, path) = if let Some(pos) = service.find('/') {
            let (name, p) = service.split_at(pos);
            (name, p.to_string())
        } else {
            (service, "/StatusNotifierItem".to_string())
        };

        // Создаём proxy для элемента
        let item_proxy = StatusNotifierItemProxy::builder(connection)
            .destination(service_name)?
            .path(path)?
            .build()
            .await?;

        // Получаем базовые данные
        let title = item_proxy.title().await.unwrap_or_else(|_| service_name.to_string());
        let status_str = item_proxy.status().await.unwrap_or_else(|_| "Active".to_string());

        let status = match status_str.as_str() {
            "Passive" => TrayStatus::Passive,
            "NeedsAttention" => TrayStatus::NeedsAttention,
            _ => TrayStatus::Active,
        };

        // Получаем иконку с приоритетом:
        // 1. AttentionIconName (если NeedsAttention)
        // 2. IconName
        // 3. IconPixmap
        // 4. Fallback

        let mut icon_name = String::new();
        let mut icon_pixmap = None;
        let mut icon_theme_path = None;

        // Проверяем attention icon если нужно
        if status == TrayStatus::NeedsAttention {
            if let Ok(attention_name) = item_proxy.attention_icon_name().await {
                if !attention_name.is_empty() {
                    icon_name = attention_name;
                }
            }
        }

        // Пробуем получить IconName
        if icon_name.is_empty() {
            if let Ok(name) = item_proxy.icon_name().await {
                icon_name = name;
            }
        }

        // Получаем IconThemePath если есть
        if let Ok(theme_path) = item_proxy.icon_theme_path().await {
            if !theme_path.is_empty() {
                icon_theme_path = Some(theme_path);
            }
        }

        // Получаем IconPixmap как fallback
        if icon_name.is_empty() || icon_name == "application-x-executable" {
            if let Ok(pixmap) = item_proxy.icon_pixmap().await {
                if !pixmap.is_empty() {
                    icon_pixmap = Some(pixmap);
                }
            }
        }

        // Получаем путь к меню если есть
        let menu_path = item_proxy.menu().await.ok().map(|p| p.to_string());

        Ok(TrayItem {
            service: service.to_string(),
            icon_name,
            icon_pixmap,
            icon_theme_path,
            menu_path,
            title,
            status,
        })
    }

    pub async fn activate_item_async(service: &str) {
        if let Ok(connection) = Connection::session().await {
            let (service_name, path) = if let Some(pos) = service.find('/') {
                let (name, p) = service.split_at(pos);
                (name, p.to_string())
            } else {
                (service, "/StatusNotifierItem".to_string())
            };

            if let Ok(proxy_builder) = StatusNotifierItemProxy::builder(&connection)
                .destination(service_name)
            {
                if let Ok(proxy_builder) = proxy_builder.path(path) {
                    if let Ok(item_proxy) = proxy_builder.build().await {
                        let _ = item_proxy.activate(0, 0).await;
                    }
                }
            }
        }
    }

    pub async fn secondary_activate_item_async(service: &str) {
        if let Ok(connection) = Connection::session().await {
            let (service_name, path) = if let Some(pos) = service.find('/') {
                let (name, p) = service.split_at(pos);
                (name, p.to_string())
            } else {
                (service, "/StatusNotifierItem".to_string())
            };

            if let Ok(proxy_builder) = StatusNotifierItemProxy::builder(&connection)
                .destination(service_name)
            {
                if let Ok(proxy_builder) = proxy_builder.path(path) {
                    if let Ok(item_proxy) = proxy_builder.build().await {
                        let _ = item_proxy.secondary_activate(0, 0).await;
                    }
                }
            }
        }
    }

    async fn get_menu_async(service: &str, menu_path: &str) -> zbus::Result<Vec<crate::domain::models::MenuItem>> {
        let connection = Connection::session().await?;

        let (service_name, _) = if let Some(pos) = service.find('/') {
            let (name, p) = service.split_at(pos);
            (name, p.to_string())
        } else {
            (service, "/StatusNotifierItem".to_string())
        };

        // Создаём proxy для DBusMenu
        let menu_proxy = DBusMenuProxy::builder(&connection)
            .destination(service_name)?
            .path(menu_path)?
            .build()
            .await?;

        // Получаем layout меню (parent_id=0 для root, recursion_depth=-1 для полной рекурсии)
        let (_, layout) = menu_proxy.get_layout(0, -1, vec![]).await?;

        // Парсим layout - теперь это кортеж (id, properties, children)
        let items = Self::parse_menu_from_tuple(&layout)?;

        Ok(items)
    }

    fn parse_menu_from_tuple(
        layout: &(i32, std::collections::HashMap<String, zbus::zvariant::OwnedValue>, Vec<zbus::zvariant::OwnedValue>)
    ) -> zbus::Result<Vec<crate::domain::models::MenuItem>> {
        use crate::domain::models::MenuItem;
        let mut items = Vec::new();

        let (_id, _properties, children) = layout;

        for child in children.iter() {
            if let Some(item) = Self::parse_menu_item(child) {
                items.push(item);
            }
        }

        Ok(items)
    }

    fn parse_menu_item(value: &zbus::zvariant::OwnedValue) -> Option<crate::domain::models::MenuItem> {
        use crate::domain::models::MenuItem;

        // Каждый child это структура (id, properties, children)
        let child_struct = value.downcast_ref::<zbus::zvariant::Structure>().ok()?;
        let fields = child_struct.fields();

        if fields.len() < 2 {
            return None;
        }

        // id
        let id = fields[0].downcast_ref::<i32>().unwrap_or(0);

        // properties (Dict)
        let props = fields[1].downcast_ref::<zbus::zvariant::Dict>().ok()?;

        let mut label = String::new();
        let mut enabled = true;
        let mut visible = true;
        let mut is_separator = false;
        let mut toggle_type: Option<String> = None;
        let mut toggle_state: i32 = -1;
        let mut icon_name: Option<String> = None;
        let mut icon_data: Option<Vec<u8>> = None;

        for (key_val, value_val) in props.iter() {
            if let Ok(key_str) = key_val.downcast_ref::<&str>() {
                match key_str {
                    "label" => {
                        if let Ok(s) = value_val.downcast_ref::<&str>() {
                            label = s.to_string();
                        }
                    }
                    "enabled" => {
                        if let Ok(b) = value_val.downcast_ref::<bool>() {
                            enabled = b;
                        }
                    }
                    "visible" => {
                        if let Ok(b) = value_val.downcast_ref::<bool>() {
                            visible = b;
                        }
                    }
                    "type" => {
                        if let Ok(s) = value_val.downcast_ref::<&str>() {
                            is_separator = s == "separator";
                        }
                    }
                    "toggle-type" => {
                        if let Ok(s) = value_val.downcast_ref::<&str>() {
                            if !s.is_empty() {
                                toggle_type = Some(s.to_string());
                            }
                        }
                    }
                    "toggle-state" => {
                        if let Ok(state) = value_val.downcast_ref::<i32>() {
                            toggle_state = state;
                        }
                    }
                    "icon-name" => {
                        if let Ok(s) = value_val.downcast_ref::<&str>() {
                            if !s.is_empty() {
                                icon_name = Some(s.to_string());
                            }
                        }
                    }
                    "icon-data" => {
                        // icon-data передаётся как массив байтов (PNG данные)
                        // Пробуем разные варианты типов
                        if let Ok(array) = value_val.downcast_ref::<zbus::zvariant::Array>() {
                            let mut bytes = Vec::new();
                            for item in array.iter() {
                                if let Ok(byte) = item.downcast_ref::<u8>() {
                                    bytes.push(byte);
                                }
                            }
                            if !bytes.is_empty() {
                                icon_data = Some(bytes);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // Парсим дочерние элементы если есть
        let mut children_items = Vec::new();
        if fields.len() >= 3 {
            if let Ok(children_array) = fields[2].downcast_ref::<zbus::zvariant::Array>() {
                for child_val in children_array.iter() {
                    // Конвертируем Value в OwnedValue правильно
                    if let Ok(owned) = zbus::zvariant::OwnedValue::try_from(child_val.clone()) {
                        if let Some(child_item) = Self::parse_menu_item(&owned) {
                            children_items.push(child_item);
                        }
                    }
                }
            }
        }

        Some(MenuItem {
            id,
            label,
            enabled,
            visible,
            is_separator,
            toggle_type,
            toggle_state,
            icon_name,
            icon_data,
            children: children_items,
        })
    }

    async fn activate_menu_item_async(service: &str, menu_path: &str, item_id: i32) {
        if let Ok(connection) = Connection::session().await {
            let (service_name, _) = if let Some(pos) = service.find('/') {
                let (name, p) = service.split_at(pos);
                (name, p.to_string())
            } else {
                (service, "/StatusNotifierItem".to_string())
            };

            if let Ok(builder) = DBusMenuProxy::builder(&connection)
                .destination(service_name)
                .and_then(|b| b.path(menu_path))
            {
                if let Ok(menu_proxy) = builder.build().await {
                    let empty_data = zbus::zvariant::Value::from(0i32);
                    let _ = menu_proxy.event(item_id, "clicked", empty_data, 0).await;
                }
            }
        }
    }
}

impl SystemTrayService for StatusNotifierTrayService {
    fn get_items(&self) -> Vec<TrayItem> {
        self.items.lock().unwrap().clone()
    }

    fn start_monitoring(&self, tx: async_channel::Sender<TrayUpdate>) {
        let items = self.items.clone();
        let (shutdown_tx, shutdown_rx) = async_channel::bounded::<()>(1);

        *self.shutdown_tx.lock().unwrap() = Some(shutdown_tx);

        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async move {
                if let Err(e) = StatusNotifierTrayService::monitor_tray_async(items, tx, shutdown_rx).await {
                    eprintln!("System tray monitoring error: {}", e);
                }
            });
        });

        *self.handle.lock().unwrap() = Some(handle);
    }

    fn stop(&self) {
        eprintln!("[SystemTray] Stopping monitoring...");

        if let Some(tx) = self.shutdown_tx.lock().unwrap().take() {
            let _ = tx.try_send(());
        }

        if let Some(handle) = self.handle.lock().unwrap().take() {
            let _ = handle.join();
        }

        self.items.lock().unwrap().clear();

        eprintln!("[SystemTray] Monitoring stopped");
    }

    fn activate_item(&self, service: &str) {
        let service = service.to_string();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(StatusNotifierTrayService::activate_item_async(&service));
        });
    }

    fn secondary_activate_item(&self, service: &str) {
        let service = service.to_string();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(StatusNotifierTrayService::secondary_activate_item_async(&service));
        });
    }

    fn get_menu(&self, service: &str, menu_path: &str, callback: Box<dyn Fn(Vec<crate::domain::models::MenuItem>) + Send + 'static>) {
        let service = service.to_string();
        let menu_path = menu_path.to_string();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(async {
                match Self::get_menu_async(&service, &menu_path).await {
                    Ok(items) => {
                        callback(items);
                    }
                    Err(_) => {
                        // Вызываем callback с пустым списком чтобы UI обновился
                        callback(Vec::new());
                    }
                }
            });
        });
    }

    fn activate_menu_item(&self, service: &str, menu_path: &str, item_id: i32) {
        let service = service.to_string();
        let menu_path = menu_path.to_string();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            rt.block_on(Self::activate_menu_item_async(&service, &menu_path, item_id));
        });
    }
}
