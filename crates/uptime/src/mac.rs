use crate::{Event, EventType};
use chrono::{NaiveDate, NaiveDateTime, TimeDelta};
use std::collections::HashMap;

pub fn get_logs() -> HashMap<NaiveDate, Vec<Event>> {
    let mut logs: HashMap<NaiveDate, Vec<Event>> = HashMap::new();

    // pmset -g log | egrep "\b(Sleep|Wake|DarkWake|Start)\s{2,}"

    if let Ok(output) = std::process::Command::new("pmset")
        .arg("-g")
        .arg("log")
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8(output.stdout);
            if let Ok(text) = text {
                for line in text.lines() {
                    let expected = "2024-10-22 13:29:19 +0200 ";
                    let expected_time = "2024-10-22 13:29:19";
                    let sleep = "Sleep";
                    let wake = "Wake";
                    let start = "Start";

                    if line.len() > expected.len() {
                        let naive_datetime = NaiveDateTime::parse_from_str(
                            &line[..expected_time.len()],
                            "%Y-%m-%d %H:%M:%S",
                        );
                        if let Ok(naive) = naive_datetime {
                            let mut event_type = None;
                            if line[expected.len()..].starts_with(sleep) {
                                if line.contains("Entering Sleep state due to 'Software Sleep"){
                                    event_type = Some(EventType::Leave);
                                }
                                if line.contains("Entering Sleep state due to 'Clamshell Sleep"){
                                    event_type = Some(EventType::Leave);
                                }
                            }
                            if line[expected.len()..].starts_with(start) {
                                event_type = Some(EventType::Come);
                            }
                            if line[expected.len()..].starts_with(wake) {
                                if line.contains("DarkWake to FullWake from Deep Idle [CDNVAP] : due to Notification Using AC (Charge:100%)"){
                                    event_type = Some(EventType::Come);
                                }
                                if line.contains("Wake from Deep Idle [CDNVA]"){
                                    event_type = Some(EventType::Come);
                                }
                            }
                            if let Some(event_type) = event_type {
                                let time = naive.time();
                                //let (time, _) = time.overflowing_add_signed(TimeDelta::hours(2));
                                if let Some(list) = logs.get_mut(&naive.date()) {
                                    list.push(Event { event_type, time });
                                } else {
                                    logs.insert(naive.date(), vec![Event { event_type, time }]);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    return logs;
}
