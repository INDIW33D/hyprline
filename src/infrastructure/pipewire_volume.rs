use crate::domain::models::VolumeInfo;
use crate::domain::volume_service::VolumeService;
use std::process::Command;
use std::sync::{Arc, Mutex};
use async_channel::Sender;
use std::thread;

pub struct PipewireVolume {
    current_info: Arc<Mutex<Option<VolumeInfo>>>,
    update_txs: Arc<Mutex<Vec<Sender<()>>>>,
}

impl PipewireVolume {
    pub fn new() -> Self {
        Self {
            current_info: Arc::new(Mutex::new(None)),
            update_txs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Запускает мониторинг изменений громкости через PipeWire
    pub fn start_monitoring(&mut self, update_tx: Sender<()>) {
        self.update_txs.lock().unwrap().push(update_tx.clone());
        let current_info = Arc::clone(&self.current_info);
        let update_txs = Arc::clone(&self.update_txs);

        // Сначала получаем текущее состояние
        if let Some(info) = Self::get_volume_info_internal() {
            *current_info.lock().unwrap() = Some(info);
        }

        // Запускаем фоновый поток для мониторинга только один раз
        if self.update_txs.lock().unwrap().len() == 1 {
            thread::spawn(move || {
                Self::monitor_pipewire_events(current_info, update_txs);
            });
        }
    }

    /// Мониторинг событий PipeWire через API с подпиской на изменения
    fn monitor_pipewire_events(current_info: Arc<Mutex<Option<VolumeInfo>>>, update_txs: Arc<Mutex<Vec<Sender<()>>>>) {
        use pipewire as pw;

        // Инициализируем PipeWire
        pw::init();

        let mainloop = pw::main_loop::MainLoop::new(None).expect("Failed to create PipeWire mainloop");
        let context = pw::context::Context::new(&mainloop).expect("Failed to create PipeWire context");
        let core = context.connect(None).expect("Failed to connect to PipeWire");
        let registry = core.get_registry().expect("Failed to get registry");

        let current_info_global = Arc::clone(&current_info);
        let update_txs_global = Arc::clone(&update_txs);

        let current_info_remove = Arc::clone(&current_info);
        let update_txs_remove = Arc::clone(&update_txs);

        // Подписываемся на глобальные события (добавление/удаление объектов)
        let _listener = registry
            .add_listener_local()
            .global(move |_global| {
                // Проверяем изменения при любом событии в реестре
                if let Some(info) = Self::get_volume_info_internal() {
                    let mut current = current_info_global.lock().unwrap();
                    if current.as_ref() != Some(&info) {
                        *current = Some(info);
                        // Отправляем во все каналы
                        for tx in update_txs_global.lock().unwrap().iter() {
                            let _ = tx.try_send(());
                        }
                    }
                }
            })
            .global_remove(move |_id| {
                // Проверяем изменения при удалении объектов
                if let Some(info) = Self::get_volume_info_internal() {
                    let mut current = current_info_remove.lock().unwrap();
                    if current.as_ref() != Some(&info) {
                        *current = Some(info);
                        // Отправляем во все каналы
                        for tx in update_txs_remove.lock().unwrap().iter() {
                            let _ = tx.try_send(());
                        }
                    }
                }
            })
            .register();

        mainloop.run();
    }

    /// Парсит вывод wpctl get-volume для получения громкости
    fn parse_volume_output(output: &str) -> Option<VolumeInfo> {
        // Формат вывода: "Volume: 0.45 [MUTED]" или "Volume: 0.45"
        let parts: Vec<&str> = output.split_whitespace().collect();

        if parts.len() < 2 {
            return None;
        }

        // Парсим значение громкости
        let volume_str = parts[1];
        let volume_float: f32 = volume_str.parse().ok()?;
        let volume = (volume_float * 100.0).round() as u8;

        // Проверяем наличие [MUTED]
        let muted = output.contains("[MUTED]");

        Some(VolumeInfo { volume, muted })
    }

    /// Получает ID дефолтного sink (аудио выхода)
    fn get_default_sink_id() -> Option<String> {
        // Используем @DEFAULT_AUDIO_SINK@ как универсальный идентификатор
        Some("@DEFAULT_AUDIO_SINK@".to_string())
    }

    /// Внутренний метод для получения информации о громкости (используется для мониторинга)
    fn get_volume_info_internal() -> Option<VolumeInfo> {
        let sink_id = Self::get_default_sink_id()?;

        let output = Command::new("wpctl")
            .args(&["get-volume", &sink_id])
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        Self::parse_volume_output(&stdout)
    }
}

impl VolumeService for PipewireVolume {
    fn get_volume_info(&self) -> Option<VolumeInfo> {
        // Возвращаем кэшированное значение, которое обновляется через мониторинг
        self.current_info.lock().unwrap().clone()
    }

    fn set_volume(&self, volume: u8) -> Result<(), String> {
        let volume = volume.min(100);
        let volume_float = volume as f32 / 100.0;

        let sink_id = Self::get_default_sink_id()
            .ok_or_else(|| "Failed to get default sink ID".to_string())?;

        let output = Command::new("wpctl")
            .args(&["set-volume", &sink_id, &format!("{:.2}", volume_float)])
            .output()
            .map_err(|e| format!("Failed to execute wpctl: {}", e))?;

        if !output.status.success() {
            return Err(format!("wpctl failed with status: {}", output.status));
        }

        // Обновляем кэшированное значение
        if let Some(info) = Self::get_volume_info_internal() {
            *self.current_info.lock().unwrap() = Some(info);
            // Отправляем уведомление об обновлении во все каналы
            for tx in self.update_txs.lock().unwrap().iter() {
                let _ = tx.try_send(());
            }
        }

        Ok(())
    }

    fn toggle_mute(&self) -> Result<(), String> {
        let sink_id = Self::get_default_sink_id()
            .ok_or_else(|| "Failed to get default sink ID".to_string())?;

        let output = Command::new("wpctl")
            .args(&["set-mute", &sink_id, "toggle"])
            .output()
            .map_err(|e| format!("Failed to execute wpctl: {}", e))?;

        if !output.status.success() {
            return Err(format!("wpctl failed with status: {}", output.status));
        }

        // Обновляем кэшированное значение
        if let Some(info) = Self::get_volume_info_internal() {
            *self.current_info.lock().unwrap() = Some(info);
            // Отправляем уведомление об обновлении во все каналы
            for tx in self.update_txs.lock().unwrap().iter() {
                let _ = tx.try_send(());
            }
        }

        Ok(())
    }

    fn set_mute(&self, muted: bool) -> Result<(), String> {
        let sink_id = Self::get_default_sink_id()
            .ok_or_else(|| "Failed to get default sink ID".to_string())?;

        let mute_arg = if muted { "1" } else { "0" };

        let output = Command::new("wpctl")
            .args(&["set-mute", &sink_id, mute_arg])
            .output()
            .map_err(|e| format!("Failed to execute wpctl: {}", e))?;

        if !output.status.success() {
            return Err(format!("wpctl failed with status: {}", output.status));
        }

        // Обновляем кэшированное значение
        if let Some(info) = Self::get_volume_info_internal() {
            *self.current_info.lock().unwrap() = Some(info);
            // Отправляем уведомление об обновлении во все каналы
            for tx in self.update_txs.lock().unwrap().iter() {
                let _ = tx.try_send(());
            }
        }

        Ok(())
    }
}

/// Создает канал для уведомлений об изменении громкости
pub fn create_volume_channel() -> (Sender<()>, async_channel::Receiver<()>) {
    async_channel::unbounded()
}

