use crate::domain::brightness_service::BrightnessService;
use std::sync::Arc;
use zbus::{Connection, proxy};
use parking_lot::Mutex;

#[proxy(
    interface = "org.lumen.Brightness",
    default_service = "org.lumen.Brightness",
    default_path = "/org/lumen/Brightness"
)]
trait Lumen {
    /// Получить текущую яркость (0.0 - 1.0)
    #[zbus(name = "GetBrightness")]
    async fn get_brightness(&self) -> zbus::Result<f64>;

    /// Установить яркость (0.0 - 1.0)
    #[zbus(name = "SetBrightness")]
    async fn set_brightness(&self, value: f64) -> zbus::Result<()>;

    /// Увеличить яркость на процент (0.0 - 1.0)
    #[zbus(name = "Increase")]
    async fn increase(&self, percentage: f64) -> zbus::Result<()>;

    /// Уменьшить яркость на процент (0.0 - 1.0)
    #[zbus(name = "Decrease")]
    async fn decrease(&self, percentage: f64) -> zbus::Result<()>;

    /// Включить автоматическую регулировку
    #[zbus(name = "EnableAuto")]
    async fn enable_auto(&self) -> zbus::Result<()>;

    /// Отключить автоматическую регулировку
    #[zbus(name = "DisableAuto")]
    async fn disable_auto(&self) -> zbus::Result<()>;

    /// Проверить состояние автоматической регулировки
    #[zbus(name = "IsAutoEnabled")]
    async fn is_auto_enabled(&self) -> zbus::Result<bool>;

    /// Включить обучение
    #[zbus(name = "EnableLearning")]
    async fn enable_learning(&self) -> zbus::Result<()>;

    /// Отключить обучение
    #[zbus(name = "DisableLearning")]
    async fn disable_learning(&self) -> zbus::Result<()>;

    /// Проверить состояние обучения
    #[zbus(name = "IsLearningEnabled")]
    async fn is_learning_enabled(&self) -> zbus::Result<bool>;
    
    /// Сигнал изменения яркости
    #[zbus(signal, name = "BrightnessChanged")]
    fn brightness_changed(&self, value: f64) -> zbus::Result<()>;
    
    /// Сигнал изменения состояния автоматической регулировки
    #[zbus(signal, name = "AutoEnabledChanged")]
    fn auto_enabled_changed(&self, enabled: bool) -> zbus::Result<()>;
}

pub struct LumenBrightnessService {
    connection: Connection,
    callback: Arc<Mutex<Option<Arc<dyn Fn(u32) + Send + Sync>>>>,
}

impl LumenBrightnessService {
    pub fn new() -> Result<Self, String> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        let connection = rt.block_on(async {
            Connection::session().await
                .map_err(|e| format!("Failed to connect to session bus: {}", e))
        })?;

        let service = Self {
            connection,
            callback: Arc::new(Mutex::new(None)),
        };

        Ok(service)
    }

    async fn get_proxy(&self) -> Result<LumenProxy<'_>, String> {
        LumenProxy::new(&self.connection).await
            .map_err(|e| format!("Failed to create Lumen proxy: {}", e))
    }

    pub fn start_signal_monitoring(self: Arc<Self>) {
        let callback = self.callback.clone();
        let connection = self.connection.clone();
        
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async move {
                if let Ok(proxy) = LumenProxy::new(&connection).await {
                    if let Ok(mut stream) = proxy.receive_brightness_changed().await {
                        eprintln!("[Brightness] ✓ Successfully subscribed to BrightnessChanged signal");
                        loop {
                            use futures_util::StreamExt;
                            if let Some(signal) = stream.next().await {
                                if let Ok(args) = signal.args() {
                                    // Конвертируем f64 (0.0-1.0) в u32 (0-100)
                                    let brightness = (args.value * 100.0).round() as u32;
                                    eprintln!("[Brightness] 📡 Received BrightnessChanged signal: {}%", brightness);
                                    if let Some(cb) = callback.lock().as_ref() {
                                        cb(brightness);
                                    }
                                }
                            }
                        }
                    } else {
                        eprintln!("[Brightness] ✗ Failed to subscribe to BrightnessChanged signal");
                    }
                } else {
                    eprintln!("[Brightness] ✗ Failed to create proxy for signal monitoring");
                }
            });
        });
    }
}

impl BrightnessService for LumenBrightnessService {
    fn get_brightness(&self) -> Result<u32, String> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        rt.block_on(async {
            let proxy = self.get_proxy().await?;
            let brightness = proxy.get_brightness().await
                .map_err(|e| format!("Failed to get brightness: {}", e))?;
            // Конвертируем f64 (0.0-1.0) в u32 (0-100)
            Ok((brightness * 100.0).round() as u32)
        })
    }

    fn set_brightness(&self, value: u32) -> Result<(), String> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        rt.block_on(async {
            let proxy = self.get_proxy().await?;
            // Конвертируем u32 (0-100) в f64 (0.0-1.0)
            let brightness = (value as f64) / 100.0;
            proxy.set_brightness(brightness).await
                .map_err(|e| format!("Failed to set brightness: {}", e))
        })
    }

    fn increase_brightness(&self, percent: u32) -> Result<(), String> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        rt.block_on(async {
            let proxy = self.get_proxy().await?;
            // Конвертируем u32 (0-100) в f64 (0.0-1.0)
            let percentage = (percent as f64) / 100.0;
            proxy.increase(percentage).await
                .map_err(|e| format!("Failed to increase brightness: {}", e))
        })
    }

    fn decrease_brightness(&self, percent: u32) -> Result<(), String> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        rt.block_on(async {
            let proxy = self.get_proxy().await?;
            // Конвертируем u32 (0-100) в f64 (0.0-1.0)
            let percentage = (percent as f64) / 100.0;
            proxy.decrease(percentage).await
                .map_err(|e| format!("Failed to decrease brightness: {}", e))
        })
    }

    fn enable_auto_adjustment(&self) -> Result<(), String> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        rt.block_on(async {
            let proxy = self.get_proxy().await?;
            proxy.enable_auto().await
                .map_err(|e| format!("Failed to enable auto adjustment: {}", e))
        })
    }

    fn disable_auto_adjustment(&self) -> Result<(), String> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        rt.block_on(async {
            let proxy = self.get_proxy().await?;
            proxy.disable_auto().await
                .map_err(|e| format!("Failed to disable auto adjustment: {}", e))
        })
    }

    fn is_auto_adjustment_enabled(&self) -> Result<bool, String> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        rt.block_on(async {
            let proxy = self.get_proxy().await?;
            proxy.is_auto_enabled().await
                .map_err(|e| format!("Failed to check auto adjustment: {}", e))
        })
    }

    fn subscribe_brightness_changed(&self, callback: Arc<dyn Fn(u32) + Send + Sync>) {
        *self.callback.lock() = Some(callback);
    }
}

