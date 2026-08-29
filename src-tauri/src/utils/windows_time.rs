//! Windows-local time helpers.

use std::time::{SystemTime, UNIX_EPOCH};

pub fn current_utc_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    format_utc_timestamp(seconds)
}

pub fn current_local_day() -> String {
    #[cfg(windows)]
    {
        let mut system_time = NativeSystemTime {
            year: 0,
            month: 0,
            day_of_week: 0,
            day: 0,
            hour: 0,
            minute: 0,
            second: 0,
            milliseconds: 0,
        };
        unsafe {
            get_local_time(&mut system_time);
        }
        if system_time.year != 0 && system_time.month != 0 && system_time.day != 0 {
            return format!(
                "{:04}-{:02}-{:02}",
                system_time.year, system_time.month, system_time.day
            );
        }
    }

    current_utc_timestamp()[..10].to_owned()
}

pub fn parse_timestamp_seconds(timestamp: &str) -> Option<i64> {
    let date_time = timestamp.get(..19)?;
    let year = date_time.get(0..4)?.parse::<i64>().ok()?;
    let month = date_time.get(5..7)?.parse::<i64>().ok()?;
    let day = date_time.get(8..10)?.parse::<i64>().ok()?;
    let hour = date_time.get(11..13)?.parse::<i64>().ok()?;
    let minute = date_time.get(14..16)?.parse::<i64>().ok()?;
    let second = date_time.get(17..19)?.parse::<i64>().ok()?;
    if date_time.as_bytes().get(4) != Some(&b'-')
        || date_time.as_bytes().get(7) != Some(&b'-')
        || date_time.as_bytes().get(10) != Some(&b'T')
        || date_time.as_bytes().get(13) != Some(&b':')
        || date_time.as_bytes().get(16) != Some(&b':')
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
        || !(0..=59).contains(&second)
    {
        return None;
    }

    let days = days_from_civil(year, month, day)?;
    let mut offset_seconds = 0_i64;
    let suffix = timestamp.get(19..)?;
    let timezone = suffix
        .find(|character: char| character == 'Z' || character == '+' || character == '-')
        .map(|index| &suffix[index..])?;
    if timezone != "Z" {
        let sign = match timezone.as_bytes().first()? {
            b'+' => 1_i64,
            b'-' => -1_i64,
            _ => return None,
        };
        let hours = timezone.get(1..3)?.parse::<i64>().ok()?;
        let minutes = timezone.get(4..6)?.parse::<i64>().ok()?;
        if timezone.as_bytes().get(3) != Some(&b':') || hours > 23 || minutes > 59 {
            return None;
        }
        offset_seconds = sign * (hours * 3_600 + minutes * 60);
    }

    days.checked_mul(86_400)?
        .checked_add(hour * 3_600 + minute * 60 + second)?
        .checked_sub(offset_seconds)
}

pub fn timestamp_local_day(timestamp: &str) -> Option<&str> {
    let day = timestamp.get(..10)?;
    if day.as_bytes().get(4) != Some(&b'-')
        || day.as_bytes().get(7) != Some(&b'-')
        || day.get(0..4)?.parse::<u32>().is_err()
        || day.get(5..7)?.parse::<u32>().is_err()
        || day.get(8..10)?.parse::<u32>().is_err()
    {
        return None;
    }
    Some(day)
}

fn format_utc_timestamp(seconds_since_epoch: u64) -> String {
    let seconds = i64::try_from(seconds_since_epoch).unwrap_or(i64::MAX);
    let days = seconds.div_euclid(86_400);
    let seconds_in_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = seconds_in_day / 3_600;
    let minute = seconds_in_day % 3_600 / 60;
    let second = seconds_in_day % 60;
    format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z",
        month = month,
        day = day
    )
}

fn civil_from_days(days_since_epoch: i64) -> (i64, u32, u32) {
    let shifted_days = days_since_epoch + 719_468;
    let era = if shifted_days >= 0 {
        shifted_days / 146_097
    } else {
        (shifted_days - 146_096) / 146_097
    };
    let day_of_era = shifted_days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    (year, month as u32, day as u32)
}

#[cfg(windows)]
#[repr(C)]
struct NativeSystemTime {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    milliseconds: u16,
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetLocalTime(system_time: *mut NativeSystemTime);
}

#[cfg(windows)]
unsafe fn get_local_time(system_time: &mut NativeSystemTime) {
    GetLocalTime(system_time);
}

fn days_from_civil(year: i64, month: i64, day: i64) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let adjusted_year = year - i64::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year / 400
    } else {
        (adjusted_year - 399) / 400
    };
    let year_of_era = adjusted_year - era * 400;
    let month_prime = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * month_prime + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146_097 + day_of_era - 719_468)
}
