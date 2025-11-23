use gtk4::prelude::*;
use std::sync::Arc;
use crate::domain::datetime_service::DateTimeService;
use crate::domain::models::DateTimeConfig;
use crate::ui::calendar::CalendarWidget;

pub struct DateTimeWidget {
    button: gtk4::Button,
    service: Arc<dyn DateTimeService + Send + Sync>,
    config: DateTimeConfig,
}

impl DateTimeWidget {
    pub fn new(service: Arc<dyn DateTimeService + Send + Sync>, config: DateTimeConfig) -> Self {
        let button = gtk4::Button::new();
        button.add_css_class("datetime-button");

        let label = gtk4::Label::new(None);
        label.add_css_class("datetime-label");
        
        // Резервируем место под максимальную ширину строки времени
        let placeholder = service.estimated_width(&config);
        label.set_width_chars(placeholder.len() as i32);
        
        button.set_child(Some(&label));

        // При клике показываем календарь
        let button_weak = button.downgrade();
        button.connect_clicked(move |_| {
            if let Some(btn) = button_weak.upgrade() {
                CalendarWidget::show(&btn);
            }
        });

        let widget = Self {
            button,
            service,
            config,
        };

        widget.update_time();
        widget
    }

    pub fn widget(&self) -> &gtk4::Button {
        &self.button
    }

    pub fn update_time(&self) {
        let time_str = self.service.format_current(&self.config);

        if let Some(label) = self.button.child().and_downcast::<gtk4::Label>() {
            label.set_text(&time_str);
        }
    }
}

