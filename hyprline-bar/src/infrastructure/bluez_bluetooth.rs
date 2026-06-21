use crate::domain::bluetooth_service::BluetoothService;
use crate::domain::models::{BluetoothDevice, BluetoothDeviceType, BluetoothInfo};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use zbus::blocking::connection;
use zbus::zvariant::{ObjectPath, OwnedObjectPath, OwnedValue, Value};

pub struct BluezBluetoothService {
    info: Arc<Mutex<Option<BluetoothInfo>>>,
    /// Persistent D-Bus connection — BlueZ ties discovery lifetime to the connection
    /// that called StartDiscovery. If we drop it, discovery stops immediately.
    connection: Arc<Mutex<Option<zbus::blocking::Connection>>>,
}

impl BluezBluetoothService {
    pub fn new() -> Self {
        // Не делаем блокирующих D-Bus вызовов в конструкторе —
        // начальное состояние будет получено при первом периодическом обновлении
        Self {
            info: Arc::new(Mutex::new(None)),
            connection: Arc::new(Mutex::new(None)),
        }
    }

    /// Получает или создаёт persistent D-Bus connection
    fn get_connection(&self) -> Result<zbus::blocking::Connection, String> {
        let mut conn_guard = self.connection.lock().unwrap();
        if let Some(ref conn) = *conn_guard {
            return Ok(conn.clone());
        }

        let conn = connection::Connection::system()
            .map_err(|e| format!("Failed to connect to D-Bus: {}", e))?;
        *conn_guard = Some(conn.clone());
        Ok(conn)
    }

    /// Получает путь к первому Bluetooth адаптеру
    fn get_adapter_path(&self) -> Result<String, String> {
        let objects = self.get_managed_objects()?;

        for (path, interfaces) in &objects {
            if interfaces.contains_key("org.bluez.Adapter1") {
                return Ok(path.to_string());
            }
        }

        Err("No Bluetooth adapter found".to_string())
    }

    /// Получает все managed objects через ObjectManager
    fn get_managed_objects(
        &self,
    ) -> Result<HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>>, String>
    {
        let conn = self.get_connection()?;

        let proxy = zbus::blocking::Proxy::new(
            &conn,
            "org.bluez",
            "/",
            "org.freedesktop.DBus.ObjectManager",
        )
        .map_err(|e| format!("Failed to create ObjectManager proxy: {}", e))?;

        let result: HashMap<OwnedObjectPath, HashMap<String, HashMap<String, OwnedValue>>> = proxy
            .call("GetManagedObjects", &())
            .map_err(|e| format!("Failed to call GetManagedObjects: {}", e))?;

        Ok(result)
    }

    /// Парсит тип устройства из icon строки BlueZ
    fn parse_device_type(icon: &str) -> BluetoothDeviceType {
        match icon {
            s if s.contains("audio") || s.contains("headset") || s.contains("headphone") => {
                BluetoothDeviceType::Audio
            }
            s if s.contains("phone") => BluetoothDeviceType::Phone,
            s if s.contains("computer") => BluetoothDeviceType::Computer,
            s if s.contains("keyboard") => BluetoothDeviceType::Keyboard,
            s if s.contains("mouse") || s.contains("input-mouse") => BluetoothDeviceType::Mouse,
            s if s.contains("game") || s.contains("joystick") || s.contains("joypad") => {
                BluetoothDeviceType::Gamepad
            }
            _ => BluetoothDeviceType::Other,
        }
    }

