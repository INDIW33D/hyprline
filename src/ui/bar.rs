use crate::config::{get_config, subscribe_config_changes, WidgetType, WidgetPosition};
use crate::domain::workspace_service::WorkspaceService;
use crate::domain::system_tray_service::SystemTrayService;
use crate::domain::datetime_service::DateTimeService;
use crate::domain::battery_service::BatteryService;
use crate::domain::volume_service::VolumeService;
use crate::domain::notification_service::NotificationService;
use crate::domain::keyboard_layout_service::KeyboardLayoutService;
use crate::domain::system_resources_service::SystemResourcesService;
use crate::domain::network_service::NetworkService;
use crate::domain::brightness_service::BrightnessService;
use crate::domain::models::DateTimeConfig;
use crate::infrastructure::event_listener;
use crate::shared_state::SharedState;
use crate::ui::{
    active_window::ActiveWindowWidget, datetime::DateTimeWidget, menu::Menu,
    system_tray::SystemTrayWidget, workspaces::WorkspacesWidget, battery::BatteryWidget,
    volume::VolumeWidget, notifications::NotificationWidget,
    keyboard_layout::KeyboardLayoutWidget, system_resources::SystemResourcesWidget,
    network::NetworkWidget, brightness::BrightnessWidget,
};
use gtk4::prelude::*;
use gtk4::{gdk, glib};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{mpsc, Arc, Mutex};

/// Контекст для создания виджетов (все сервисы)
#[derive(Clone)]
pub struct WidgetContext {
    pub app: gtk4::Application,
    pub monitor_name: String,
    pub workspace_keys: HashMap<i32, String>,
    pub workspace_service: Arc<dyn WorkspaceService + Send + Sync>,
    pub tray_service: Arc<dyn SystemTrayService + Send + Sync>,
    pub datetime_service: Arc<dyn DateTimeService + Send + Sync>,
    pub datetime_config: DateTimeConfig,
    pub battery_service: Arc<dyn BatteryService + Send + Sync>,
    pub volume_service: Arc<dyn VolumeService + Send + Sync>,
    pub notification_service: Arc<dyn NotificationService + Send + Sync>,
    pub keyboard_layout_service: Arc<dyn KeyboardLayoutService + Send + Sync>,
    pub system_resources_service: Arc<dyn SystemResourcesService + Send + Sync>,
    pub network_service: Arc<dyn NetworkService + Send + Sync>,
    pub brightness_service: Arc<dyn BrightnessService + Send + Sync>,
    pub shared_state: Arc<SharedState>,
}

/// Созданные виджеты
struct CreatedWidgets {
    workspaces: Option<Arc<Mutex<WorkspacesWidget>>>,
    active_window: Option<Arc<Mutex<ActiveWindowWidget>>>,
    datetime: Option<Arc<Mutex<DateTimeWidget>>>,
    system_tray: Option<Arc<Mutex<SystemTrayWidget>>>,
    battery: Option<Arc<Mutex<BatteryWidget>>>,
    volume: Option<Arc<Mutex<VolumeWidget>>>,
    notifications: Option<Arc<Mutex<NotificationWidget>>>,
    keyboard_layout: Option<Arc<Mutex<KeyboardLayoutWidget>>>,
    system_resources: Option<Arc<Mutex<SystemResourcesWidget>>>,
    network: Option<NetworkWidget>,
    brightness: Option<BrightnessWidget>,
}

impl CreatedWidgets {
    fn new() -> Self {
        Self {
            workspaces: None,
            active_window: None,
            datetime: None,
            system_tray: None,
            battery: None,
            volume: None,
            notifications: None,
            keyboard_layout: None,
            system_resources: None,
            network: None,
            brightness: None,
        }
    }
}

