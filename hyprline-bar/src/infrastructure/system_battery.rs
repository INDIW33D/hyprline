use crate::domain::battery_service::BatteryService;
use crate::domain::models::{BatteryInfo, BatteryStatus};
use std::sync::{Arc, Mutex};
use async_channel::Sender;
use zbus::Connection;
use zbus::fdo::PropertiesProxy;
use futures::stream::StreamExt;

pub struct SystemBatteryService {
    cached_info: Arc<Mutex<Option<BatteryInfo>>>,
    battery_path: Arc<Mutex<Option<String>>>,
}

impl SystemBatteryService {
    pub fn new() -> Self {
        Self {
            cached_info: Arc::new(Mutex::new(None)),
            battery_path: Arc::new(Mutex::new(None)),
        }
    }

    /// Запускает мониторинг событий батареи и возвращает канал для получения событий
    pub fn start_monitoring(&self, tx: Sender<()>) {
        let cached_info = self.cached_info.clone();
        let battery_path = self.battery_path.clone();

        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async {
                if let Err(e) = Self::monitor_battery_events(tx, cached_info, battery_path).await {
                    eprintln!("[Battery] Error monitoring battery events: {}", e);
                }
            });
        });
    }

    /// Мониторинг событий батареи через UPower D-Bus
    async fn monitor_battery_events(
        tx: Sender<()>,
        cached_info: Arc<Mutex<Option<BatteryInfo>>>,
        battery_path_arc: Arc<Mutex<Option<String>>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        println!("[Battery] Starting UPower D-Bus monitoring...");

        let conn = Connection::system().await?;

        // Находим устройство батареи
        let battery_path = Self::find_battery_device(&conn).await?;
        println!("[Battery] ✓ Found battery device: {}", battery_path);

        // Сохраняем путь к батарее
        *battery_path_arc.lock().unwrap() = Some(battery_path.clone());

        // Получаем начальное состояние
        if let Some(info) = Self::fetch_battery_info(&conn, &battery_path).await {
            *cached_info.lock().unwrap() = Some(info);
        }

        // Создаём proxy для получения сигналов изменения свойств
        let properties = PropertiesProxy::builder(&conn)
            .destination("org.freedesktop.UPower")?
            .path(battery_path.clone())?
            .build()
            .await?;

        println!("[Battery] ✓ Listening for battery property changes");

        // Подписываемся на изменения свойств
        let mut stream = properties.receive_properties_changed().await?;

        while let Some(_signal) = stream.next().await {
            println!("[Battery] ⚡ Battery property changed event received");

            // Обновляем кэш с новыми данными
            if let Some(info) = Self::fetch_battery_info(&conn, &battery_path).await {
                *cached_info.lock().unwrap() = Some(info);
            }

            // Отправляем событие в UI
            if let Err(e) = tx.send(()).await {
                eprintln!("[Battery] Failed to send event: {}", e);
            }
        }

        Ok(())
    }

    /// Поиск батареи через UPower D-Bus
    async fn find_battery_device(conn: &Connection) -> Result<String, Box<dyn std::error::Error>> {
        use zbus::zvariant::Value;
        use zbus::names::InterfaceName;

        // Подключаемся к UPower
        let proxy = zbus::Proxy::new(
            conn,
            "org.freedesktop.UPower",
            "/org/freedesktop/UPower",
            "org.freedesktop.UPower",
        ).await?;

        // Получаем список устройств
        let devices: Vec<zbus::zvariant::OwnedObjectPath> = proxy
            .call("EnumerateDevices", &())
            .await?;

        // Ищем батарею
        for device in devices {
            let device_proxy = PropertiesProxy::builder(conn)
                .destination("org.freedesktop.UPower")?
                .path(device.clone())?
                .build()
                .await?;

            // Проверяем тип устройства (Type = 2 означает Battery)
            let interface: InterfaceName<'_> = "org.freedesktop.UPower.Device".try_into()?;
            if let Ok(device_type) = device_proxy.get(interface, "Type").await {
                let value: Value = device_type.try_into()?;
                if let Value::U32(type_val) = value {
                    if type_val == 2 {
                        return Ok(device.as_str().to_string());
                    }
                }
            }
        }

        Err("No battery device found".into())
    }

    /// Получение информации о батарее через UPower D-Bus
    async fn fetch_battery_info(conn: &Connection, battery_path: &str) -> Option<BatteryInfo> {
        use zbus::names::InterfaceName;

        let battery_path_obj: zbus::zvariant::ObjectPath = battery_path.try_into().ok()?;
        let properties = PropertiesProxy::builder(conn)
            .destination("org.freedesktop.UPower").ok()?
            .path(battery_path_obj).ok()?
            .build()
            .await
            .ok()?;

        let interface: InterfaceName<'_> = "org.freedesktop.UPower.Device".try_into().ok()?;

        // Читаем процент заряда
        let percentage_value = properties.get(interface, "Percentage").await.ok()?;
        let percentage_double: f64 = percentage_value.try_into().ok()?;
        let percentage = percentage_double.round() as u8;

        // Читаем статус
        let interface: InterfaceName<'_> = "org.freedesktop.UPower.Device".try_into().ok()?;
        let state_value = properties.get(interface, "State").await.ok()?;
        let state: u32 = state_value.try_into().ok()?;

        let status = match state {
            1 => BatteryStatus::Charging,
            2 => BatteryStatus::Discharging,
            3 => BatteryStatus::Discharging,
            4 => BatteryStatus::Full,
            5 => BatteryStatus::NotCharging,
            6 => BatteryStatus::NotCharging,
            _ => BatteryStatus::Unknown,
        };

        // Читаем время до разрядки/зарядки
        let interface: InterfaceName<'_> = "org.freedesktop.UPower.Device".try_into().ok()?;
        let time_to_empty_value = properties.get(interface, "TimeToEmpty").await.ok()?;
        let time_to_empty_secs: i64 = time_to_empty_value.try_into().ok()?;
        let time_to_empty = if time_to_empty_secs > 0 {
            Some((time_to_empty_secs / 60) as u32)
        } else {
            None
        };

        let interface: InterfaceName<'_> = "org.freedesktop.UPower.Device".try_into().ok()?;
        let time_to_full_value = properties.get(interface, "TimeToFull").await.ok()?;
        let time_to_full_secs: i64 = time_to_full_value.try_into().ok()?;
        let time_to_full = if time_to_full_secs > 0 {
            Some((time_to_full_secs / 60) as u32)
        } else {
            None
        };

        Some(BatteryInfo {
            percentage: percentage.min(100),
            status,
            time_to_empty,
            time_to_full,
        })
    }
}

impl BatteryService for SystemBatteryService {
    fn get_battery_info(&self) -> Option<BatteryInfo> {
        // Возвращаем закэшированное значение (обновляется через события)
        self.cached_info.lock().unwrap().clone()
    }
}

