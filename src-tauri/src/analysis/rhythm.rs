//! When the work happens: the weekday-by-hour grid and the day series behind it.
//!
//! Timestamps are stored as UTC. They are converted to local time here, because
//! "when do I work" is a question about the wall clock the user lives by, not
//! about UTC.

use anyhow::Result;
use chrono::{DateTime, Datelike, Local, Timelike};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayRow {
    /// Local calendar date, `YYYY-MM-DD`.
    pub date: String,
    pub turns: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rhythm {
    /// 7 × 24 turn counts, Monday first, in local time.
    pub grid: Vec<Vec<i64>>,
    pub busiest_hour: Option<u32>,
    pub busiest_weekday: Option<u32>,
    pub days: Vec<DayRow>,
    pub active_days: i64,
    /// Longest run of consecutive active days.
    pub longest_streak: i64,
    /// The streak that ends today or yesterday, if any.
    pub current_streak: i64,
}

pub fn load(conn: &Connection) -> Result<Rhythm> {
    let mut stmt = conn.prepare("SELECT ts_utc FROM turns WHERE ts_utc IS NOT NULL")?;
    let stamps = stmt.query_map([], |row| row.get::<_, String>(0))?;

    let mut grid = vec![vec![0_i64; 24]; 7];
    let mut per_day: BTreeMap<String, i64> = BTreeMap::new();

    for stamp in stamps {
        let Ok(parsed) = DateTime::parse_from_rfc3339(&stamp?) else {
            continue;
        };
        let local = parsed.with_timezone(&Local);
        let weekday = local.weekday().num_days_from_monday() as usize;
        grid[weekday][local.hour() as usize] += 1;
        *per_day
            .entry(local.format("%Y-%m-%d").to_string())
            .or_default() += 1;
    }

    let busiest = grid
        .iter()
        .enumerate()
        .flat_map(|(day, hours)| hours.iter().enumerate().map(move |(h, n)| (day, h, *n)))
        .max_by_key(|(_, _, n)| *n)
        .filter(|(_, _, n)| *n > 0);

    let days: Vec<DayRow> = per_day
        .iter()
        .map(|(date, turns)| DayRow {
            date: date.clone(),
            turns: *turns,
        })
        .collect();

    let (longest_streak, current_streak) = streaks(&days);

    Ok(Rhythm {
        busiest_weekday: busiest.map(|(day, _, _)| day as u32),
        busiest_hour: busiest.map(|(_, hour, _)| hour as u32),
        active_days: days.len() as i64,
        longest_streak,
        current_streak,
        grid,
        days,
    })
}

/// Longest and current run of consecutive days, from an ascending date list.
fn streaks(days: &[DayRow]) -> (i64, i64) {
    let parsed: Vec<chrono::NaiveDate> = days
        .iter()
        .filter_map(|day| chrono::NaiveDate::parse_from_str(&day.date, "%Y-%m-%d").ok())
        .collect();

    let mut longest = 0;
    let mut run = 0;
    let mut previous: Option<chrono::NaiveDate> = None;

    for date in &parsed {
        run = match previous {
            Some(before) if (*date - before).num_days() == 1 => run + 1,
            _ => 1,
        };
        longest = longest.max(run);
        previous = Some(*date);
    }

    // A streak counts as current while today is still open: work may yet
    // happen, so yesterday must not end it.
    let today = Local::now().date_naive();
    let current = match previous {
        Some(last) if (today - last).num_days() <= 1 => run,
        _ => 0,
    };

    (longest, current)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(date: &str) -> DayRow {
        DayRow {
            date: date.to_owned(),
            turns: 1,
        }
    }

    #[test]
    fn the_longest_streak_spans_consecutive_days_only() {
        let days = [
            day("2026-01-01"),
            day("2026-01-02"),
            day("2026-01-03"),
            day("2026-01-10"),
        ];
        let (longest, current) = streaks(&days);
        assert_eq!(longest, 3);
        assert_eq!(current, 0, "the last day is long past");
    }

    #[test]
    fn a_streak_ending_today_is_current() {
        let today = Local::now().date_naive();
        let days = [
            day(&(today - chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string()),
            day(&today.format("%Y-%m-%d").to_string()),
        ];
        let (longest, current) = streaks(&days);
        assert_eq!(longest, 2);
        assert_eq!(current, 2);
    }

    #[test]
    fn a_gap_of_one_day_keeps_yesterdays_streak_current() {
        let today = Local::now().date_naive();
        let days = [day(&(today - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string())];
        let (_, current) = streaks(&days);
        assert_eq!(current, 1, "today is not over yet");
    }
}
