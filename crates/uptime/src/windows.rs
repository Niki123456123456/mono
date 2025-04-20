extern crate win_event_log;

use win_event_log::prelude::*;

use std::collections::HashMap;
use chrono::{NaiveDate, TimeDelta};
use crate::{Event, EventType};

pub fn get_logs() -> HashMap<NaiveDate, Vec<Event>> {
    let mut test: HashMap<NaiveDate, Vec<Event>> = HashMap::new();
        let event_types = HashMap::from([
            (12, EventType::Come),    // operating system started
            (13, EventType::Leave),   // operating system shutting down
            (41, EventType::Leave), // The system has rebooted without cleanly shutting down first. This error could be caused if the system stopped responding, crashed, or lost power unexpectedly.
            (1074, EventType::Leave), // Logged when an app (ex: Windows Update) causes the system to restart, or when a user initiates a restart or shutdown.
            (6005, EventType::Come),
            (6006, EventType::Leave), // Logged when an app (ex: Windows Update) causes the system to restart, or when a user initiates a restart or shutdown.
            (6008, EventType::Leave), // Logged when an app (ex: Windows Update) causes the system to restart, or when a user initiates a restart or shutdown.
            (506, EventType::Come),   // system is entering Modern Standby
            (507, EventType::Leave),  // system is exiting Modern Standby
            (42, EventType::Come),    // system is entering sleep
            (107, EventType::Leave),  //system has resumed sleep
            (105, EventType::Come),   //power source changed
        ]);

        let conditions = event_types
            .iter()
            .map(|t| Condition::filter(EventFilter::event(*t.0)))
            .collect();
        let query = QueryList::new()
            .with_query(
                Query::new()
                    .item(
                        QueryItem::selector("System".to_owned())
                            .system_conditions(Condition::or(conditions))
                            .build(),
                    )
                    .query(),
            )
            .build();

        match WinEvents::get(query) {
            Ok(events) => {
                for event in events.into_iter() {
                    //println!("{}", event);
                    let parsed: MyEvent = event.into();
                    if parsed.system.provider.name != "Microsoft-Windows-Kernel-Power"
                        && parsed.system.provider.name != "Microsoft-Windows-Kernel-General"
                    {
                        //continue;
                    }
                    if parsed.system.provider.name == "Microsoft-Windows-Wininit" {
                        continue;
                    }
                    //println!("{:?}", parsed);
                    if let Some(event_type) = event_types.get(&parsed.system.eventID) {
                        let naive = parsed.system.timeCreated.systemTime.naive_utc();
                        let time = naive.time();
                        let (time, _) = time.overflowing_add_signed(TimeDelta::hours(2));
                        if let Some(list) = test.get_mut(&naive.date()) {
                            list.push(Event {
                                event_type: *event_type,
                                time,
                            });
                        } else {
                            test.insert(
                                naive.date(),
                                vec![Event {
                                    event_type: *event_type,
                                    time,
                                }],
                            );
                        }
                    }
                }
            }
            Err(e) => println!("Error: {}", e),
        }

        return test;

}