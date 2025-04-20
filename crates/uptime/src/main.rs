use chrono::{DateTime, Duration, IsoWeek, NaiveDate, TimeDelta, Utc, Weekday};
use chrono::{Datelike, NaiveTime};
use egui::{Align2, Color32, FontId, Pos2, Rangef, Rect, Stroke, pos2, vec2};
use serde::Deserialize;
use std::collections::HashMap;

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use windows::get_logs;
#[cfg(not(target_os = "windows"))]
pub mod mac;
#[cfg(not(target_os = "windows"))]
pub use mac::get_logs;

fn main() {
    common::app::run("uptime", |cc| {
        let mut selected_week = Utc::now().iso_week();
        let events = get_logs();

        return Box::new(move |ctx| {
            let ui = ctx.ui;

            ui_week_header(&mut selected_week, ui);
            let font_size = 15.0;
            let av_height = ui.available_height() - font_size * 2.0;
            ui.horizontal(|ui| {
                let weekdays = vec![
                    Weekday::Mon,
                    Weekday::Tue,
                    Weekday::Wed,
                    Weekday::Thu,
                    Weekday::Fri,
                    Weekday::Sat,
                    Weekday::Sun,
                ];

                let item_width = ui.available_width() / weekdays.len() as f32;
                let mut index = item_width / 2.0;
                let hour_height = av_height / 15.0;
                let start_time = NaiveTime::from_hms(7, 0, 0);
                let start_pos = ui.next_widget_position();

                for weekday in weekdays.into_iter() {
                    let date = NaiveDate::from_isoywd(
                        selected_week.year(),
                        selected_week.week(),
                        weekday,
                    );

                    if let Some(events) = events.get(&date) {
                        for event in events.iter() {
                            let time: chrono::TimeDelta = event.time - start_time;
                            let height =
                                time.num_seconds() as f32 / 3600.0 * hour_height + font_size * 2.0;

                            let color = match event.event_type {
                                EventType::Come => Color32::GREEN,
                                EventType::Leave => Color32::RED,
                            };
                            let text = match event.event_type {
                                EventType::Come => "come",
                                EventType::Leave => "leave",
                            };

                            let range = match event.event_type {
                                EventType::Come => Rangef::new(index - item_width / 2.0, index),
                                EventType::Leave => Rangef::new(index, index + item_width / 2.0),
                            };

                            let rect = match event.event_type {
                                EventType::Come => Rect::from_center_size(
                                    pos2(index - item_width / 2.0 + item_width / 4.0, height),
                                    vec2(item_width / 2.0, 10.0),
                                ),
                                EventType::Leave => Rect::from_center_size(
                                    pos2(index + item_width / 4.0, height),
                                    vec2(item_width / 2.0, 10.0),
                                ),
                            };
                            ui.painter().hline(range, height, Stroke::new(1.0, color));

                            ui.horizontal(|ui| {
                                ui.interact(rect, ui.next_auto_id(), egui::Sense::hover())
                                    .on_hover_text(format!(
                                        "{} {}",
                                        text,
                                        event.time.format("%H:%M")
                                    ));
                            });
                        }

                        if let Some(first) = events.first() {
                            let first = first.time;
                            let now = Utc::now();
                            let last = if selected_week == now.iso_week()
                                && weekday == now.weekday()
                            {
                                now.time().overflowing_add_signed(TimeDelta::hours(2)).0
                            } else {
                                events.last().unwrap().time
                            };

                            let break_start =
                                get_next_event(events, NaiveTime::from_hms(11, 55, 0));
                            let break_end = if let Some(start) = break_start {
                                get_next_event(
                                    events,
                                    start.overflowing_add_signed(TimeDelta::minutes(25)).0,
                                )
                            } else {
                                None
                            };
                            let break_time = if let Some((start, end)) = break_start.zip(break_end)
                            {
                                Some((start, end, end - start))
                            } else {
                                None
                            };
                            let work_delta = if let Some((_, _, break_delta)) = break_time {
                                last - first - break_delta
                            } else {
                                last - first
                            };

                            ui.painter().text(
                                start_pos
                                    + vec2(index - item_width / 2.0, av_height - font_size * 2.0),
                                Align2::LEFT_TOP,
                                format!(
                                    "{} - {} {}",
                                    first.format("%H:%M"),
                                    last.format("%H:%M"),
                                    format_delta(work_delta)
                                ),
                                FontId::proportional(font_size),
                                Color32::WHITE,
                            );
                            if let Some((start, end, delta)) = break_time {
                                ui.painter().text(
                                    start_pos
                                        + vec2(
                                            index - item_width / 2.0,
                                            av_height - font_size * 1.0,
                                        ),
                                    Align2::LEFT_TOP,
                                    format!(
                                        "{} - {} {}",
                                        start.format("%H:%M"),
                                        end.format("%H:%M"),
                                        format_delta(delta)
                                    ),
                                    FontId::proportional(font_size),
                                    Color32::WHITE,
                                );
                            }
                        }
                    }

                    ui_day_header(ui, index, weekday, font_size, date, start_pos);

                    index += item_width;
                }
            });
        });
    });
}

