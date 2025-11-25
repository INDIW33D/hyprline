use gtk4::prelude::*;
use std::sync::Arc;
use crate::domain::battery_service::BatteryService;
use crate::domain::models::{BatteryInfo, BatteryStatus};

pub struct BatteryWidget {
    container: gtk4::Box,
    service: Arc<dyn BatteryService + Send + Sync>,
}

impl BatteryWidget {
    pub fn new(service: Arc<dyn BatteryService + Send + Sync>) -> Self {
        let container = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
        container.add_css_class("battery-widget");

        Self {
            container,
            service,
        }
    }

    pub fn widget(&self) -> &gtk4::Box {
        &self.container
    }

    pub fn update(&self) {
        // Очищаем контейнер
        while let Some(child) = self.container.first_child() {
            self.container.remove(&child);
        }

        // Получаем информацию о батарее
        if let Some(battery_info) = self.service.get_battery_info() {
            // Создаём иконку
            let icon = self.create_battery_icon(&battery_info);
            self.container.append(&icon);

            // Создаём лейбл с процентами
            let label = gtk4::Label::new(Some(&format!("{}%", battery_info.percentage)));
            label.add_css_class("battery-percentage");
            self.container.append(&label);

            // Устанавливаем tooltip с дополнительной информацией
            let tooltip = self.create_tooltip(&battery_info);
            self.container.set_tooltip_text(Some(&tooltip));

            // Устанавливаем CSS класс в зависимости от уровня заряда
            self.apply_battery_level_class(&battery_info);
        } else {
            // Батарея не обнаружена
            let label = gtk4::Label::new(Some("No Battery"));
            label.add_css_class("battery-not-found");
            self.container.append(&label);
        }
    }

    /// Создаёт иконку батареи в зависимости от статуса и уровня заряда
    fn create_battery_icon(&self, info: &BatteryInfo) -> gtk4::Label {
        let icon_text = match info.status {
            BatteryStatus::Charging => "󰂄", // Nerd Font: battery charging
            BatteryStatus::Full => "󰁹", // Nerd Font: battery full
            _ => {
                // Выбираем иконку в зависимости от уровня заряда
                match info.percentage {
                    90..=100 => "󰁹", // battery full
                    70..=89 => "󰂂", // battery 90%
                    50..=69 => "󰂀", // battery 70%
                    30..=49 => "󰁾", // battery 50%
                    10..=29 => "󰁼", // battery 30%
                    _ => "󰁺", // battery low/critical
                }
            }
        };

        let icon = gtk4::Label::new(Some(icon_text));
        icon.add_css_class("battery-icon");
        icon
    }

    /// Создаёт текст для tooltip
    fn create_tooltip(&self, info: &BatteryInfo) -> String {
        let mut tooltip = format!("Battery: {}%\n", info.percentage);

        tooltip.push_str(&format!(
            "Status: {}\n",
            match info.status {
                BatteryStatus::Charging => "Charging",
                BatteryStatus::Discharging => "Discharging",
                BatteryStatus::Full => "Full",
                BatteryStatus::NotCharging => "Not Charging",
                BatteryStatus::Unknown => "Unknown",
            }
        ));

        if let Some(minutes) = info.time_to_empty {
            let hours = minutes / 60;
            let mins = minutes % 60;
            tooltip.push_str(&format!("Time remaining: {}h {:02}m\n", hours, mins));
        }

        if let Some(minutes) = info.time_to_full {
            let hours = minutes / 60;
            let mins = minutes % 60;
            tooltip.push_str(&format!("Time to full: {}h {:02}m\n", hours, mins));
        }

        tooltip
    }

    /// Применяет CSS класс в зависимости от уровня заряда
    fn apply_battery_level_class(&self, info: &BatteryInfo) {
        // Удаляем предыдущие классы уровня
        self.container.remove_css_class("battery-critical");
        self.container.remove_css_class("battery-low");
        self.container.remove_css_class("battery-medium");
        self.container.remove_css_class("battery-high");
        self.container.remove_css_class("battery-full");
        self.container.remove_css_class("battery-charging");

        // Добавляем класс для зарядки
        if info.status == BatteryStatus::Charging {
            self.container.add_css_class("battery-charging");
        }

        // Добавляем класс уровня заряда
        match info.percentage {
            0..=10 => self.container.add_css_class("battery-critical"),
            11..=30 => self.container.add_css_class("battery-low"),
            31..=60 => self.container.add_css_class("battery-medium"),
            61..=90 => self.container.add_css_class("battery-high"),
            _ => self.container.add_css_class("battery-full"),
        }
    }
}