    /// Создаёт blocking proxy для адаптера
    fn create_adapter_proxy<'a>(
        conn: &'a zbus::blocking::Connection,
        adapter_path: &str,
    ) -> Result<zbus::blocking::Proxy<'a>, String> {
        zbus::blocking::Proxy::new(
            conn,
            "org.bluez",
            ObjectPath::from_string_unchecked(adapter_path.to_string()),
            "org.bluez.Adapter1",
        )
        .map_err(|e| format!("Failed to create adapter proxy: {}", e))
    }

    /// Получает bool свойство адаптера
    fn get_adapter_bool(&self, adapter_path: &str, property: &str) -> Result<bool, String> {
        let conn = self.get_connection()?;
        let proxy = Self::create_adapter_proxy(&conn, adapter_path)?;
        proxy
            .get_property::<bool>(property)
            .map_err(|e| format!("Failed to get property {}: {}", property, e))
    }

    /// Получает String свойство адаптера
    fn get_adapter_string(&self, adapter_path: &str, property: &str) -> Result<String, String> {
        let conn = self.get_connection()?;
        let proxy = Self::create_adapter_proxy(&conn, adapter_path)?;
        proxy
            .get_property::<String>(property)
            .map_err(|e| format!("Failed to get property {}: {}", property, e))
    }

    /// Собирает полную информацию о Bluetooth
    fn fetch_bluetooth_info(&self) -> Option<BluetoothInfo> {
        let adapter_path = self.get_adapter_path().ok()?;

        let powered: bool = self
            .get_adapter_bool(&adapter_path, "Powered")
            .unwrap_or(false);
        let discovering: bool = self
            .get_adapter_bool(&adapter_path, "Discovering")
            .unwrap_or(false);
        let adapter_name: String = self
            .get_adapter_string(&adapter_path, "Alias")
            .unwrap_or_else(|_| "Bluetooth".to_string());

        let all_devices = self.fetch_devices_internal(&adapter_path);
        let connected_devices = all_devices
            .iter()
            .filter(|d| d.connected)
            .cloned()
            .collect();

        Some(BluetoothInfo {
            powered,
            discovering,
            adapter_name,
            connected_devices,
        })
    }

    /// Получает список устройств для данного адаптера
    fn fetch_devices_internal(&self, adapter_path: &str) -> Vec<BluetoothDevice> {
        let objects = match self.get_managed_objects() {
            Ok(o) => o,
            Err(e) => {
                eprintln!("[Bluetooth] Failed to get managed objects: {}", e);
                return Vec::new();
            }
        };

        let mut devices = Vec::new();

        for (path, interfaces) in &objects {
            // Устройства находятся под адаптером: /org/bluez/hci0/dev_XX_XX_XX
            if !path.as_str().starts_with(adapter_path) {
                continue;
            }

            if let Some(dev_props) = interfaces.get("org.bluez.Device1") {
                let battery_props = interfaces.get("org.bluez.Battery1");
                let device = Self::parse_device(dev_props, battery_props);
                if let Some(device) = device {
                    devices.push(device);
                }
            }
        }

        // Сортируем: подключённые первые, потом спаренные, потом остальные
        devices.sort_by(|a, b| {
            b.connected
                .cmp(&a.connected)
                .then(b.paired.cmp(&a.paired))
                .then(a.name.cmp(&b.name))
        });

        devices
    }

    /// Парсит BluetoothDevice из D-Bus properties
    fn parse_device(
        dev_props: &HashMap<String, OwnedValue>,
        battery_props: Option<&HashMap<String, OwnedValue>>,
    ) -> Option<BluetoothDevice> {
        let address = Self::extract_string(dev_props, "Address")?;

        let name = Self::extract_string(dev_props, "Name").unwrap_or_else(|| address.clone());

        let alias = Self::extract_string(dev_props, "Alias").unwrap_or_else(|| name.clone());

        let paired = Self::extract_bool(dev_props, "Paired").unwrap_or(false);
        let connected = Self::extract_bool(dev_props, "Connected").unwrap_or(false);
        let trusted = Self::extract_bool(dev_props, "Trusted").unwrap_or(false);

        let icon = Self::extract_string(dev_props, "Icon");
        let device_type = icon
            .as_deref()
            .map(Self::parse_device_type)
            .unwrap_or(BluetoothDeviceType::Other);

        let rssi = Self::extract_i16(dev_props, "RSSI");

        // Пробуем получить уровень заряда из org.bluez.Battery1
        let battery_percentage =
            battery_props.and_then(|bat_props| Self::extract_u8(bat_props, "Percentage"));

        Some(BluetoothDevice {
            address,
            name,
            alias,
            paired,
            connected,
            trusted,
            icon,
            rssi,
            device_type,
            battery_percentage,
        })
    }

    /// Получает D-Bus object path устройства по MAC-адресу
    fn get_device_path(&self, address: &str) -> Result<String, String> {
        let adapter_path = self.get_adapter_path()?;
        // BlueZ формат: /org/bluez/hci0/dev_AA_BB_CC_DD_EE_FF
        let dev_part = address.replace(':', "_");
        Ok(format!("{}/dev_{}", adapter_path, dev_part))
    }

    // === Хелперы для извлечения значений из HashMap<String, OwnedValue> ===

    fn extract_string(props: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
        let owned = props.get(key)?;
        let val: &Value<'_> = owned;
        if let Value::Str(s) = val {
            Some(s.to_string())
        } else {
            None
        }
    }

    fn extract_bool(props: &HashMap<String, OwnedValue>, key: &str) -> Option<bool> {
        let owned = props.get(key)?;
        let val: &Value<'_> = owned;
        if let Value::Bool(b) = val {
            Some(*b)
        } else {
            None
        }
    }

    fn extract_i16(props: &HashMap<String, OwnedValue>, key: &str) -> Option<i16> {
        let owned = props.get(key)?;
        let val: &Value<'_> = owned;
        match val {
            Value::I16(v) => Some(*v),
            _ => None,
        }
    }

    fn extract_u8(props: &HashMap<String, OwnedValue>, key: &str) -> Option<u8> {
        let owned = props.get(key)?;
        let val: &Value<'_> = owned;
        match val {
            Value::U8(v) => Some(*v),
            _ => None,
        }
    }
}

impl BluetoothService for BluezBluetoothService {
    fn get_bluetooth_info(&self) -> Option<BluetoothInfo> {
        // Обновляем информацию при каждом вызове
        let info = self.fetch_bluetooth_info();
        *self.info.lock().unwrap() = info.clone();
        info
    }