#[derive(Debug, Clone, Copy)]
pub enum EventType {
    Come,
    Leave,
}

#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Time {
    pub systemTime: DateTime<Utc>,
}

#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct Provider {
    pub name: String,
}

#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct EventInfo {
    pub eventID: u32,
    pub timeCreated: Time,
    pub provider: Provider,
}

#[derive(Deserialize, Default, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct MyEvent {
    pub system: EventInfo,
}

pub struct Event {
    pub event_type: EventType,
    pub time: NaiveTime,
}
pub struct App {
    pub selected_week: IsoWeek,
    pub events: HashMap<NaiveDate, Vec<Event>>,
}

fn ui_week_header(selected_week: &mut IsoWeek, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        if ui.button("<").on_hover_text("previous week").clicked() {
            *selected_week = previous_week(*selected_week);
        }
        ui.label(format!(
            "{} KW {}",
            selected_week.week(),
            selected_week.year()
        ));
        if ui.button(">").on_hover_text("next week").clicked() {
            *selected_week = next_week(*selected_week);
        }
        if ui.button("⌂").on_hover_text("current week").clicked() {
            *selected_week = Utc::now().iso_week();
        }
    });
}

fn format_delta(delta: TimeDelta) -> String {
    let hours = delta.num_hours();
    let minutes = delta.num_minutes() % 60;
    if hours > 0 {
        if minutes > 0 {
            return format!("{}h {}m", hours, minutes);
        }
        return format!("{}h", hours);
    }
    return format!("{}m", minutes);
}

fn get_next_event(events: &Vec<Event>, time: NaiveTime) -> Option<NaiveTime> {
    for event in events.iter() {
        if event.time > time {
            return Some(event.time);
        }
    }
    return None;
}

fn ui_day_header(
    ui: &mut egui::Ui,
    index: f32,
    weekday: Weekday,
    font_size: f32,
    date: NaiveDate,
    start_pos: Pos2,
) {
    ui.painter().text(
        start_pos + vec2(index, 0.0),
        Align2::CENTER_TOP,
        format!("{}", weekday),
        FontId::proportional(font_size),
        Color32::WHITE,
    );

    ui.painter().text(
        start_pos + vec2(index, font_size),
        Align2::CENTER_TOP,
        date.format("%d.%m"),
        FontId::proportional(font_size),
        Color32::WHITE,
    );
}

fn previous_week(week: IsoWeek) -> IsoWeek {
    let mut date = NaiveDate::from_isoywd(week.year(), week.week(), Weekday::Mon);

    date -= Duration::weeks(1);
    return date.iso_week();
}

fn next_week(week: IsoWeek) -> IsoWeek {
    let mut date = NaiveDate::from_isoywd(week.year(), week.week(), Weekday::Mon);
    date += Duration::weeks(1);
    return date.iso_week();
}
