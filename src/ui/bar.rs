use crate::config::bar_config::{load_bar_config, WidgetType, WidgetZone};
use crate::domain::workspace_service::WorkspaceService;
use crate::domain::system_tray_service::SystemTrayService;
use crate::domain::datetime_service::DateTimeService;
use crate::domain::battery_service::BatteryService;
use crate::domain::volume_service::VolumeService;
use crate::domain::notification_service::NotificationService;
use crate::domain::keyboard_layout_service::KeyboardLayoutService;
use crate::domain::system_resources_service::SystemResourcesService;
use crate::domain::models::{TrayItem, DateTimeConfig, Notification};
use crate::infrastructure::event_listener;
use crate::ui::{
    active_window::ActiveWindowWidget, datetime::DateTimeWidget, menu::Menu,
    system_tray::SystemTrayWidget, workspaces::WorkspacesWidget, battery::BatteryWidget,
    volume::VolumeWidget, notifications::NotificationWidget, notification_popup::NotificationPopup,
    keyboard_layout::KeyboardLayoutWidget, system_resources::SystemResourcesWidget,
};
use gtk4::prelude::*;
use gtk4::{gdk, glib};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};

pub struct Bar {
    window: gtk4::ApplicationWindow,
    workspaces_widget: Option<Arc<Mutex<WorkspacesWidget>>>,
    active_window_widget: Option<Arc<Mutex<ActiveWindowWidget>>>,
    datetime_widget: Option<Arc<Mutex<DateTimeWidget>>>,
    #[allow(dead_code)]
    system_tray_widget: Option<Arc<Mutex<SystemTrayWidget>>>,
    battery_widget: Option<Arc<Mutex<BatteryWidget>>>,
    volume_widget: Option<Arc<Mutex<VolumeWidget>>>,
    notifications_widget: Option<Arc<Mutex<NotificationWidget>>>,
    keyboard_layout_widget: Option<Arc<Mutex<KeyboardLayoutWidget>>>,
    system_resources_widget: Option<Arc<Mutex<SystemResourcesWidget>>>,
    app: gtk4::Application,
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
    ) -> Self {
        let window = gtk4::ApplicationWindow::new(app);

        window.init_layer_shell();
        window.set_title(Some(&format!("Bar - {}", monitor_name)));
        window.set_layer(Layer::Top);

        // Привязка к монитору
        if let Some(display) = gdk::Display::default() {
            let monitors = display.monitors();
            for i in 0..monitors.n_items() {
                if let Some(monitor) = monitors.item(i).and_then(|m| m.downcast::<gdk::Monitor>().ok()) {
                    if let Some(connector) = monitor.connector() {
                        if connector.as_str() == monitor_name {
                            window.set_monitor(Some(&monitor));
                            break;
                        }
                    }
                }
            }
        }

        window.set_anchor(Edge::Top, true);
        window.set_anchor(Edge::Left, true);
        window.set_anchor(Edge::Right, true);
        window.auto_exclusive_zone_enable();
        window.add_css_class("window");

        // Загружаем конфигурацию
        let config = load_bar_config();

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

        // Создаём виджеты
        let mut workspaces_widget = None;
        let mut active_window_widget = None;
        let mut datetime_widget = None;
        let mut system_tray_widget = None;
        let mut battery_widget = None;
        let mut volume_widget = None;
        let mut notifications_widget = None;
        let mut keyboard_layout_widget = None;
        let mut system_resources_widget = None;

        // Создаём список виджетов с их конфигурацией
        let mut widgets_to_place: Vec<(WidgetType, WidgetZone, usize)> = config
            .widgets
            .iter()
            .map(|(wtype, wconfig)| (wtype.clone(), wconfig.zone.clone(), wconfig.order))
            .collect();

        // Сортируем по порядку
        widgets_to_place.sort_by_key(|(_, _, order)| *order);

        // Группируем по зонам
        let mut zones: HashMap<WidgetZone, Vec<(WidgetType, usize)>> = HashMap::new();
        for (wtype, zone, order) in widgets_to_place {
            zones.entry(zone).or_default().push((wtype, order));
        }

        // Размещаем виджеты по зонам
        for (zone, mut widgets) in zones {
            widgets.sort_by_key(|(_, order)| *order);

            let target_box = match zone {
                WidgetZone::Left => &left_box,
                WidgetZone::Center => &center_box,
                WidgetZone::Right => &right_box,
            };

            for (wtype, _) in widgets {
                match wtype {
                    WidgetType::Menu => {
                        let menu = Menu::new();
                        let button = menu.create_button();
                        target_box.append(&button);
                    }
                    WidgetType::Workspaces => {
                        let widget = Arc::new(Mutex::new(WorkspacesWidget::new(
                            monitor_name.to_string(),
                            workspace_keys.clone(),
                            service.clone(),
                        )));
                        target_box.append(widget.lock().unwrap().widget());
                        workspaces_widget = Some(widget);
                    }
                    WidgetType::ActiveWindow => {
                        let widget = Arc::new(Mutex::new(ActiveWindowWidget::new(service.clone())));
                        target_box.append(widget.lock().unwrap().widget());
                        active_window_widget = Some(widget);
                    }
                    WidgetType::DateTime => {
                        let widget = Arc::new(Mutex::new(DateTimeWidget::new(
                            datetime_service.clone(),
                            datetime_config.clone(),
                        )));
                        target_box.append(widget.lock().unwrap().widget());
                        datetime_widget = Some(widget);
                    }
                    WidgetType::SystemTray => {
                        let widget = Arc::new(Mutex::new(SystemTrayWidget::new(tray_service.clone())));
                        target_box.append(widget.lock().unwrap().widget());
                        system_tray_widget = Some(widget);
                    }
                    WidgetType::Battery => {
                        let widget = Arc::new(Mutex::new(BatteryWidget::new(battery_service.clone())));
                        target_box.append(widget.lock().unwrap().widget());
                        battery_widget = Some(widget);
                    }
                    WidgetType::Volume => {
                        let widget = Arc::new(Mutex::new(VolumeWidget::new(volume_service.clone())));
                        target_box.append(widget.lock().unwrap().widget());
                        volume_widget = Some(widget);
                    }
                    WidgetType::Notifications => {
                        let widget = Arc::new(Mutex::new(NotificationWidget::new(notification_service.clone())));
                        target_box.append(widget.lock().unwrap().widget());
                        notifications_widget = Some(widget);
                    }
                    WidgetType::KeyboardLayout => {
                        let widget = Arc::new(Mutex::new(KeyboardLayoutWidget::new(keyboard_layout_service.clone())));
                        target_box.append(widget.lock().unwrap().widget());
                        keyboard_layout_widget = Some(widget);
                    }
                    WidgetType::SystemResources => {
                        let widget = Arc::new(Mutex::new(SystemResourcesWidget::new(system_resources_service.clone())));
                        target_box.append(widget.lock().unwrap().widget());
                        system_resources_widget = Some(widget);
                    }
                }
            }
        }

        window.set_child(Some(&main_box));

        Self {
            window,
            workspaces_widget,
            active_window_widget,
            datetime_widget,
            system_tray_widget,
            battery_widget,
            volume_widget,
            notifications_widget,
            keyboard_layout_widget,
            system_resources_widget,
            app: app.clone(),
        }
    }

    pub fn setup_event_listener(
        &self,
        tray_rx: async_channel::Receiver<Vec<TrayItem>>,
        volume_rx: async_channel::Receiver<()>,
        notification_rx: async_channel::Receiver<Notification>,
        keyboard_layout_rx: async_channel::Receiver<()>,
        battery_rx: async_channel::Receiver<()>,
    ) {
        let (tx, rx) = mpsc::channel();

        event_listener::start_event_listener(move || {
            let _ = tx.send(());
        });

        let workspaces_widget = self.workspaces_widget.clone();
        let active_window_widget = self.active_window_widget.clone();
        let keyboard_layout_widget = self.keyboard_layout_widget.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
            let mut should_update = false;

            while rx.try_recv().is_ok() {
                should_update = true;
            }

            if should_update {
                if let Some(ref widget) = workspaces_widget {
                    widget.lock().unwrap().update();
                }
                if let Some(ref widget) = active_window_widget {
                    widget.lock().unwrap().update();
                }
                if let Some(ref widget) = keyboard_layout_widget {
                    widget.lock().unwrap().update();
                }
            }

            glib::ControlFlow::Continue
        });

        // Обновление времени каждую секунду
        let datetime_widget = self.datetime_widget.clone();
        glib::timeout_add_local(std::time::Duration::from_secs(1), move || {
            if let Some(ref widget) = datetime_widget {
                widget.lock().unwrap().update_time();
            }
            glib::ControlFlow::Continue
        });

        // Обновление системного трея
        let system_tray_widget = self.system_tray_widget.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok(items) = tray_rx.try_recv() {
                if let Some(ref widget) = system_tray_widget {
                    widget.lock().unwrap().update(&items);
                }
            }
            glib::ControlFlow::Continue
        });

        // Обновление батареи по событиям от UPower D-Bus
        let battery_widget = self.battery_widget.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok(_) = battery_rx.try_recv() {
                if let Some(ref widget) = battery_widget {
                    widget.lock().unwrap().update();
                }
            }
            glib::ControlFlow::Continue
        });

        // Обновление громкости по событиям от PipeWire
        let volume_widget = self.volume_widget.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok(_) = volume_rx.try_recv() {
                if let Some(ref widget) = volume_widget {
                    widget.lock().unwrap().update();
                }
            }
            glib::ControlFlow::Continue
        });

        // Обработка уведомлений
        let notifications_widget = self.notifications_widget.clone();
        let app = self.app.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            while let Ok(notification) = notification_rx.try_recv() {
                // Показываем popup на 5 секунд
                let popup = NotificationPopup::new(notification, &app);
                popup.show(5);

                // Обновляем виджет уведомлений (счётчик)
                if let Some(ref widget) = notifications_widget {
                    widget.lock().unwrap().update();
                }
            }
            glib::ControlFlow::Continue
        });

        // Обработка событий смены раскладки клавиатуры
        let keyboard_layout_widget = self.keyboard_layout_widget.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while let Ok(_) = keyboard_layout_rx.try_recv() {
                if let Some(ref widget) = keyboard_layout_widget {
                    widget.lock().unwrap().update();
                }
            }
            glib::ControlFlow::Continue
        });

        // Обновление системных ресурсов каждые 2 секунды
        let system_resources_widget = self.system_resources_widget.clone();
        glib::timeout_add_local(std::time::Duration::from_secs(2), move || {
            if let Some(ref widget) = system_resources_widget {
                widget.lock().unwrap().update();
            }
            glib::ControlFlow::Continue
        });

        // Первоначальное обновление
        if let Some(ref widget) = self.workspaces_widget {
            widget.lock().unwrap().update();
        }
        if let Some(ref widget) = self.active_window_widget {
            widget.lock().unwrap().update();
        }
        if let Some(ref widget) = self.datetime_widget {
            widget.lock().unwrap().update_time();
        }
        if let Some(ref widget) = self.battery_widget {
            widget.lock().unwrap().update();
        }
        if let Some(ref widget) = self.volume_widget {
            widget.lock().unwrap().update();
        }
        if let Some(ref widget) = self.notifications_widget {
            widget.lock().unwrap().update();
        }
        if let Some(ref widget) = self.keyboard_layout_widget {
            widget.lock().unwrap().update();
        }
        if let Some(ref widget) = self.system_resources_widget {
            widget.lock().unwrap().update();
        }
    }

    pub fn present(&self) {
        self.window.present();
    }
}

