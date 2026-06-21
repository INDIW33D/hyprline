pub mod bar_config;
pub mod widget_config;

pub use widget_config::{
    HyprlineConfig, WidgetConfig, WidgetType, WidgetPosition, 
    WidgetProfile, MonitorConfig, BarPadding,
    NotificationCenterConfig,
    get_config, save_config, subscribe_config_changes, notify_config_changed
};
