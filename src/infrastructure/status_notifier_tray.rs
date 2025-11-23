use crate::domain::models::{TrayItem, TrayStatus};
use crate::domain::system_tray_service::SystemTrayService;
use std::sync::Arc;
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
    items: Arc<std::sync::Mutex<Vec<TrayItem>>>,
}

impl StatusNotifierTrayService {
    pub fn new() -> Self {
        Self {
            items: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Запускает мониторинг трея в фоновом потоке
    pub fn start_monitoring(&self, tx: Sender<Vec<TrayItem>>) {
        let items = self.items.clone();

        std::thread::spawn(move || {
            // Создаём async runtime
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            rt.block_on(async move {
                if let Err(e) = Self::monitor_tray_async(items, tx).await {
                    eprintln!("System tray monitoring error: {}", e);
                }
            });
        });
    }

    async fn monitor_tray_async(
        items: Arc<std::sync::Mutex<Vec<TrayItem>>>,
        tx: Sender<Vec<TrayItem>>,
    ) -> zbus::Result<()> {
        // Подключаемся к session bus
        let connection = Connection::session().await?;

        // Создаём proxy для StatusNotifierWatcher
        let watcher = StatusNotifierWatcherProxy::new(&connection).await?;

        // Получаем начальный список зарегистрированных элементов
        let registered_items = watcher.registered_status_notifier_items().await?;

        let mut tray_items = Vec::new();

        for service in registered_items {
            if let Ok(item) = Self::fetch_tray_item(&connection, &service).await {
                tray_items.push(item);
            }
        }

        // Сохраняем элементы
        *items.lock().unwrap() = tray_items.clone();

        // Отправляем в UI поток
        let _ = tx.send(tray_items.clone()).await;

        // Подписываемся на сигналы
        let mut registered_stream = watcher.receive_status_notifier_item_registered().await?;
        let mut unregistered_stream = watcher.receive_status_notifier_item_unregistered().await?;

        // Слушаем сигналы в бесконечном цикле
        loop {
            tokio::select! {
                // Новый элемент зарегистрирован
                Some(signal) = registered_stream.next() => {
                    if let Ok(args) = signal.args() {
                        let service = args.service;
                        eprintln!("[Tray] Item registered: {}", service);

                        // Добавляем новый элемент
                        if let Ok(item) = Self::fetch_tray_item(&connection, service).await {
                            let mut items_guard = items.lock().unwrap();

                            // Проверяем, нет ли уже такого элемента
                            if !items_guard.iter().any(|i| i.service == item.service) {
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
                        eprintln!("[Tray] Item unregistered: {}", service);

                        // Удаляем элемент
                        let mut items_guard = items.lock().unwrap();
                        items_guard.retain(|i| i.service != service);
                        let updated = items_guard.clone();
                        drop(items_guard);

                        // Отправляем обновлённый список
                        let _ = tx.send(updated).await;
                    }
                }
            }
        }
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

        // Получаем layout меню (parent_id=0 для root, recursion_depth=1 для одного уровня)
        let (_, layout) = menu_proxy.get_layout(0, 1, vec![]).await?;

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
            // Каждый child это тоже структура (id, properties, children)
            if let Ok(child_struct) = child.downcast_ref::<zbus::zvariant::Structure>() {
                let fields = child_struct.fields();

                if fields.len() >= 2 {
                    // id
                    let id = if let Ok(id_val) = fields[0].downcast_ref::<i32>() {
                        id_val
                    } else {
                        0
                    };

                    // properties (Dict)
                    if let Ok(props) = fields[1].downcast_ref::<zbus::zvariant::Dict>() {
                        let mut label = String::new();
                        let mut enabled = true;
                        let mut visible = true;
                        let mut is_separator = false;

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
                                    _ => {}
                                }
                            }
                        }

                        items.push(MenuItem {
                            id,
                            label,
                            enabled,
                            visible,
                            is_separator,
                        });
                    }
                }
            }
        }

        Ok(items)
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



