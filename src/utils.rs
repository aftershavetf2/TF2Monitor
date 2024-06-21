use chrono::{DateTime, Local};
use date_component::date_component;
use std::error::Error;

// From https://stevedonovan.github.io/rust-gentle-intro/6-error-handling.html
// Thanks Steve.
pub type BoxResult<T> = Result<T, Box<dyn Error>>;

pub fn duration_as_string(when: DateTime<Local>) -> String {
    let now = Local::now();
    let days = (now - when).num_days();

    // Special case for today
    if days == 0 {
        return "Today".to_string();
    }

    let date_interval = date_component::calculate(&when, &now);
    let years = if date_interval.year > 0 {
        format!("{}y ", date_interval.year)
    } else {
        "".to_string()
    };

    let months = if date_interval.year == 0 && date_interval.month > 0 {
        format!("{}mo ", date_interval.month)
    } else {
        "".to_string()
    };

    let days = if date_interval.year == 0 && date_interval.month == 0 && date_interval.day > 0 {
        format!("{}d ", date_interval.day)
    } else {
        "".to_string()
    };

    let hours = if date_interval.year == 0
        && date_interval.month == 0
        && date_interval.day == 0
        && date_interval.hour > 0
    {
        format!("{}h ", date_interval.hour)
    } else {
        "".to_string()
    };

    let minutes = if date_interval.year == 0
        && date_interval.month == 0
        && date_interval.day == 0
        && date_interval.minute > 0
    {
        format!("{}m ", date_interval.hour)
    } else {
        "".to_string()
    };

    format!("{}{}{}{}{}", years, months, days, hours, minutes)
}
