use crate::todo_list::TodoList;
use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDateTime, ParseResult};
use std::{
    env,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter},
    path::PathBuf,
};

pub fn load_todo_list(file_path: &PathBuf) -> Result<TodoList> {
    if file_path.exists() {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);
        let todo_list = serde_json::from_reader(reader)?;
        Ok(todo_list)
    } else {
        Ok(TodoList::new())
    }
}

pub fn save_todo_list(file_path: &PathBuf, todo_list: &TodoList) -> Result<()> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)?;

    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, todo_list)?;
    Ok(())
}

pub fn expand_path(path: &String) -> Result<PathBuf> {
    if path.starts_with('~') {
        let home_dir = env::var("HOME").context("HOME environment variable not set")?;

        if path == "~" {
            Ok(PathBuf::from(home_dir))
        } else {
            let stripped_path = path.trim_start_matches('~').trim_start_matches('/');
            Ok(PathBuf::from(home_dir).join(stripped_path))
        }
    } else {
        Ok(PathBuf::from(path))
    }
}

fn parse_relative_time(time_str: &String) -> Option<DateTime<Local>> {
    let now = Local::now();

    match time_str.to_lowercase().as_str() {
        "today" => Some(
            now.date_naive()
                .and_hms_opt(23, 59, 59)?
                .and_local_timezone(Local)
                .unwrap(),
        ),
        "tomorrow" => Some(
            (now + chrono::Duration::days(1))
                .date_naive()
                .and_hms_opt(23, 59, 59)?
                .and_local_timezone(Local)
                .unwrap(),
        ),
        "nextweek" => Some(
            (now + chrono::Duration::weeks(1))
                .date_naive()
                .and_hms_opt(23, 59, 59)?
                .and_local_timezone(Local)
                .unwrap(),
        ),
        _ => {
            if time_str.starts_with('+') {
                parse_duration_offset(&time_str[1..], now)
            } else {
                None
            }
        }
    }
}

fn parse_duration_offset(
    duration_str: &str,
    base_time: DateTime<Local>,
) -> Option<DateTime<Local>> {
    let parts: Vec<&str> = duration_str.split_whitespace().collect();
    let mut duration = chrono::Duration::zero();

    for part in parts {
        if part.ends_with("d") || part.ends_with("days") {
            let days = part
                .trim_end_matches("d")
                .trim_end_matches("day")
                .trim_end_matches("days")
                .parse()
                .ok()?;
            println!("run here {}", days);
            duration = duration + chrono::Duration::days(days);
        } else if part.ends_with("h") || part.ends_with("hours") {
            let hours = part
                .trim_end_matches("h")
                .trim_end_matches("hour")
                .trim_end_matches("hours")
                .parse()
                .ok()?;
            duration = duration + chrono::Duration::hours(hours);
        } else if part.ends_with("m") || part.ends_with("min") || part.ends_with("minutes") {
            let minutes = part
                .trim_end_matches("m")
                .trim_end_matches("min")
                .trim_end_matches("minute")
                .trim_end_matches("minutes")
                .parse()
                .ok()?;
            duration = duration + chrono::Duration::minutes(minutes);
        }
    }

    Some(base_time + duration)
}

pub fn parse_deadline(deadline: Option<String>) -> Result<DateTime<Local>> {
    if let Some(deadline_str) = deadline {
        // 尝试解析完整日期时间格式: YYYY-MM-DD HH:MM
        if let ParseResult::Ok(datetime) =
            NaiveDateTime::parse_from_str(&deadline_str, "%Y-%m-%d %H:%M")
        {
            // return Ok(DateTime::from_naive_utc_and_offset(datetime, Local));
            return Ok(DateTime::from_naive_utc_and_offset(
                datetime,
                Local::now().offset().clone(),
            ));
        }

        // 尝试解析日期格式: YYYY-MM-DD (默认为当天23:59)
        if let ParseResult::Ok(datetime) = NaiveDateTime::parse_from_str(&deadline_str, "%Y-%m-%d")
        {
            // let datetime = date.and_hms_opt(23, 59, 0).context("Invalid time")?;
            return Ok(DateTime::from_naive_utc_and_offset(
                datetime,
                Local::now().offset().clone(),
            ));
        }

        // 尝试解析相对时间
        if let Some(relative_time) = parse_relative_time(&deadline_str) {
            return Ok(relative_time);
        }
    }

    Err(anyhow::anyhow!(
        "Invalid deadline format. Use: YYYY-MM-DD HH:MM, YYYY-MM-DD, or relative time like 'tomorrow' or '+2days'"
    ))
}
