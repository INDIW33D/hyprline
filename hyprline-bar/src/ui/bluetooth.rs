use crate::domain::bluetooth_service::BluetoothService;
use crate::domain::models::{BluetoothDevice, BluetoothDeviceType, BluetoothInfo};
use crate::shared_state::get_shared_state;
use gtk4::{glib, prelude::*, Box as GtkBox, Button, Label, Orientation, Popover, ScrolledWindow};
use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

pub struct BluetoothWidget {
    pub container: GtkBox,
}

impl BluetoothWidget {
    pub fn new(bluetooth_service: Arc<dyn BluetoothService + Send + Sync>) -> Self {
        let container = GtkBox::new(Orientation::Horizontal, 4);
        container.set_css_classes(&["bluetooth-widget"]);

        // Иконка
        let icon_label = Label::new(Some("󰂯"));
        icon_label.set_css_classes(&["bluetooth-icon"]);
        container.append(&icon_label);

        // Обновляем начальное состояние из SharedState
        let shared_state = get_shared_state();
        Self::update_display(&icon_label, shared_state.get_bluetooth());

        // Создаем popover для управления Bluetooth
        let popover = Self::create_bluetooth_popover(bluetooth_service.clone());
        popover.set_parent(&container);

        // Обработчик клика
        let gesture = gtk4::GestureClick::new();
        {
            let popover = popover.clone();
            gesture.connect_released(move |_, _, _, _| {
                popover.popup();
            });
        }
        container.add_controller(gesture);

        // Подписка на обновления через SharedState
        let (tx, rx) = async_channel::unbounded::<()>();
        let icon_label_clone = icon_label.clone();

        shared_state.subscribe_bluetooth(move || {
            let _ = tx.send_blocking(());
        });

        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while rx.try_recv().is_ok() {
                let info = get_shared_state().get_bluetooth();
                Self::update_display(&icon_label_clone, info);
            }
            glib::ControlFlow::Continue
        });

        Self { container }
    }

    fn update_display(icon_label: &Label, info: Option<BluetoothInfo>) {
        match info {
            Some(bt) if bt.powered => {
                if !bt.connected_devices.is_empty() {
                    // Подключено устройство — показываем иконку подключения
                    icon_label.set_text("󰂱"); // nf-md-bluetooth_connect
                    icon_label.set_css_classes(&["bluetooth-icon", "bluetooth-connected"]);
                } else {
                    // Bluetooth включён, но ничего не подключено
                    icon_label.set_text("󰂯"); // nf-md-bluetooth
                    icon_label.set_css_classes(&["bluetooth-icon"]);
                }
            }
            _ => {
                // Bluetooth выключен или недоступен
                icon_label.set_text("󰂲"); // nf-md-bluetooth_off
                icon_label.set_css_classes(&["bluetooth-icon", "bluetooth-off"]);
            }
        }
    }

    fn get_device_icon(device: &BluetoothDevice) -> &'static str {
        match device.device_type {
            BluetoothDeviceType::Audio => "󰋋",    // nf-md-headphones
            BluetoothDeviceType::Phone => "󰏲",    // nf-md-cellphone
            BluetoothDeviceType::Computer => "󰍹", // nf-md-monitor
            BluetoothDeviceType::Keyboard => "󰌌", // nf-md-keyboard
            BluetoothDeviceType::Mouse => "󰍽",    // nf-md-mouse
            BluetoothDeviceType::Gamepad => "󰊗",  // nf-md-gamepad_variant
            BluetoothDeviceType::Other => "󰂯",    // nf-md-bluetooth
        }
    }

    fn create_bluetooth_popover(
        bluetooth_service: Arc<dyn BluetoothService + Send + Sync>,
    ) -> Popover {
        let popover = Popover::new();
        popover.set_css_classes(&["bluetooth-popover"]);

        let main_box = GtkBox::new(Orientation::Vertical, 8);
        main_box.set_margin_start(12);
        main_box.set_margin_end(12);
        main_box.set_margin_top(12);
        main_box.set_margin_bottom(12);

        // Заголовок с переключателем Bluetooth
        let header_box = GtkBox::new(Orientation::Horizontal, 8);

        let title = Label::new(Some("Bluetooth"));
        title.set_css_classes(&["bluetooth-title"]);
        title.set_hexpand(true);
        title.set_halign(gtk4::Align::Start);

        let bt_switch = gtk4::Switch::new();
        bt_switch.set_active(bluetooth_service.is_powered());

        {
            let bluetooth_service = bluetooth_service.clone();
            bt_switch.connect_active_notify(move |switch| {
                let _ = bluetooth_service.set_powered(switch.is_active());
            });
        }

        header_box.append(&title);
        header_box.append(&bt_switch);
        main_box.append(&header_box);

        // Разделитель
        let separator = gtk4::Separator::new(Orientation::Horizontal);
        main_box.append(&separator);

        // Подключённые устройства (секция)
        let connected_label = Label::new(Some("Connected"));
        connected_label.set_css_classes(&["section-label"]);
        connected_label.set_halign(gtk4::Align::Start);
        main_box.append(&connected_label);

        let connected_box = GtkBox::new(Orientation::Vertical, 4);
        connected_box.set_css_classes(&["bluetooth-connected-list"]);
        main_box.append(&connected_box);

        let separator2 = gtk4::Separator::new(Orientation::Horizontal);
        main_box.append(&separator2);

        // Список устройств (paired + discovered)
        let devices_label = Label::new(Some("Devices"));
        devices_label.set_css_classes(&["section-label"]);
        devices_label.set_halign(gtk4::Align::Start);
        main_box.append(&devices_label);

        let scroll = ScrolledWindow::new();
        scroll.set_min_content_height(200);
        scroll.set_min_content_width(300);
        scroll.set_max_content_height(400);
        scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);

        let devices_box = GtkBox::new(Orientation::Vertical, 4);
        scroll.set_child(Some(&devices_box));
        main_box.append(&scroll);

        // Флаг для управления авто-обновлением во время сканирования
        // true = таймер должен продолжать обновлять, false = остановить
        let scanning_active = Rc::new(Cell::new(false));

        // Кнопки управления
        let button_box = GtkBox::new(Orientation::Horizontal, 8);
        button_box.set_halign(gtk4::Align::Fill);
        button_box.set_homogeneous(true);

        let scan_button = Button::with_label("󰑐 Scan");
        scan_button.set_css_classes(&["bluetooth-scan-button"]);

        let refresh_button = Button::with_label("󰑐 Refresh");
        refresh_button.set_css_classes(&["refresh-button"]);

        // Обработчик кнопки Scan
        {
            let bluetooth_service = bluetooth_service.clone();
            let scan_button_ref = scan_button.clone();
            let scanning_active = scanning_active.clone();
            let connected_box = connected_box.clone();
            let devices_box = devices_box.clone();
            let popover = popover.clone();

            scan_button.connect_clicked(move |_| {
                let is_discovering = bluetooth_service
                    .get_bluetooth_info()
                    .map(|i| i.discovering)
                    .unwrap_or(false);

                if is_discovering {
                    // Останавливаем сканирование
                    let _ = bluetooth_service.stop_discovery();
                    scan_button_ref.set_label("󰑐 Scan");
                    scan_button_ref.remove_css_class("bluetooth-scanning");
                    scanning_active.set(false);

                    // Финальное обновление списка
                    Self::load_devices(
                        &connected_box,
                        &devices_box,
                        bluetooth_service.clone(),
                        popover.clone(),
                    );
                } else {
                    // Запускаем сканирование
                    let _ = bluetooth_service.start_discovery();
                    scan_button_ref.set_label("󰓛 Stop");
                    scan_button_ref.add_css_class("bluetooth-scanning");
                    scanning_active.set(true);

                    // Сразу загружаем текущий список
                    Self::load_devices(
                        &connected_box,
                        &devices_box,
                        bluetooth_service.clone(),
                        popover.clone(),
                    );

                    // Запускаем периодическое обновление каждые 3 секунды
                    {
                        let scanning_active = scanning_active.clone();
                        let connected_box = connected_box.clone();
                        let devices_box = devices_box.clone();
                        let bluetooth_service = bluetooth_service.clone();
                        let popover = popover.clone();

                        glib::timeout_add_local(std::time::Duration::from_secs(3), move || {
                            if !scanning_active.get() {
                                return glib::ControlFlow::Break;
                            }

                            Self::load_devices_silent(
                                &connected_box,
                                &devices_box,
                                bluetooth_service.clone(),
                                popover.clone(),
                            );

                            glib::ControlFlow::Continue
                        });
                    }

                    // Автоматически останавливаем через 30 секунд
                    {
                        let bt_service = bluetooth_service.clone();
                        let btn = scan_button_ref.clone();
                        let scanning_active = scanning_active.clone();
                        let connected_box = connected_box.clone();
                        let devices_box = devices_box.clone();
                        let popover = popover.clone();

                        glib::timeout_add_local_once(
                            std::time::Duration::from_secs(30),
                            move || {
                                if scanning_active.get() {
                                    let _ = bt_service.stop_discovery();
                                    btn.set_label("󰑐 Scan");
                                    btn.remove_css_class("bluetooth-scanning");
                                    scanning_active.set(false);

                                    // Финальное обновление
                                    Self::load_devices(
                                        &connected_box,
                                        &devices_box,
                                        bt_service.clone(),
                                        popover.clone(),
                                    );
                                }
                            },
                        );
                    }
                }
            });
        }

        // Обработчик кнопки Refresh
        {
            let bluetooth_service = bluetooth_service.clone();
            let devices_box = devices_box.clone();
            let connected_box = connected_box.clone();
            let popover = popover.clone();

            refresh_button.connect_clicked(move |_| {
                Self::load_devices(
                    &connected_box,
                    &devices_box,
                    bluetooth_service.clone(),
                    popover.clone(),
                );
            });
        }

        button_box.append(&scan_button);
        button_box.append(&refresh_button);
        main_box.append(&button_box);

        popover.set_child(Some(&main_box));

        // Загружаем устройства при открытии popover
        {
            let devices_box = devices_box.clone();
            let connected_box = connected_box.clone();
            let bluetooth_service = bluetooth_service.clone();
            let popover_clone = popover.clone();
            let bt_switch_clone = bt_switch.clone();
            let scan_button_clone = scan_button.clone();
            let scanning_active_show = scanning_active.clone();

            popover.connect_show(move |_| {
                // Обновляем состояние переключателя
                let is_powered = bluetooth_service.is_powered();
                bt_switch_clone.set_active(is_powered);

                // Автоматически запускаем сканирование при открытии, если Bluetooth включён
                if is_powered {
                    let is_discovering = bluetooth_service
                        .get_bluetooth_info()
                        .map(|i| i.discovering)
                        .unwrap_or(false);

                    // Если discovery ещё не идёт — запускаем
                    if !is_discovering {
                        let _ = bluetooth_service.start_discovery();
                    }

                    // Обновляем UI кнопки на "Stop"
                    scan_button_clone.set_label("󰓛 Stop");
                    scan_button_clone.add_css_class("bluetooth-scanning");

                    // Запускаем авто-обновление, если ещё не запущено
                    if !scanning_active_show.get() {
                        scanning_active_show.set(true);

                        let scanning_active = scanning_active_show.clone();
                        let connected_box = connected_box.clone();
                        let devices_box = devices_box.clone();
                        let bluetooth_service = bluetooth_service.clone();
                        let popover = popover_clone.clone();

                        glib::timeout_add_local(std::time::Duration::from_secs(3), move || {
                            if !scanning_active.get() {
                                return glib::ControlFlow::Break;
                            }

                            Self::load_devices_silent(
                                &connected_box,
                                &devices_box,
                                bluetooth_service.clone(),
                                popover.clone(),
                            );

                            glib::ControlFlow::Continue
                        });
                    }
                } else {
                    // Bluetooth выключен — сбрасываем состояние
                    scan_button_clone.set_label("󰑐 Scan");
                    scan_button_clone.remove_css_class("bluetooth-scanning");
                    scanning_active_show.set(false);
                }

                Self::load_devices(
                    &connected_box,
                    &devices_box,
                    bluetooth_service.clone(),
                    popover_clone.clone(),
                );
            });
        }

        // При закрытии popover останавливаем авто-обновление
        {
            let scanning_active = scanning_active.clone();
            let bluetooth_service = bluetooth_service.clone();
            popover.connect_closed(move |_| {
                if scanning_active.get() {
                    // Останавливаем discovery при закрытии
                    let _ = bluetooth_service.stop_discovery();
                    scanning_active.set(false);
                }
            });
        }

        popover
    }

    /// Загружает устройства с индикатором загрузки (для первоначальной загрузки / ручного refresh)
    fn load_devices(
        connected_box: &GtkBox,
        devices_box: &GtkBox,
        bluetooth_service: Arc<dyn BluetoothService + Send + Sync>,
        popover: Popover,
    ) {
        // Очищаем списки
        while let Some(child) = connected_box.first_child() {
            connected_box.remove(&child);
        }
        while let Some(child) = devices_box.first_child() {
            devices_box.remove(&child);
        }

        // Показываем индикатор загрузки
        let loading = Label::new(Some("Loading..."));
        loading.set_css_classes(&["loading-label"]);
        devices_box.append(&loading);

        glib::spawn_future_local({
            let connected_box = connected_box.clone();
            let devices_box = devices_box.clone();
            let bluetooth_service = bluetooth_service.clone();

            async move {
                // Даем время на отрисовку индикатора
                glib::timeout_future(std::time::Duration::from_millis(100)).await;

                let devices = bluetooth_service.get_devices();
                Self::populate_device_lists(
                    &connected_box,
                    &devices_box,
                    &devices,
                    bluetooth_service,
                    popover,
                );
            }
        });
    }

    /// Загружает устройства без индикатора загрузки (для автообновления во время сканирования)
    fn load_devices_silent(
        connected_box: &GtkBox,
        devices_box: &GtkBox,
        bluetooth_service: Arc<dyn BluetoothService + Send + Sync>,
        popover: Popover,
    ) {
        let devices = bluetooth_service.get_devices();
        Self::populate_device_lists(
            connected_box,
            devices_box,
            &devices,
            bluetooth_service,
            popover,
        );
    }

    /// Заполняет списки устройствами
    fn populate_device_lists(
        connected_box: &GtkBox,
        devices_box: &GtkBox,
        devices: &[BluetoothDevice],
        bluetooth_service: Arc<dyn BluetoothService + Send + Sync>,
        popover: Popover,
    ) {
        // Очищаем оба списка
        while let Some(child) = connected_box.first_child() {
            connected_box.remove(&child);
        }
        while let Some(child) = devices_box.first_child() {
            devices_box.remove(&child);
        }

        let mut has_connected = false;
        let mut has_other = false;

        for device in devices {
            if device.connected {
                has_connected = true;
                let item = Self::create_connected_device_item(
                    device,
                    bluetooth_service.clone(),
                    popover.clone(),
                );
                connected_box.append(&item);
            } else {
                has_other = true;
                let item =
                    Self::create_device_item(device, bluetooth_service.clone(), popover.clone());
                devices_box.append(&item);
            }
        }

        if !has_connected {
            let no_conn = Label::new(Some("No connected devices"));
            no_conn.set_css_classes(&["bluetooth-no-devices"]);
            connected_box.append(&no_conn);
        }

        if !has_other {
            let no_devices = Label::new(Some("No other devices found"));
            no_devices.set_css_classes(&["bluetooth-no-devices"]);
            devices_box.append(&no_devices);
        }
    }

    fn create_connected_device_item(
        device: &BluetoothDevice,
        bluetooth_service: Arc<dyn BluetoothService + Send + Sync>,
        _popover: Popover,
    ) -> GtkBox {
        let item_box = GtkBox::new(Orientation::Horizontal, 8);
        item_box.set_css_classes(&["bluetooth-device-item", "bluetooth-device-connected"]);
        item_box.set_margin_start(4);
        item_box.set_margin_end(4);
        item_box.set_margin_top(2);
        item_box.set_margin_bottom(2);

        // Иконка типа устройства
        let icon = Label::new(Some(Self::get_device_icon(device)));
        icon.set_css_classes(&["bluetooth-device-icon"]);

        // Информация об устройстве
        let info_box = GtkBox::new(Orientation::Vertical, 2);
        info_box.set_hexpand(true);

        let name_label = Label::new(Some(&device.alias));
        name_label.set_css_classes(&["bluetooth-device-name"]);
        name_label.set_halign(gtk4::Align::Start);
        name_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        name_label.set_max_width_chars(20);

        let mut status_parts = vec![device.device_type.to_string()];
        if let Some(battery) = device.battery_percentage {
            status_parts.push(format!("{}%", battery));
        }
        let status_text = status_parts.join(" · ");

        let status_label = Label::new(Some(&status_text));
        status_label.set_css_classes(&["bluetooth-device-status"]);
        status_label.set_halign(gtk4::Align::Start);

        info_box.append(&name_label);
        info_box.append(&status_label);

        // Кнопка отключения
        let disconnect_btn = Button::with_label("Disconnect");
        disconnect_btn.set_css_classes(&["bluetooth-disconnect-button"]);
        disconnect_btn.set_valign(gtk4::Align::Center);

        {
            let address = device.address.clone();
            disconnect_btn.connect_clicked(move |btn| {
                btn.set_sensitive(false);
                btn.set_label("...");
                let bt = bluetooth_service.clone();
                let addr = address.clone();
                let btn_clone = btn.clone();
                glib::spawn_future_local(async move {
                    glib::timeout_future(std::time::Duration::from_millis(50)).await;
                    match bt.disconnect_device(&addr) {
                        Ok(()) => {
                            btn_clone.set_label("Disconnected");
                        }
                        Err(e) => {
                            eprintln!("[Bluetooth UI] Failed to disconnect: {}", e);
                            btn_clone.set_label("Error");
                            btn_clone.set_sensitive(true);
                        }
                    }
                });
            });
        }

        item_box.append(&icon);
        item_box.append(&info_box);
        item_box.append(&disconnect_btn);

        item_box
    }

    fn create_device_item(
        device: &BluetoothDevice,
        bluetooth_service: Arc<dyn BluetoothService + Send + Sync>,
        _popover: Popover,
    ) -> GtkBox {
        let item_box = GtkBox::new(Orientation::Horizontal, 8);
        item_box.set_css_classes(&["bluetooth-device-item"]);
        item_box.set_margin_start(4);
        item_box.set_margin_end(4);
        item_box.set_margin_top(2);
        item_box.set_margin_bottom(2);

        // Иконка типа устройства
        let icon = Label::new(Some(Self::get_device_icon(device)));
        icon.set_css_classes(&["bluetooth-device-icon"]);

        // Информация об устройстве
        let info_box = GtkBox::new(Orientation::Vertical, 2);
        info_box.set_hexpand(true);

        let name_label = Label::new(Some(&device.alias));
        name_label.set_css_classes(&["bluetooth-device-name"]);
        name_label.set_halign(gtk4::Align::Start);
        name_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
        name_label.set_max_width_chars(20);

        let mut status_parts = Vec::new();
        if device.paired {
            status_parts.push("Paired".to_string());
        }
        if let Some(rssi) = device.rssi {
            let strength = Self::rssi_to_strength(rssi);
            status_parts.push(format!("{}%", strength));
        }

        let status_text = if status_parts.is_empty() {
            device.device_type.to_string()
        } else {
            format!("{} · {}", device.device_type, status_parts.join(" · "))
        };

        let status_label = Label::new(Some(&status_text));
        status_label.set_css_classes(&["bluetooth-device-status"]);
        status_label.set_halign(gtk4::Align::Start);

        info_box.append(&name_label);
        info_box.append(&status_label);

        // Кнопка подключения
        let action_btn = if device.paired {
            let btn = Button::with_label("Connect");
            btn.set_css_classes(&["bluetooth-connect-button"]);

            let address = device.address.clone();
            let bt = bluetooth_service.clone();
            btn.connect_clicked(move |btn| {
                btn.set_sensitive(false);
                btn.set_label("...");
                let bt = bt.clone();
                let addr = address.clone();
                let btn_clone = btn.clone();
                glib::spawn_future_local(async move {
                    glib::timeout_future(std::time::Duration::from_millis(50)).await;
                    match bt.connect_device(&addr) {
                        Ok(()) => {
                            btn_clone.set_label("Connected");
                        }
                        Err(e) => {
                            eprintln!("[Bluetooth UI] Failed to connect: {}", e);
                            btn_clone.set_label("Error");
                            btn_clone.set_sensitive(true);
                        }
                    }
                });
            });
            btn
        } else {
            let btn = Button::with_label("Pair");
            btn.set_css_classes(&["bluetooth-pair-button"]);

            let address = device.address.clone();
            let bt = bluetooth_service.clone();
            btn.connect_clicked(move |btn| {
                btn.set_sensitive(false);
                btn.set_label("...");
                let bt = bt.clone();
                let addr = address.clone();
                let btn_clone = btn.clone();
                glib::spawn_future_local(async move {
                    glib::timeout_future(std::time::Duration::from_millis(50)).await;
                    // Сначала trust, затем pair
                    let _ = bt.trust_device(&addr);
                    match bt.pair_device(&addr) {
                        Ok(()) => {
                            btn_clone.set_label("Paired");
                        }
                        Err(e) => {
                            eprintln!("[Bluetooth UI] Failed to pair: {}", e);
                            btn_clone.set_label("Error");
                            btn_clone.set_sensitive(true);
                        }
                    }
                });
            });
            btn
        };
        action_btn.set_valign(gtk4::Align::Center);

        item_box.append(&icon);
        item_box.append(&info_box);
        item_box.append(&action_btn);

        item_box
    }

    /// Конвертирует RSSI (dBm) в процент силы сигнала
    fn rssi_to_strength(rssi: i16) -> u8 {
        // Типичный диапазон RSSI для Bluetooth: -100 dBm (слабый) до -40 dBm (сильный)
        let clamped = rssi.max(-100).min(-40);
        let normalized = (clamped + 100) as f32 / 60.0;
        (normalized * 100.0) as u8
    }
}
