//! Read the weekly schedules that nlclient saves, including inactive modes.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;

use crate::bridge::Bridge;

pub mod edit;

pub const RECOVERY_FILE: &str = "/media/data/schedulemanager.recovery";
const MAX_FILE: u64 = 1024 * 1024;
const MODES: [(&str, &str); 3] = [("HEAT", "heat"), ("COOL", "cool"), ("RANGE", "heat_cool")];
pub const TOPICS: [&str; 3] = [MODES[0].1, MODES[1].1, MODES[2].1];
const DAYS: [(&str, &str); 7] = [
    ("Mon", "monday"),
    ("Tue", "tuesday"),
    ("Wed", "wednesday"),
    ("Thu", "thursday"),
    ("Fri", "friday"),
    ("Sat", "saturday"),
    ("Sun", "sunday"),
];

pub type Schedules = BTreeMap<&'static str, Value>;

#[derive(Default)]
pub struct Dashboard {
    pub schedules: Option<Schedules>,
    pub result: Option<Value>,
    pub published: Option<String>,
}

pub async fn read(path: &Path) -> Option<String> {
    let file = tokio::fs::File::open(path).await.ok()?;
    let mut text = String::new();
    let size = file.take(MAX_FILE + 1).read_to_string(&mut text).await.ok()?;
    (size as u64 <= MAX_FILE).then_some(text)
}

pub fn active_mode(target_type: &str) -> Option<&'static str> {
    match target_type {
        "heat" => Some("heat"),
        "cool" => Some("cool"),
        "range" => Some("heat_cool"),
        _ => None,
    }
}

fn native_mode(mode: &str) -> Option<&'static str> {
    MODES.iter().find_map(|&(native, name)| (mode == name).then_some(native))
}

pub fn revision(schedule: &Value) -> String {
    format!("{:x}", Sha256::digest(schedule.to_string()))
}

pub async fn watch(bridge: Arc<Bridge>, path: &Path) {
    let mut last_file = String::new();
    let mut refresh = tokio::time::interval(Duration::from_secs(15));
    refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        refresh.tick().await;
        let Some(text) = read(path).await else { continue };
        if text == last_file {
            continue;
        }
        // Keep the last complete snapshot if nlclient is still writing the file.
        if let Some(schedules) = parse(&text) {
            bridge.set_schedules(schedules);
        }
        last_file = text;
    }
}

/// Each recovery field ends at a comma outside records and quoted strings.
fn fields(input: &str) -> Option<Vec<(&str, &str)>> {
    let mut fields = Vec::new();
    let (mut start, mut depth) = (0, 0_usize);
    let (mut quoted, mut escaped) = (false, false);
    for (index, byte) in input.bytes().chain(std::iter::once(b',')).enumerate() {
        if quoted {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                quoted = false;
            }
            continue;
        }
        match byte {
            b'"' => quoted = true,
            b'{' => depth += 1,
            b'}' => depth = depth.checked_sub(1)?,
            b',' if depth == 0 => {
                let field = input[start..index].trim();
                if !field.is_empty() {
                    let (key, value) = field.split_once(':')?;
                    fields.push((key.trim(), value.trim()));
                }
                start = index + 1;
            }
            _ => {}
        }
    }
    (!quoted && depth == 0).then_some(fields)
}

fn record(value: &str) -> Option<(&str, &str)> {
    let (name, body) = value.strip_prefix('"')?.split_once('"')?;
    Some((name, body.trim().strip_prefix('{')?.strip_suffix('}')?))
}

fn value<'a>(fields: &[(&str, &'a str)], key: &str) -> Option<&'a str> {
    fields.iter().find_map(|&(name, value)| (name == key).then_some(value))
}

pub fn parse(text: &str) -> Option<Schedules> {
    parse_schedules(&manager(text)?, None)
}

fn manager(text: &str) -> Option<Vec<(&str, &str)>> {
    let manager = text.trim().strip_prefix("ScheduleManager:")?.trim_start().strip_prefix("\"\"")?;
    let manager = manager.trim_start().strip_prefix('{')?;
    // The gen 3 file can end after its final schedule without closing the
    // manager record. Its declared count distinguishes that from a short write.
    let manager = fields(manager).or_else(|| fields(manager.strip_suffix('}')?))?;
    let total = value(&manager, "NumObjects")?.parse::<usize>().ok()?;
    if manager.iter().filter(|(_, field)| record(field).is_some()).count() != total {
        return None;
    }
    Some(manager)
}

fn parse_schedules(manager: &[(&str, &str)], touched_mode: Option<&str>) -> Option<Schedules> {
    let mut schedules = BTreeMap::new();
    for &(_, week) in manager.iter().filter(|(key, _)| *key == "WeeklySchedule") {
        let (name, week) = record(week)?;
        let Some((_, mode)) = MODES.into_iter().find(|(native, _)| *native == name) else { continue };
        let week = fields(week)?;
        if value(&week, "Mode")? != name {
            return None;
        }
        let mut days = BTreeMap::new();
        for (_, day) in week.into_iter().filter(|(key, _)| *key == "DaySchedule") {
            let (name, day) = record(day)?;
            let (index, (_, weekday)) = DAYS.into_iter().enumerate().find(|(_, (native, _))| *native == name)?;
            let setpoints = parse_day(day, index, mode == "heat_cool", touched_mode == Some(mode))?;
            if days.insert(weekday, setpoints).is_some() {
                return None;
            }
        }
        if days.len() != DAYS.len() || schedules.insert(mode, json!({ "mode": mode, "days": days })).is_some() {
            return None;
        }
    }
    (!schedules.is_empty()).then_some(schedules)
}

