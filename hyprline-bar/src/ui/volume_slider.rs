use gtk4::prelude::*;
use gtk4::glib;
use std::cell::Cell;
use std::rc::Rc;

/// Кастомный вертикальный слайдер громкости с цветовой индикацией
pub struct VolumeSlider {
    drawing_area: gtk4::DrawingArea,
    current_volume: Rc<Cell<f64>>,
    is_muted: Rc<Cell<bool>>,
    is_interactive: bool,
}

impl VolumeSlider {
    /// Создаёт новый слайдер
    /// - `width`: ширина слайдера
    /// - `height`: высота слайдера
    /// - `interactive`: если true, можно менять значение кликом/drag/scroll
    pub fn new(width: i32, height: i32, interactive: bool) -> Self {
        let current_volume = Rc::new(Cell::new(50.0));
        let is_muted = Rc::new(Cell::new(false));
        
        let drawing_area = gtk4::DrawingArea::new();
        drawing_area.set_size_request(width, height);
        drawing_area.add_css_class("volume-custom-slider");

        // Отрисовка слайдера
        let volume_for_draw = Rc::clone(&current_volume);
        let muted_for_draw = Rc::clone(&is_muted);
        drawing_area.set_draw_func(move |_area, cr, w, h| {
            Self::draw_slider(cr, w, h, volume_for_draw.get(), muted_for_draw.get());
        });

        Self {
            drawing_area,
            current_volume,
            is_muted,
            is_interactive: interactive,
        }
    }

    /// Создаёт интерактивный слайдер с callback при изменении значения
    pub fn new_interactive<F>(width: i32, height: i32, on_change: F) -> Self
    where
        F: Fn(u8) + 'static,
    {
        let slider = Self::new(width, height, true);
        slider.setup_interactions(on_change);
        slider
    }

    /// Настраивает обработчики взаимодействия
    fn setup_interactions<F>(&self, on_change: F)
    where
        F: Fn(u8) + 'static,
    {
        let on_change = Rc::new(on_change);
        let is_dragging = Rc::new(Cell::new(false));
        let drag_start_volume = Rc::new(Cell::new(0.0f64));

        // Drag gesture
        let gesture = gtk4::GestureDrag::new();
        
        {
            let volume = Rc::clone(&self.current_volume);
            let dragging = Rc::clone(&is_dragging);
            let start_vol = Rc::clone(&drag_start_volume);
            let on_change = Rc::clone(&on_change);
            let area = self.drawing_area.downgrade();
            
            gesture.connect_drag_begin(move |gesture, _, y| {
                dragging.set(true);
                if let Some(area) = area.upgrade() {
                    let height = area.height() as f64;
                    let new_volume = ((height - y) / height * 100.0).clamp(0.0, 100.0);
                    start_vol.set(new_volume);
                    volume.set(new_volume);
                    on_change(new_volume as u8);
                    area.queue_draw();
                }
                gesture.set_state(gtk4::EventSequenceState::Claimed);
            });
        }
        
        {
            let volume = Rc::clone(&self.current_volume);
            let start_vol = Rc::clone(&drag_start_volume);
            let on_change = Rc::clone(&on_change);
            let area = self.drawing_area.downgrade();
            
            gesture.connect_drag_update(move |_, _, offset_y| {
                if let Some(area) = area.upgrade() {
                    let height = area.height() as f64;
                    let delta = -offset_y / height * 100.0;
                    let new_volume = (start_vol.get() + delta).clamp(0.0, 100.0);
                    volume.set(new_volume);
                    on_change(new_volume as u8);
                    area.queue_draw();
                }
            });
        }
        
        {
            let dragging = Rc::clone(&is_dragging);
            gesture.connect_drag_end(move |_, _, _| {
                dragging.set(false);
            });
        }
        
        self.drawing_area.add_controller(gesture);

        // Click gesture
        let click_gesture = gtk4::GestureClick::new();
        {
            let volume = Rc::clone(&self.current_volume);
            let on_change = Rc::clone(&on_change);
            let area = self.drawing_area.downgrade();
            
            click_gesture.connect_released(move |_, _, _, y| {
                if let Some(area) = area.upgrade() {
                    let height = area.height() as f64;
                    let new_volume = ((height - y) / height * 100.0).clamp(0.0, 100.0);
                    volume.set(new_volume);
                    on_change(new_volume as u8);
                    area.queue_draw();
                }
            });
        }
        self.drawing_area.add_controller(click_gesture);

        // Scroll controller
        let scroll_ctrl = gtk4::EventControllerScroll::new(gtk4::EventControllerScrollFlags::VERTICAL);
        {
            let volume = Rc::clone(&self.current_volume);
            let on_change = Rc::clone(&on_change);
            let area = self.drawing_area.downgrade();
            
            scroll_ctrl.connect_scroll(move |_, _, dy| {
                if let Some(area) = area.upgrade() {
                    let current = volume.get();
                    let new_volume = (current - dy * 5.0).clamp(0.0, 100.0);
                    volume.set(new_volume);
                    on_change(new_volume as u8);
                    area.queue_draw();
                }
                glib::Propagation::Stop
            });
        }
        self.drawing_area.add_controller(scroll_ctrl);
    }