    fn get_devices(&self) -> Vec<BluetoothDevice> {
        let adapter_path = match self.get_adapter_path() {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[Bluetooth] Failed to get adapter: {}", e);
                return Vec::new();
            }
        };
        self.fetch_devices_internal(&adapter_path)
    }

    fn get_connected_devices(&self) -> Vec<BluetoothDevice> {
        self.get_devices()
            .into_iter()
            .filter(|d| d.connected)
            .collect()
    }

    fn set_powered(&self, enabled: bool) -> Result<(), String> {
        let adapter_path = self.get_adapter_path()?;
        let conn = self.get_connection()?;

        let proxy = Self::create_adapter_proxy(&conn, &adapter_path)?;

        proxy
            .set_property("Powered", enabled)
            .map_err(|e| format!("Failed to set Powered: {}", e))?;

        eprintln!(
            "[Bluetooth] Adapter powered {}",
            if enabled { "on" } else { "off" }
        );
        Ok(())
    }

    fn is_powered(&self) -> bool {
        self.get_adapter_path()
            .and_then(|path| self.get_adapter_bool(&path, "Powered"))
            .unwrap_or(false)
    }

    fn start_discovery(&self) -> Result<(), String> {
        let adapter_path = self.get_adapter_path()?;
        let conn = self.get_connection()?;

        let proxy = Self::create_adapter_proxy(&conn, &adapter_path)?;

        let _: () = proxy
            .call("StartDiscovery", &())
            .map_err(|e| format!("Failed to start discovery: {}", e))?;

        eprintln!("[Bluetooth] Discovery started");
        Ok(())
    }

    fn stop_discovery(&self) -> Result<(), String> {
        let adapter_path = self.get_adapter_path()?;
        let conn = self.get_connection()?;

        let proxy = Self::create_adapter_proxy(&conn, &adapter_path)?;

        let _: () = proxy
            .call("StopDiscovery", &())
            .map_err(|e| format!("Failed to stop discovery: {}", e))?;

        eprintln!("[Bluetooth] Discovery stopped");
        Ok(())
    }

    fn connect_device(&self, address: &str) -> Result<(), String> {
        let device_path = self.get_device_path(address)?;
        let conn = self.get_connection()?;

        let proxy = zbus::blocking::Proxy::new(
            &conn,
            "org.bluez",
            ObjectPath::from_string_unchecked(device_path),
            "org.bluez.Device1",
        )
        .map_err(|e| format!("Failed to create device proxy: {}", e))?;

        let _: () = proxy
            .call("Connect", &())
            .map_err(|e| format!("Failed to connect to device {}: {}", address, e))?;

        eprintln!("[Bluetooth] Connected to {}", address);
        Ok(())
    }

    fn disconnect_device(&self, address: &str) -> Result<(), String> {
        let device_path = self.get_device_path(address)?;
        let conn = self.get_connection()?;

        let proxy = zbus::blocking::Proxy::new(
            &conn,
            "org.bluez",
            ObjectPath::from_string_unchecked(device_path),
            "org.bluez.Device1",
        )
        .map_err(|e| format!("Failed to create device proxy: {}", e))?;

        let _: () = proxy
            .call("Disconnect", &())
            .map_err(|e| format!("Failed to disconnect device {}: {}", address, e))?;

        eprintln!("[Bluetooth] Disconnected from {}", address);
        Ok(())
    }

    fn pair_device(&self, address: &str) -> Result<(), String> {
        let device_path = self.get_device_path(address)?;
        let conn = self.get_connection()?;

        let proxy = zbus::blocking::Proxy::new(
            &conn,
            "org.bluez",
            ObjectPath::from_string_unchecked(device_path),
            "org.bluez.Device1",
        )
        .map_err(|e| format!("Failed to create device proxy: {}", e))?;

        let _: () = proxy
            .call("Pair", &())
            .map_err(|e| format!("Failed to pair device {}: {}", address, e))?;

        eprintln!("[Bluetooth] Paired with {}", address);
        Ok(())
    }

    fn remove_device(&self, address: &str) -> Result<(), String> {
        let adapter_path = self.get_adapter_path()?;
        let device_path = self.get_device_path(address)?;
        let conn = self.get_connection()?;

        let proxy = Self::create_adapter_proxy(&conn, &adapter_path)?;

        let device_obj_path = ObjectPath::from_string_unchecked(device_path);

        let _: () = proxy
            .call("RemoveDevice", &(device_obj_path,))
            .map_err(|e| format!("Failed to remove device {}: {}", address, e))?;

        eprintln!("[Bluetooth] Removed device {}", address);
        Ok(())
    }

    fn trust_device(&self, address: &str) -> Result<(), String> {
        let device_path = self.get_device_path(address)?;
        let conn = self.get_connection()?;

        let proxy = zbus::blocking::Proxy::new(
            &conn,
            "org.bluez",
            ObjectPath::from_string_unchecked(device_path),
            "org.bluez.Device1",
        )
        .map_err(|e| format!("Failed to create device proxy: {}", e))?;

        proxy
            .set_property("Trusted", true)
            .map_err(|e| format!("Failed to trust device {}: {}", address, e))?;

        eprintln!("[Bluetooth] Trusted device {}", address);
        Ok(())
    }
}