fn parse_day(day: &str, day_index: usize, range: bool, keep_touches: bool) -> Option<Vec<Value>> {
    let day = fields(day)?;
    let total = value(&day, "Total")?.parse::<usize>().ok()?;
    let mut setpoints = Vec::new();
    for (_, setpoint) in day.into_iter().filter(|(key, _)| *key == "SetPoint") {
        let (_, setpoint) = record(setpoint)?;
        let setpoint = fields(setpoint)?;
        let time = value(&setpoint, "Time")?.parse::<u32>().ok()?;
        if time >= 86_400 || value(&setpoint, "Day")?.parse::<usize>().ok()? != day_index {
            return None;
        }
        // The recovery file uses signed Q16.16 degrees Celsius.
        let temp = |key| value(&setpoint, key)?.parse::<i32>().ok().map(|raw| f64::from(raw) / 65_536.0);
        let temperature = temp("Temp")?;
        let clock = time_string(time);
        let continuation = time == 0 && value(&setpoint, "Default") == Some("1");
        let mut entry = if range {
            json!({ "time": clock, "heat_c": temperature, "cool_c": temp("TempMax")?, "continuation": continuation })
        } else {
            json!({ "time": clock, "temperature_c": temperature, "continuation": continuation })
        };
        if keep_touches {
            for (native, bucket) in
                [("TouchedAt", "touched_at"), ("TouchedBy", "touched_by"), ("TouchedTZO", "touched_tzo")]
            {
                if let Some(touch) = value(&setpoint, native).and_then(|raw| raw.parse::<i64>().ok()) {
                    entry[bucket] = json!(touch);
                }
            }
        }
        setpoints.push((time, entry));
    }
    setpoints.sort_by_key(|(time, _)| *time);
    (setpoints.len() == total).then(|| setpoints.into_iter().map(|(_, entry)| entry).collect())
}

fn time_string(time: u32) -> String {
    format!("{:02}:{:02}:{:02}", time / 3600, time / 60 % 60, time % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recovery() -> String {
        let mut weeks = Vec::new();
        for (mode, _) in MODES {
            let mut days = Vec::new();
            for (day, (name, _)) in DAYS.into_iter().enumerate() {
                let end = if name == "Sun" {
                    format!(
                        r#"
                        SetPoint:"" {{ Temp:1376256, TempMax:1769472, Day:{day}, Time:27000 }},
                        SetPoint:"" {{ Temp:1310720, TempMax:1703936, Day:{day}, Time:0, Default:1 }}
                    "#
                    )
                } else {
                    String::new()
                };
                let total = if end.is_empty() { 0 } else { 2 };
                days.push(format!(r#"DaySchedule:"{name}" {{ Total:{total}, {end} }}"#));
            }
            weeks.push(format!(
                r#"WeeklySchedule:"{mode}" {{ Mode:{mode}, {}, LearnedData:"" {{ Note:"{{, \" }}" }} }}"#,
                days.join(", ")
            ));
        }
        format!(
            r#"ScheduleManager:"" {{ NumObjects:4, CurrentScheduleID:HEAT, {}, WeeklyIntervalSchedule:"HOT_WATER" {{}}"#,
            weeks.join(", ")
        )
    }

    #[test]
    fn all_saved_modes_include_sorted_setpoints_and_named_days() {
        let data = parse(&recovery()).unwrap();
        assert_eq!(data.len(), 3);
        assert_eq!(parse(&(recovery() + "}")), Some(data.clone()));
        assert_eq!(
            data["heat"]["days"]["sunday"],
            json!([
                {"time":"00:00:00","temperature_c":20.0,"continuation":true},
                {"time":"07:30:00","temperature_c":21.0,"continuation":false},
            ])
        );
        assert_eq!(data["cool"]["days"]["monday"], json!([]));
        assert_eq!(data["cool"]["days"].as_object().unwrap().len(), 7);
        assert_eq!(
            data["heat_cool"]["days"]["sunday"][1],
            json!({"time":"07:30:00","heat_c":21.0,"cool_c":27.0,"continuation":false})
        );
    }

    #[test]
    fn incomplete_records_are_rejected() {
        let text = recovery();
        assert!(parse(&text[..text.len() - 1]).is_none());
        assert!(parse(text.split(", WeeklyIntervalSchedule").next().unwrap()).is_none());
        assert!(parse(&text.replace("Total:2", "Total:3")).is_none());
        assert!(parse(&text.replace("Time:27000", "Time:86400")).is_none());
        assert!(parse(&text.replace("Day:6", "Day:0")).is_none());
    }
}