pub struct Bar {
    window: gtk4::ApplicationWindow,
    left_box: gtk4::Box,
    center_box: gtk4::Box,
    right_box: gtk4::Box,
    context: Rc<WidgetContext>,
    widgets: Rc<RefCell<CreatedWidgets>>,
    shared_state: Arc<SharedState>,
}

impl Bar {
    pub fn new(
        app: &gtk4::Application,
        monitor_name: &str,
        workspace_keys: HashMap<i32, String>,
        service: Arc<dyn WorkspaceService + Send + Sync>,
        tray_service: Arc<dyn SystemTrayService + Send + Sync>,
        datetime_service: Arc<dyn DateTimeService + Send + Sync>,
        datetime_config: DateTimeConfig,
        battery_service: Arc<dyn BatteryService + Send + Sync>,
        volume_service: Arc<dyn VolumeService + Send + Sync>,
        notification_service: Arc<dyn NotificationService + Send + Sync>,
        keyboard_layout_service: Arc<dyn KeyboardLayoutService + Send + Sync>,
        system_resources_service: Arc<dyn SystemResourcesService + Send + Sync>,
        network_service: Arc<dyn NetworkService + Send + Sync>,
        brightness_service: Arc<dyn BrightnessService + Send + Sync>,
        shared_state: Arc<SharedState>,
    ) -> Self {
        let window = gtk4::ApplicationWindow::new(app);

        window.init_layer_shell();
        window.set_title(Some(&format!("Bar - {}", monitor_name)));
        window.set_layer(Layer::Top);

        // Привязка к монитору
        let monitor_found = Self::try_bind_to_monitor(&window, monitor_name);

        // Если монитор не найден сразу - пробуем через задержку
        // (монитор может ещё не быть зарегистрирован в GDK)
        if !monitor_found && monitor_name != "default" {
            let window_weak = window.downgrade();
            let monitor_name_owned = monitor_name.to_string();
            glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
                if let Some(win) = window_weak.upgrade() {
                    if Self::try_bind_to_monitor(&win, &monitor_name_owned) {
                        eprintln!("[Bar] ✓ Delayed binding to monitor: {}", monitor_name_owned);
                    } else {
                        eprintln!("[Bar] ✗ Monitor not found after delay: {}", monitor_name_owned);
                    }
                }
            });
        }

        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.auto_exclusive_zone_enable();
        window.add_css_class("window");

        // Создаём три зоны
        let left_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        left_box.add_css_class("zone-left");
        left_box.set_halign(gtk4::Align::Start);

        let center_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        center_box.add_css_class("zone-center");
        center_box.set_halign(gtk4::Align::Center);
        center_box.set_hexpand(true);

        let right_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        right_box.add_css_class("zone-right");
        right_box.set_halign(gtk4::Align::End);

        // Контейнер для всех зон
        let main_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
        main_box.add_css_class("line");
        main_box.append(&left_box);
        main_box.append(&center_box);
        main_box.append(&right_box);

        window.set_child(Some(&main_box));

        // Создаём контекст для виджетов
        let context = Rc::new(WidgetContext {
            app: app.clone(),
            monitor_name: monitor_name.to_string(),
            workspace_keys,
            workspace_service: service,
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
            shared_state: shared_state.clone(),
        });

        let widgets = Rc::new(RefCell::new(CreatedWidgets::new()));

        let mut bar = Self {
            window,
            left_box,
            center_box,
            right_box,
            context,
            widgets,
            shared_state,
        };

        // Создаём виджеты из конфигурации
        bar.rebuild_widgets();

        bar
    }

    /// Очищает все зоны
    fn clear_zones(&self) {
        while let Some(child) = self.left_box.first_child() {
            self.left_box.remove(&child);
        }
        while let Some(child) = self.center_box.first_child() {
            self.center_box.remove(&child);
        }
        while let Some(child) = self.right_box.first_child() {
            self.right_box.remove(&child);
        }
    }

    /// Перестраивает виджеты на основе конфигурации
    pub fn rebuild_widgets(&mut self) {
        // Очищаем зоны
        self.clear_zones();

        // Сбрасываем ссылки на виджеты
        *self.widgets.borrow_mut() = CreatedWidgets::new();

        // Загружаем конфигурацию и копируем нужные данные
        let (left_widgets, center_widgets, right_widgets) = {
            let config = get_config().read().unwrap();

            let mut left: Vec<_> = config.widgets.iter()
                .filter(|w| w.enabled && w.position == WidgetPosition::Left)
                .map(|w| (w.widget_type, w.order))
                .collect();
            let mut center: Vec<_> = config.widgets.iter()
                .filter(|w| w.enabled && w.position == WidgetPosition::Center)
                .map(|w| (w.widget_type, w.order))
                .collect();
            let mut right: Vec<_> = config.widgets.iter()
                .filter(|w| w.enabled && w.position == WidgetPosition::Right)
                .map(|w| (w.widget_type, w.order))
                .collect();

            left.sort_by_key(|(_, order)| *order);
            center.sort_by_key(|(_, order)| *order);
            right.sort_by_key(|(_, order)| *order);

            (left, center, right)
        };

        // Создаём виджеты для каждой зоны
        for (widget_type, _) in left_widgets {
            self.create_widget(widget_type, &self.left_box.clone());
        }
        for (widget_type, _) in center_widgets {
            self.create_widget(widget_type, &self.center_box.clone());
        }
        for (widget_type, _) in right_widgets {
            self.create_widget(widget_type, &self.right_box.clone());
        }

        // Обновляем все виджеты с текущими данными
        self.initial_update();

        // Обновляем трей с текущими данными
        let items = self.shared_state.get_tray();
        let widgets = self.widgets.borrow();
        if let Some(ref widget) = widgets.system_tray {
            widget.lock().unwrap().update(&items);
        }

        eprintln!("[Bar] ✓ Widgets rebuilt");
    }

    /// Создаёт виджет и добавляет его в контейнер
    fn create_widget(&self, widget_type: WidgetType, container: &gtk4::Box) {
        let ctx = &self.context;
        let mut widgets = self.widgets.borrow_mut();

        match widget_type {
            WidgetType::Menu => {
                let menu = Menu::new();
                let button = menu.create_button(&ctx.app);
                container.append(&button);
            }
            WidgetType::Workspaces => {
                let widget = Arc::new(Mutex::new(WorkspacesWidget::new(
                    ctx.monitor_name.clone(),
                    ctx.workspace_keys.clone(),
                    ctx.workspace_service.clone(),
                )));
                container.append(widget.lock().unwrap().widget());
                widgets.workspaces = Some(widget);
            }
            WidgetType::ActiveWindow => {
                let widget = Arc::new(Mutex::new(ActiveWindowWidget::new(ctx.workspace_service.clone())));
                container.append(widget.lock().unwrap().widget());
                widgets.active_window = Some(widget);
            }
            WidgetType::DateTime => {
                let widget = Arc::new(Mutex::new(DateTimeWidget::new(
                    ctx.datetime_service.clone(),
                    ctx.datetime_config.clone(),
                )));
                container.append(widget.lock().unwrap().widget());
                widgets.datetime = Some(widget);
            }
            WidgetType::SystemTray => {
                let widget = Arc::new(Mutex::new(SystemTrayWidget::new(ctx.tray_service.clone())));
                container.append(widget.lock().unwrap().widget());
                widgets.system_tray = Some(widget);
            }
            WidgetType::Battery => {
                let widget = Arc::new(Mutex::new(BatteryWidget::new(ctx.battery_service.clone())));
                container.append(widget.lock().unwrap().widget());
                widgets.battery = Some(widget);
            }
            WidgetType::Volume => {
                let widget = Arc::new(Mutex::new(VolumeWidget::new(ctx.volume_service.clone())));
                container.append(widget.lock().unwrap().widget());
                widgets.volume = Some(widget);
            }
            WidgetType::Notifications => {
                let widget = Arc::new(Mutex::new(NotificationWidget::new(ctx.notification_service.clone())));
                container.append(widget.lock().unwrap().widget());
                widgets.notifications = Some(widget);
            }
            WidgetType::KeyboardLayout => {
                let widget = Arc::new(Mutex::new(KeyboardLayoutWidget::new(ctx.keyboard_layout_service.clone())));
                container.append(widget.lock().unwrap().widget());
                widgets.keyboard_layout = Some(widget);
            }
            WidgetType::SystemResources => {
                let widget = Arc::new(Mutex::new(SystemResourcesWidget::new(ctx.system_resources_service.clone())));
                container.append(widget.lock().unwrap().widget());
                widgets.system_resources = Some(widget);
            }
            WidgetType::Network => {
                let widget = NetworkWidget::new(ctx.network_service.clone());
                container.append(&widget.container);
                widgets.network = Some(widget);
            }
            WidgetType::Brightness => {
                let widget = BrightnessWidget::new(ctx.brightness_service.clone());
                container.append(&widget.container);
                widgets.brightness = Some(widget);
            }
        }
    }

    pub fn setup_event_listener(&self) {
        let (tx, rx) = mpsc::channel();

        event_listener::start_event_listener(move || {
            let _ = tx.send(());
        });

        // Обработка событий Hyprland (workspaces, active window)
        let widgets = self.widgets.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            let mut should_update = false;

            while rx.try_recv().is_ok() {
                should_update = true;
            }

            if should_update {
                let widgets = widgets.borrow();
                if let Some(ref widget) = widgets.workspaces {
                    widget.lock().unwrap().update();
                }
                if let Some(ref widget) = widgets.active_window {
                    widget.lock().unwrap().update();
                }
            }

            glib::ControlFlow::Continue
        });

        // Обновление времени каждую секунду
        let widgets = self.widgets.clone();
        glib::timeout_add_local(std::time::Duration::from_secs(1), move || {
            let widgets = widgets.borrow();
            if let Some(ref widget) = widgets.datetime {
                widget.lock().unwrap().update_time();
            }
            glib::ControlFlow::Continue
        });

        // === ПОДПИСКИ НА SHARED STATE ===
        self.setup_shared_state_subscriptions();

        // Первоначальное обновление
        self.initial_update();
    }

    fn setup_shared_state_subscriptions(&self) {
        // Подписка на обновления батареи
        {
            let widgets = self.widgets.clone();
            let (sender, receiver) = async_channel::unbounded::<()>();

            self.shared_state.subscribe_battery(move || {
                let _ = sender.send_blocking(());
            });

            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                while receiver.try_recv().is_ok() {
                    let widgets = widgets.borrow();
                    if let Some(ref widget) = widgets.battery {
                        widget.lock().unwrap().update();
                    }
                }
                glib::ControlFlow::Continue
            });
        }

        // Подписка на обновления громкости
        {
            let widgets = self.widgets.clone();
            let (sender, receiver) = async_channel::unbounded::<()>();

            self.shared_state.subscribe_volume(move || {
                let _ = sender.send_blocking(());
            });

            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                while receiver.try_recv().is_ok() {
                    let widgets = widgets.borrow();
                    if let Some(ref widget) = widgets.volume {
                        widget.lock().unwrap().update();
                    }
                }
                glib::ControlFlow::Continue
            });
        }

        // Подписка на обновления трея
        {
            let widgets = self.widgets.clone();
            let shared_state = self.shared_state.clone();
            let (sender, receiver) = async_channel::unbounded::<()>();

            self.shared_state.subscribe_tray(move || {
                let _ = sender.send_blocking(());
            });

            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                while receiver.try_recv().is_ok() {
                    let items = shared_state.get_tray();
                    let widgets = widgets.borrow();
                    if let Some(ref widget) = widgets.system_tray {
                        widget.lock().unwrap().update(&items);
                    }
                }
                glib::ControlFlow::Continue
            });
        }

        // Подписка на обновления раскладки клавиатуры
        {
            let widgets = self.widgets.clone();
            let (sender, receiver) = async_channel::unbounded::<()>();

            self.shared_state.subscribe_keyboard_layout(move || {
                let _ = sender.send_blocking(());
            });

            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                while receiver.try_recv().is_ok() {
                    let widgets = widgets.borrow();
                    if let Some(ref widget) = widgets.keyboard_layout {
                        widget.lock().unwrap().update();
                    }
                }
                glib::ControlFlow::Continue
            });
        }

        // Подписка на обновления уведомлений
        {
            let widgets = self.widgets.clone();
            let (sender, receiver) = async_channel::unbounded::<()>();

            self.shared_state.subscribe_notifications(move || {
                let _ = sender.send_blocking(());
            });

            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                while receiver.try_recv().is_ok() {
                    let widgets = widgets.borrow();
                    if let Some(ref widget) = widgets.notifications {
                        widget.lock().unwrap().update();
                    }
                }
                glib::ControlFlow::Continue
            });
        }

        // Подписка на обновления системных ресурсов
        {
            let widgets = self.widgets.clone();
            let (sender, receiver) = async_channel::unbounded::<()>();

            self.shared_state.subscribe_system_resources(move || {
                let _ = sender.send_blocking(());
            });

            glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
                while receiver.try_recv().is_ok() {
                    let widgets = widgets.borrow();
                    if let Some(ref widget) = widgets.system_resources {
                        widget.lock().unwrap().update();
                    }
                }
                glib::ControlFlow::Continue
            });
        }
    }

    fn initial_update(&self) {
        let widgets = self.widgets.borrow();

        if let Some(ref widget) = widgets.workspaces {
            widget.lock().unwrap().update();
        }
        if let Some(ref widget) = widgets.active_window {
            widget.lock().unwrap().update();
        }
        if let Some(ref widget) = widgets.datetime {
            widget.lock().unwrap().update_time();
        }
        if let Some(ref widget) = widgets.battery {
            widget.lock().unwrap().update();
        }
        if let Some(ref widget) = widgets.volume {
            widget.lock().unwrap().update();
        }
        if let Some(ref widget) = widgets.notifications {
            widget.lock().unwrap().update();
        }
        if let Some(ref widget) = widgets.keyboard_layout {
            widget.lock().unwrap().update();
        }
        if let Some(ref widget) = widgets.system_resources {
            widget.lock().unwrap().update();
        }
    }

    pub fn present(&self) {
        self.window.present();
    }

    /// Получить имя монитора, к которому привязан бар
    pub fn monitor_name(&self) -> &str {
        &self.context.monitor_name
    }

    /// Закрыть окно бара
    pub fn close(&self) {
        self.window.close();
    }

    /// Получить ссылку на контейнеры для hot reload
    pub fn get_zone_boxes(&self) -> (gtk4::Box, gtk4::Box, gtk4::Box) {
        (self.left_box.clone(), self.center_box.clone(), self.right_box.clone())
    }

    /// Пытается привязать окно к монитору по имени
    /// Возвращает true если монитор найден и привязан
    fn try_bind_to_monitor(window: &gtk4::ApplicationWindow, monitor_name: &str) -> bool {
        if let Some(display) = gdk::Display::default() {
            let monitors = display.monitors();
            for i in 0..monitors.n_items() {
                if let Some(monitor) = monitors.item(i).and_then(|m| m.downcast::<gdk::Monitor>().ok()) {
                    if let Some(connector) = monitor.connector() {
                        if connector.as_str() == monitor_name {
                            window.set_monitor(Some(&monitor));
                            eprintln!("[Bar] Bound to monitor: {}", monitor_name);
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

