use gtk4::prelude::*;
use chrono::{Datelike, Local, NaiveDate, TimeZone};
use std::rc::Rc;
use std::cell::Cell;

pub struct CalendarWidget;

impl CalendarWidget {
    /// Создаёт и показывает календарь в виде Popover
    pub fn show(button: &gtk4::Button) {
        let popover = gtk4::Popover::new();
        popover.set_parent(button);
        popover.set_position(gtk4::PositionType::Bottom);

        let now = Local::now();
        let current_year = Rc::new(Cell::new(now.year()));
        let current_month = Rc::new(Cell::new(now.month()));

        let calendar_box = Self::create_content(
            current_year,
            current_month,
            popover.downgrade(),
        );

        popover.set_child(Some(&calendar_box));
        popover.popup();
    }

    fn create_content(
        current_year: Rc<Cell<i32>>,
        current_month: Rc<Cell<u32>>,
        popover: gtk4::glib::WeakRef<gtk4::Popover>,
    ) -> gtk4::Box {
        let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 10);
        main_box.add_css_class("calendar-container");
        main_box.set_margin_top(10);
        main_box.set_margin_bottom(10);
        main_box.set_margin_start(10);
        main_box.set_margin_end(10);

        // Заголовок с месяцем и кнопками навигации
        let header = Self::create_header();
        let month_label = header
            .first_child()
            .unwrap()
            .next_sibling()
            .unwrap()
            .downcast::<gtk4::Label>()
            .unwrap();

        // Сетка календаря
        let grid = Self::create_grid();

        main_box.append(&header);
        main_box.append(&grid);

        // Функция обновления календаря
        let update_calendar = {
            let month_label = month_label.clone();
            let grid = grid.clone();
            let current_year = current_year.clone();
            let current_month = current_month.clone();
            let popover = popover.clone();

            move || {
                Self::update_calendar_grid(
                    &grid,
                    &month_label,
                    current_year.get(),
                    current_month.get(),
                    &popover,
                );
            }
        };

        // Настройка кнопок навигации
        Self::setup_navigation(&header, current_year, current_month, update_calendar.clone());

        // Первоначальная отрисовка
        update_calendar();

        main_box
    }

    fn create_header() -> gtk4::Box {
        let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 5);
        header.add_css_class("calendar-header");

        let prev_button = gtk4::Button::with_label("◀");
        prev_button.add_css_class("calendar-nav");

        let month_label = gtk4::Label::new(Some(""));
        month_label.add_css_class("calendar-month");
        month_label.set_hexpand(true);

        let next_button = gtk4::Button::with_label("▶");
        next_button.add_css_class("calendar-nav");

        header.append(&prev_button);
        header.append(&month_label);
        header.append(&next_button);

        header
    }

    fn create_grid() -> gtk4::Grid {
        let grid = gtk4::Grid::new();
        grid.add_css_class("calendar-grid");
        grid.set_column_homogeneous(true);
        grid.set_row_homogeneous(true);
        grid.set_column_spacing(5);
        grid.set_row_spacing(5);

        // Дни недели
        let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
        for (i, day) in weekdays.iter().enumerate() {
            let label = gtk4::Label::new(Some(day));
            label.add_css_class("calendar-weekday");
            grid.attach(&label, i as i32, 0, 1, 1);
        }

        grid
    }

    fn setup_navigation<F>(
        header: &gtk4::Box,
        current_year: Rc<Cell<i32>>,
        current_month: Rc<Cell<u32>>,
        update_calendar: F,
    ) where
        F: Fn() + 'static + Clone,
    {
        let prev_button = header.first_child().unwrap().downcast::<gtk4::Button>().unwrap();
        let next_button = header.last_child().unwrap().downcast::<gtk4::Button>().unwrap();

        // Кнопка "назад"
        {
            let update = update_calendar.clone();
            let current_year = current_year.clone();
            let current_month = current_month.clone();
            prev_button.connect_clicked(move |_| {
                let year = current_year.get();
                let month = current_month.get();

                if month == 1 {
                    current_year.set(year - 1);
                    current_month.set(12);
                } else {
                    current_month.set(month - 1);
                }
                update();
            });
        }

        // Кнопка "вперёд"
        {
            let update = update_calendar;
            next_button.connect_clicked(move |_| {
                let year = current_year.get();
                let month = current_month.get();

                if month == 12 {
                    current_year.set(year + 1);
                    current_month.set(1);
                } else {
                    current_month.set(month + 1);
                }
                update();
            });
        }
    }

    fn update_calendar_grid(
        grid: &gtk4::Grid,
        month_label: &gtk4::Label,
        year: i32,
        month: u32,
        popover: &gtk4::glib::WeakRef<gtk4::Popover>,
    ) {
        // Обновляем заголовок
        let date = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let month_year = Local
            .from_local_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
            .unwrap()
            .format("%B %Y")
            .to_string();
        month_label.set_text(&month_year);

        // Очищаем старые дни
        let mut child = grid.first_child();
        let mut to_remove = Vec::new();
        while let Some(widget) = child {
            let next = widget.next_sibling();
            if !widget.has_css_class("calendar-weekday") {
                to_remove.push(widget.clone());
            }
            child = next;
        }
        for widget in to_remove {
            grid.remove(&widget);
        }

        // Первый день месяца
        let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
        let weekday = first_day.weekday().num_days_from_monday() as i32;

        // Количество дней в месяце
        let days_in_month = if month == 12 {
            NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
        } else {
            NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
        }
        .signed_duration_since(first_day)
        .num_days() as i32;

        // Текущая дата
        let today = Local::now().naive_local().date();

        // Заполняем дни
        let mut day = 1;
        let mut row = 1;
        let mut col = weekday;

        while day <= days_in_month {
            let day_button = gtk4::Button::with_label(&day.to_string());
            day_button.add_css_class("calendar-day");

            let current_date = NaiveDate::from_ymd_opt(year, month, day as u32).unwrap();
            if current_date == today {
                day_button.add_css_class("calendar-today");
            }

            // Закрываем popover при клике на день
            let popover_weak = popover.clone();
            day_button.connect_clicked(move |_| {
                if let Some(p) = popover_weak.upgrade() {
                    p.popdown();
                }
            });

            grid.attach(&day_button, col, row, 1, 1);

            col += 1;
            if col > 6 {
                col = 0;
                row += 1;
            }
            day += 1;
        }
    }
}

