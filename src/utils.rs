use chrono::{DateTime, Local, TimeZone};
use date_component::date_component;
use std::error::Error;
use std::fmt::Write;

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
        format!("{}days ", date_interval.day)
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

pub fn unix_time_u64_to_local_datetime(unix_time: Option<u64>) -> Option<DateTime<Local>> {
    if let Some(unix_time) = unix_time {
        match chrono::Local.timestamp_opt(unix_time as i64, 0) {
            chrono::offset::LocalResult::Single(x) => Some(x),
            _ => None,
        }
    } else {
        None
    }
}

pub fn unix_time_i64_to_local_datetime(unix_time: Option<i64>) -> Option<DateTime<Local>> {
    if let Some(unix_time) = unix_time {
        match chrono::Local.timestamp_opt(unix_time, 0) {
            chrono::offset::LocalResult::Single(x) => Some(x),
            _ => None,
        }
    } else {
        None
    }
}

pub fn hexdump(data: &[u8]) -> String {
    const BYTES_PER_LINE: usize = 16;

    let mut output = String::new();

    for (line_index, chunk) in data.chunks(BYTES_PER_LINE).enumerate() {
        let offset = line_index * BYTES_PER_LINE;

        write!(&mut output, "{offset:08x}  ").unwrap();

        for index in 0..BYTES_PER_LINE {
            match chunk.get(index) {
                Some(byte) => write!(&mut output, "{byte:02x} ").unwrap(),
                None => output.push_str("   "),
            }

            if index == 7 {
                output.push(' ');
            }
        }

        output.push('|');

        for &byte in chunk {
            let character = if byte.is_ascii_graphic() || byte == b' ' {
                byte as char
            } else {
                '.'
            };

            output.push(character);
        }

        output.push_str("|\n");
    }

    output
}

pub fn print_image_data(url: &str) {
    // Manually load the image from the URL and print the first 128 chars
    let bytes = reqwest::blocking::get(url)
        .and_then(|resp| resp.bytes())
        .unwrap_or_else(|_| {
            log::error!(
                "try_print_image_data: Failed to fetch image data for URL {}",
                url
            );
            Vec::new().into()
        });

    // Trim the bytes to the first 128 characters for debugging
    let bytes = if bytes.len() > 128 {
        &bytes[..128]
    } else {
        &bytes
    };

    let hexdump = hexdump(bytes);

    log::info!(
        "try_print_image_data: Image preview for URL {}:\n{}",
        url,
        hexdump
    );
}
