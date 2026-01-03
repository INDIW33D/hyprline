use crate::domain::models::{BatteryInfo, KeyboardLayout, TrayItem, VolumeInfo};
use std::sync::{Arc, Mutex, RwLock};

/// Тип callback-функции для обновления виджетов
type UpdateCallback = Box<dyn Fn() + Send + Sync>;

/// Структура для хранения списка callback-ов
struct Callbacks {
    callbacks: Vec<UpdateCallback>,
}

impl Callbacks {
    fn new() -> Self {
        Self { callbacks: Vec::new() }
    }

    fn add(&mut self, callback: UpdateCallback) {
        self.callbacks.push(callback);
    }

    fn notify_all(&self) {
        for callback in &self.callbacks {
            callback();
        }
    }
}

/// Общее состояние приложения, которое синхронизируется между всеми мониторами
pub struct SharedState {
    // Состояния
    pub battery_info: RwLock<Option<BatteryInfo>>,
    pub volume_info: RwLock<Option<VolumeInfo>>,
    pub tray_items: RwLock<Vec<TrayItem>>,
    pub keyboard_layout: RwLock<Option<KeyboardLayout>>,
    pub notification_count: RwLock<usize>,
    pub brightness: RwLock<u32>,

    // Callback-и для обновления UI
    battery_callbacks: Mutex<Callbacks>,
    volume_callbacks: Mutex<Callbacks>,
    tray_callbacks: Mutex<Callbacks>,
    keyboard_layout_callbacks: Mutex<Callbacks>,
    notification_callbacks: Mutex<Callbacks>,
    brightness_callbacks: Mutex<Callbacks>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            battery_info: RwLock::new(None),
            volume_info: RwLock::new(None),
            tray_items: RwLock::new(Vec::new()),
            keyboard_layout: RwLock::new(None),
            notification_count: RwLock::new(0),
            brightness: RwLock::new(100),
            battery_callbacks: Mutex::new(Callbacks::new()),
            volume_callbacks: Mutex::new(Callbacks::new()),
            tray_callbacks: Mutex::new(Callbacks::new()),
            keyboard_layout_callbacks: Mutex::new(Callbacks::new()),
            notification_callbacks: Mutex::new(Callbacks::new()),
            brightness_callbacks: Mutex::new(Callbacks::new()),
        }
    }

    // === Battery ===
    pub fn update_battery(&self, info: Option<BatteryInfo>) {
        *self.battery_info.write().unwrap() = info;
        self.battery_callbacks.lock().unwrap().notify_all();
    }

    pub fn get_battery(&self) -> Option<BatteryInfo> {
        self.battery_info.read().unwrap().clone()
    }

    pub fn subscribe_battery<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.battery_callbacks.lock().unwrap().add(Box::new(callback));
    }

    // === Volume ===
    pub fn update_volume(&self, info: Option<VolumeInfo>) {
        *self.volume_info.write().unwrap() = info;
        self.volume_callbacks.lock().unwrap().notify_all();
    }

    pub fn get_volume(&self) -> Option<VolumeInfo> {
        self.volume_info.read().unwrap().clone()
    }

    pub fn subscribe_volume<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.volume_callbacks.lock().unwrap().add(Box::new(callback));
    }

    // === Tray ===
    pub fn update_tray(&self, items: Vec<TrayItem>) {
        *self.tray_items.write().unwrap() = items;
        self.tray_callbacks.lock().unwrap().notify_all();
    }

    pub fn get_tray(&self) -> Vec<TrayItem> {
        self.tray_items.read().unwrap().clone()
    }

    pub fn subscribe_tray<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.tray_callbacks.lock().unwrap().add(Box::new(callback));
    }

    // === Keyboard Layout ===
    pub fn update_keyboard_layout(&self, layout: KeyboardLayout) {
        *self.keyboard_layout.write().unwrap() = Some(layout);
        self.keyboard_layout_callbacks.lock().unwrap().notify_all();
    }

    pub fn get_keyboard_layout(&self) -> Option<KeyboardLayout> {
        self.keyboard_layout.read().unwrap().clone()
    }

    pub fn subscribe_keyboard_layout<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.keyboard_layout_callbacks.lock().unwrap().add(Box::new(callback));
    }

    // === Notifications ===
    pub fn update_notifications(&self, count: usize) {
        *self.notification_count.write().unwrap() = count;
        self.notification_callbacks.lock().unwrap().notify_all();
    }

    pub fn get_notification_count(&self) -> usize {
        *self.notification_count.read().unwrap()
    }

    pub fn subscribe_notifications<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.notification_callbacks.lock().unwrap().add(Box::new(callback));
    }

    // === Brightness ===
    pub fn update_brightness(&self, brightness: u32) {
        *self.brightness.write().unwrap() = brightness;
        self.brightness_callbacks.lock().unwrap().notify_all();
    }

    pub fn get_brightness(&self) -> u32 {
        *self.brightness.read().unwrap()
    }

    pub fn subscribe_brightness<F>(&self, callback: F)
    where
        F: Fn() + Send + Sync + 'static,
    {
        self.brightness_callbacks.lock().unwrap().add(Box::new(callback));
    }
}

impl Default for SharedState {
    fn default() -> Self {
        Self::new()
    }
}

/// Глобальный синглтон shared state
static SHARED_STATE: std::sync::OnceLock<Arc<SharedState>> = std::sync::OnceLock::new();

pub fn get_shared_state() -> Arc<SharedState> {
    SHARED_STATE.get_or_init(|| Arc::new(SharedState::new())).clone()
}