    /// Отрисовка слайдера
    fn draw_slider(cr: &gtk4::cairo::Context, width: i32, height: i32, volume: f64, muted: bool) {
        let volume_frac = volume / 100.0;
        let w = width as f64;
        let h = height as f64;
        
        let padding = 4.0;
        let bar_width = w - padding * 2.0;
        let bar_height = h - padding * 2.0;
        let filled_height = bar_height * volume_frac;
        
        // Фон слайдера (тёмный)
        cr.set_source_rgba(0.15, 0.15, 0.15, 0.9);
        Self::rounded_rect(cr, padding, padding, bar_width, bar_height, 6.0);
        let _ = cr.fill();
        
        // Заполненная часть с цветом в зависимости от громкости
        if filled_height > 0.0 {
            if muted {
                // Серый цвет когда звук выключен
                cr.set_source_rgba(0.5, 0.5, 0.5, 0.7);
            } else {
                let (r, g, b) = Self::volume_to_color(volume_frac);
                cr.set_source_rgba(r, g, b, 1.0);
            }
            
            let y_start = padding + bar_height - filled_height;
            Self::rounded_rect_bottom(cr, padding, y_start, bar_width, filled_height, 6.0);
            let _ = cr.fill();
        }
        
        // Текст с процентами или иконка mute
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.9);
        cr.set_font_size(12.0);
        let text = if muted {
            "󰝟".to_string() // Muted icon
        } else {
            format!("{}%", volume as u8)
        };
        if let Ok(extents) = cr.text_extents(&text) {
            let x = (w - extents.width()) / 2.0;
            let y = h / 2.0 + extents.height() / 2.0;
            let _ = cr.move_to(x, y);
            let _ = cr.show_text(&text);
        }
    }

    /// Рисует прямоугольник с закруглёнными углами
    fn rounded_rect(cr: &gtk4::cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
        let degrees = std::f64::consts::PI / 180.0;
        cr.new_sub_path();
        let _ = cr.arc(x + w - r, y + r, r, -90.0 * degrees, 0.0 * degrees);
        let _ = cr.arc(x + w - r, y + h - r, r, 0.0 * degrees, 90.0 * degrees);
        let _ = cr.arc(x + r, y + h - r, r, 90.0 * degrees, 180.0 * degrees);
        let _ = cr.arc(x + r, y + r, r, 180.0 * degrees, 270.0 * degrees);
        let _ = cr.close_path();
    }

    /// Рисует прямоугольник с закруглёнными нижними углами
    fn rounded_rect_bottom(cr: &gtk4::cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
        let degrees = std::f64::consts::PI / 180.0;
        cr.new_sub_path();
        let top_r = r.min(h / 2.0);
        let _ = cr.move_to(x, y + top_r);
        let _ = cr.arc(x + top_r, y + top_r, top_r, 180.0 * degrees, 270.0 * degrees);
        let _ = cr.arc(x + w - top_r, y + top_r, top_r, -90.0 * degrees, 0.0 * degrees);
        let _ = cr.arc(x + w - r, y + h - r, r, 0.0 * degrees, 90.0 * degrees);
        let _ = cr.arc(x + r, y + h - r, r, 90.0 * degrees, 180.0 * degrees);
        let _ = cr.close_path();
    }

    /// Преобразует громкость (0.0-1.0) в RGB цвет
    /// Инвертировано: красный при низкой громкости, зелёный при высокой
    fn volume_to_color(volume: f64) -> (f64, f64, f64) {
        // 0-50%: красный -> жёлтый
        // 50-100%: жёлтый -> зелёный
        if volume <= 0.5 {
            let t = volume * 2.0; // 0 to 1
            // Красный (0.9, 0.3, 0.3) -> Жёлтый (0.9, 0.8, 0.2)
            (0.9, 0.3 + t * 0.5, 0.3 - t * 0.1)
        } else {
            let t = (volume - 0.5) * 2.0; // 0 to 1
            // Жёлтый (0.9, 0.8, 0.2) -> Зелёный (0.4, 0.8, 0.4)
            (0.9 - t * 0.5, 0.8, 0.2 + t * 0.2)
        }
    }

    /// Возвращает виджет DrawingArea
    pub fn widget(&self) -> &gtk4::DrawingArea {
        &self.drawing_area
    }

    /// Устанавливает значение громкости (0-100)
    pub fn set_value(&self, volume: u8) {
        self.current_volume.set(volume as f64);
        self.drawing_area.queue_draw();
    }

    /// Устанавливает значение громкости и состояние muted
    pub fn set_volume_state(&self, volume: u8, muted: bool) {
        self.current_volume.set(volume as f64);
        self.is_muted.set(muted);
        self.drawing_area.queue_draw();
    }

    /// Устанавливает состояние muted
    pub fn set_muted(&self, muted: bool) {
        self.is_muted.set(muted);
        self.drawing_area.queue_draw();
    }

    /// Получает текущее значение громкости
    pub fn get_value(&self) -> u8 {
        self.current_volume.get() as u8
    }

    /// Проверяет, выключен ли звук
    pub fn is_muted(&self) -> bool {
        self.is_muted.get()
    }
}
