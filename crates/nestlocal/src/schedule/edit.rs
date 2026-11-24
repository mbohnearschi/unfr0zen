//! Convert a validated day edit to a native schedule bucket and confirm it
//! against the recovery file written by nlclient.
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde::Deserialize;
use serde_json::{json, Value};

use super::{manager, native_mode, parse_schedules, read, revision, time_string, value, DAYS, RECOVERY_FILE};
use crate::{bridge::Bridge, commands::SETPOINT, fields, json::Fields, settings_ui};

const READBACK_TIMEOUT: Duration = Duration::from_secs(30);
const STALE: &str = "The schedule changed on the Nest. Reopen the day and try again.";
const WAITING: &str = "The Nest has not reported a matching active schedule yet.";

#[derive(Deserialize)]
pub struct Edit {
    pub request_id: String,
    expires_at: i64,
    mode: String,
    day: String,
    revision: String,
    setpoints: Vec<Setpoint>,
}

#[derive(Deserialize)]
struct Setpoint {
    time: String,
    temperature_c: Option<f64>,
    heat_c: Option<f64>,
    cool_c: Option<f64>,
}

#[derive(Clone, Copy)]
struct Temperature {
    low: f64,
    high: Option<f64>,
}

impl Temperature {
    fn read(point: &Value, low: &str, high: Option<&str>) -> Option<Self> {
        let low = point[low].as_f64()?;
        let high = match high {
            Some(key) => Some(point[key].as_f64()?),
            None => None,
        };
        Some(Self { low, high })
    }

    fn saved(point: &Value, range: bool) -> Option<Self> {
        let low = if range { "heat_c" } else { "temperature_c" };
        Self::read(point, low, range.then_some("cool_c"))
    }

    fn matches(self, other: Self) -> bool {
        same_temperature(self.low, other.low)
            && match (self.high, other.high) {
                (Some(a), Some(b)) => same_temperature(a, b),
                (None, None) => true,
                _ => false,
            }
    }

    fn validate(self) -> Result<(), &'static str> {
        let allowed = SETPOINT.0..=SETPOINT.1;
        if !allowed.contains(&self.low) || self.high.is_some_and(|high| !allowed.contains(&high)) {
            return Err("Setpoint temperatures must be between 4.5 and 35 °C.");
        }
        if self.high.is_some_and(|high| high - self.low < 1.5) {
            return Err("The cooling setpoint must be at least 1.5 °C above heating.");
        }
        Ok(())
    }
}

impl Setpoint {
    fn temperature(&self, range: bool) -> Result<Temperature, &'static str> {
        match (range, self.temperature_c, self.heat_c, self.cool_c) {
            (false, Some(low), None, None) => Ok(Temperature { low, high: None }),
            (true, None, Some(low), Some(high)) => Ok(Temperature { low, high: Some(high) }),
            _ => Err("Enter the temperatures required for this schedule."),
        }
    }
}

#[derive(Clone)]
struct Entry {
    time: u32,
    temperature: Temperature,
    continuation: bool,
    raw: Value,
}

type Week = [BTreeMap<u32, Entry>; 7];

fn seconds(time: &str) -> Option<u32> {
    if time.len() != 8 {
        return None;
    }
    let (hours, rest) = time.split_once(':')?;
    let (minutes, seconds) = rest.split_once(':')?;
    let (hours, minutes, seconds): (u32, u32, u32) =
        (hours.parse().ok()?, minutes.parse().ok()?, seconds.parse().ok()?);
    let seconds = (hours < 24 && minutes < 60 && seconds < 60).then_some(hours * 3600 + minutes * 60 + seconds)?;
    (time_string(seconds) == time).then_some(seconds)
}

fn raw_week(bucket: &Fields, mode: &str) -> Option<Week> {
    let name = native_mode(mode)?;
    let range = name == "RANGE";
    let low = if range { "temp-min" } else { "temp" };
    if bucket.get("ver")?.as_i64()? != 2 || bucket.get("schedule_mode")?.as_str()? != name {
        return None;
    }
    let raw_days = bucket.get("days")?.as_object()?;
    if raw_days.len() != DAYS.len() {
        return None;
    }
    let mut week: Week = std::array::from_fn(|_| BTreeMap::new());
    for (index, day) in week.iter_mut().enumerate() {
        for raw in raw_days.get(&index.to_string())?.as_object()?.values() {
            let time = u32::try_from(raw["time"].as_u64()?).ok()?;
            let temperature = Temperature::read(raw, low, range.then_some("temp-max"))?;
            let continuation = match raw["entry_type"].as_str()? {
                "continuation" if time == 0 => true,
                "setpoint" => false,
                _ => return None,
            };
            if time >= 86_400
                || raw["type"] != name
                || day.insert(time, Entry { time, temperature, continuation, raw: raw.clone() }).is_some()
            {
                return None;
            }
        }
    }
    Some(week)
}

fn same_temperature(a: f64, b: f64) -> bool {
    // The bucket rounds the recovery file's Q16.16 value to five decimals.
    (a - b).abs() < 0.00002
}

fn matches_saved(week: &Week, saved: &Value) -> bool {
    DAYS.iter().zip(week).all(|((_, name), day)| {
        let Some(entries) = saved["days"][*name].as_array() else { return false };
        entries.len() == day.len()
            && entries.iter().zip(day.values()).all(|(saved, entry)| {
                saved["time"].as_str().and_then(seconds) == Some(entry.time)
                    && saved["continuation"].as_bool() == Some(entry.continuation)
                    && Temperature::saved(saved, entry.temperature.high.is_some())
                        .is_some_and(|temp| temp.matches(entry.temperature))
            })
    })
}

pub fn editable(bucket: &Fields, mode: &str, active: &str) -> bool {
    native_mode(mode).is_some() && raw_week(bucket, active).is_some()
}

fn new_entry(name: &str, time: u32, temperature: Temperature, continuation: bool) -> Entry {
    let now = chrono::Local::now();
    let mut raw = json!({
        "time": time, "type": name,
        "entry_type": if continuation { "continuation" } else { "setpoint" },
        "touched_at": now.timestamp(), "touched_tzo": now.offset().local_minus_utc(), "touched_by": 1
    });
    if let Some(high) = temperature.high {
        raw["temp-min"] = json!(temperature.low);
        raw["temp-max"] = json!(high);
    } else {
        raw["temp"] = json!(temperature.low);
    }
    Entry { time, temperature, continuation, raw }
}

/// A native save can arrive without a corresponding bucket report. Use the
/// saved week as the edit baseline and reuse matching bucket metadata or
/// the inactive schedule's saved touch fields.
fn saved_week(raw: &Week, saved: &Value, name: &str) -> Option<Week> {
    let mut week: Week = std::array::from_fn(|_| BTreeMap::new());
    for (index, (_, day)) in DAYS.iter().enumerate() {
        for point in saved["days"][*day].as_array()? {
            let time = seconds(point["time"].as_str()?)?;
            let temp = Temperature::saved(point, name == "RANGE")?;
            let continuation = point["continuation"].as_bool()?;
            let entry = raw[index]
                .get(&time)
                .filter(|entry| entry.continuation == continuation && entry.temperature.matches(temp))
                .cloned()
                .unwrap_or_else(|| {
                    let mut entry = new_entry(name, time, temp, continuation);
                    for key in ["touched_at", "touched_by", "touched_tzo"] {
                        if let Some(touch) = point.get(key) {
                            entry.raw[key] = touch.clone();
                        }
                    }
                    entry
                });
            if week[index].insert(time, entry).is_some() {
                return None;
            }
        }
    }
    Some(week)
}

fn replace_day(week: &Week, edit: &Edit, name: &str) -> Result<Week, &'static str> {
    let index = DAYS.iter().position(|(_, day)| *day == edit.day).ok_or("Choose a valid weekday.")?;
    let mut result = week.clone();
    let day = &mut result[index];
    day.retain(|_, entry| entry.continuation);
    for point in &edit.setpoints {
        let time = seconds(&point.time).ok_or("Enter a valid local time for each setpoint.")?;
        let temperature = point.temperature(name == "RANGE")?;
        let previous = week[index].get(&time).filter(|entry| !entry.continuation);
        let entry = if let Some(previous) = previous.filter(|entry| entry.temperature.matches(temperature)) {
            previous.clone()
        } else {
            temperature.validate()?;
            new_entry(name, time, temperature, false)
        };
        if day.insert(time, entry).is_some_and(|entry| !entry.continuation) {
            return Err("Each setpoint needs a different time.");
        }
    }

    // With no scheduled changes, retain the last effective target all week.
    // Otherwise start with Sunday's last change, walking backwards across days.
    let mut temperature = result
        .iter()
        .rev()
        .flat_map(|day| day.values().rev())
        .find(|entry| !entry.continuation)
        .map(|entry| entry.temperature)
        .or_else(|| week.iter().find_map(|day| day.get(&0).map(|entry| entry.temperature)))
        .ok_or("Add at least one setpoint to an empty schedule.")?;
    for day in &mut result {
        match day.get(&0) {
            Some(entry) if !entry.continuation || entry.temperature.matches(temperature) => {}
            _ => {
                day.insert(0, new_entry(name, 0, temperature, true));
            }
        }
        temperature = day.values().next_back().unwrap().temperature;
    }
    Ok(result)
}

fn encoded_days(week: &Week) -> Value {
    Value::Object(
        week.iter()
            .enumerate()
            .map(|(i, day)| {
                let entries = day.values().enumerate().map(|(i, entry)| (i.to_string(), entry.raw.clone())).collect();
                (i.to_string(), Value::Object(entries))
            })
            .collect(),
    )
}

pub async fn save(bridge: Arc<Bridge>, request: Edit) {
    if request.request_id.is_empty()
        || request.request_id.len() > 64
        || !request.request_id.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return;
    }
    let result = match bridge.schedule_edit.try_lock() {
        Ok(_guard) => apply(&bridge, &request, Path::new(RECOVERY_FILE)).await,
        Err(_) => Err("A schedule change is already in progress."),
    };
    bridge.set_schedule_result(json!({
        "request_id": request.request_id,
        "status": if result.is_ok() { "saved" } else { "error" },
        "message": result.err(),
    }));
}

async fn apply(bridge: &Bridge, edit: &Edit, path: &Path) -> Result<(), &'static str> {
    let text = read(path).await.ok_or(WAITING)?;
    let recovery = manager(&text).ok_or(WAITING)?;
    let before = parse_schedules(&recovery, None).ok_or(WAITING)?;
    let mode = edit.mode.as_str();
    let name = native_mode(mode).ok_or("Choose Heat, Cool or Heat • Cool.")?;
    let saved = before.get(mode).ok_or(WAITING)?;
    if revision(saved) != edit.revision {
        bridge.set_schedules(before);
        return Err(STALE);
    }
    if !(1..=60).contains(&edit.expires_at.saturating_sub(chrono::Utc::now().timestamp())) {
        return Err("The edit has expired or the thermostat clock is out of sync.");
    }
    let active = bridge.active_schedule().ok_or(WAITING)?;
    let current = bridge.store.get(&format!("schedule.{}", bridge.serial)).ok_or(WAITING)?;
    let active_name = native_mode(active).ok_or(WAITING)?;
    let raw = raw_week(&current.value, active).ok_or(WAITING)?;
    let inactive = mode != active;
    let week = if inactive {
        if value(&recovery, "CurrentScheduleID") != Some(active_name) {
            return Err(WAITING);
        }
        let saved_with_touches = parse_schedules(&recovery, Some(mode)).ok_or(WAITING)?;
        saved_week(&std::array::from_fn(|_| BTreeMap::new()), &saved_with_touches[mode], name)
    } else {
        saved_week(&raw, saved, name)
    }
    .ok_or(WAITING)?;
    let expected = replace_day(&week, edit, name)?;
    if matches_saved(&expected, saved) {
        return Ok(());
    }
    if bridge.active_schedule() != Some(active) {
        return Err(STALE);
    }
    let _ = settings_ui::request_wake();
    let mut changes = fields! { "days" => encoded_days(&expected) };
    if inactive {
        changes.extend(fields! { "schedule_mode" => name, "name" => name });
    }
    let proposal = bridge.store.offer(&current, changes)?;

    tokio::time::timeout(READBACK_TIMEOUT, async {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if let Some(text) = read(path).await {
                let Some(recovery) = manager(&text) else { continue };
                let Some(schedules) = parse_schedules(&recovery, None) else { continue };
                if inactive
                    && (value(&recovery, "CurrentScheduleID") != Some(active_name)
                        || bridge.active_schedule() != Some(active)
                        || before.iter().any(|(other, saved)| *other != mode && schedules.get(other) != Some(saved)))
                {
                    bridge.set_schedules(schedules);
                    return Err("The active mode or another schedule changed during the edit. Check the Nest.");
                }
                let matched = schedules.get(mode).is_some_and(|saved| matches_saved(&expected, saved));
                if matched {
                    bridge.set_schedules(schedules);
                    // An inactive proposal must never replace the active bucket.
                    if !inactive {
                        bridge.store.confirm_offer(&current, proposal);
                    }
                    return Ok(());
                }
            }
        }
    })
    .await
    .map_err(|_| "The Nest did not confirm the edit. Check its saved schedule before retrying.")?
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::tests::test_bridge;
    use crate::schedule::parse;

    fn single(low: f64) -> Temperature {
        Temperature { low, high: None }
    }

    fn recovery(monday_morning: u32, monday_temp: i32) -> String {
        format!(r#"ScheduleManager:"" {{ NumObjects:1, {}"#, recovery_week("HEAT", monday_morning, monday_temp))
    }

    fn recovery_week(mode: &str, monday_morning: u32, monday_temp: i32) -> String {
        let mut days = Vec::new();
        for (day, (label, _)) in DAYS.into_iter().enumerate() {
            let extra = if day == 0 {
                format!(
                    r#", SetPoint:"" {{ Temp:{monday_temp}, Day:0, Time:{monday_morning} }},
                    SetPoint:"" {{ Temp:1310720, Day:0, Time:79200, TouchedAt:1234, TouchedBy:1, TouchedTZO:-3600 }}"#
                )
            } else {
                String::new()
            };
            let total = if day == 0 { 3 } else { 1 };
            days.push(format!(
                r#"DaySchedule:"{label}" {{ Total:{total}, SetPoint:"" {{ Temp:1310720, Day:{day}, Time:0, Default:1 }}{extra} }}"#
            ));
        }
        format!(r#"WeeklySchedule:"{mode}" {{ Mode:{mode}, {} }}"#, days.join(","))
    }

    fn bucket() -> Fields {
        let mut days = Fields::new();
        for day in 0..7 {
            let row = new_entry("HEAT", 0, single(20.0), true).raw;
            let mut points = fields! {"0" => row};
            if day == 0 {
                points.insert("1".into(), new_entry("HEAT", 21600, single(21.79736), false).raw);
                points.insert("2".into(), new_entry("HEAT", 79200, single(20.0), false).raw);
            }
            days.insert(day.to_string(), Value::Object(points));
        }
        fields! {"ver" => 2, "schedule_mode" => "HEAT", "days" => days}
    }

    fn edit(revision: String, day: &str, rows: &[(&str, f64)]) -> Edit {
        Edit {
            request_id: "test".into(),
            expires_at: chrono::Utc::now().timestamp() + 30,
            mode: "heat".into(),
            day: day.into(),
            revision,
            setpoints: rows
                .iter()
                .map(|(time, temp)| Setpoint {
                    time: (*time).into(),
                    temperature_c: Some(*temp),
                    heat_c: None,
                    cool_c: None,
                })
                .collect(),
        }
    }

    #[test]
    fn edits_preserve_native_metadata_and_propagate_midnight_targets() {
        let week = raw_week(&bucket(), "heat").unwrap();
        let saved = parse(&recovery(21600, 1428512)).unwrap();
        assert!(matches_saved(&week, &saved["heat"]));
        let change = edit("".into(), "monday", &[("06:00:00", 21.79736328125), ("22:00:00", 18.0)]);
        let updated = replace_day(&week, &change, "HEAT").unwrap();
        assert_eq!(updated[0][&21600].raw, week[0][&21600].raw);
        for day in updated.iter().skip(1) {
            assert_eq!(day[&0].temperature.low, 18.0);
            assert!(day[&0].continuation);
        }
        assert_eq!(updated[0][&0].temperature.low, 18.0);
        for rows in
            [&[("08:00:00", 81.0)][..], &[("24:00:00", 20.0)][..], &[("08:00:00", 20.0), ("08:00:00", 21.0)][..]]
        {
            assert!(replace_day(&week, &edit("".into(), "monday", rows), "HEAT").is_err());
        }
    }

    #[test]
    fn the_next_edit_can_use_a_saved_week_with_an_older_mqtt_bucket() {
        let bucket = bucket();
        let copied = raw_week(&bucket, "heat").unwrap();
        let saved = parse(&recovery(25200, 1376256)).unwrap();
        assert!(!matches_saved(&copied, &saved["heat"]));
        assert!(editable(&bucket, "heat", "heat"));
        assert!(editable(&bucket, "cool", "heat"));
        assert!(editable(&bucket, "heat_cool", "heat"));
        assert!(!editable(&bucket, "cool", "heat_cool"));
        let current = saved_week(&copied, &saved["heat"], "HEAT").unwrap();
        let other_day = edit("".into(), "tuesday", &[("12:00:00", 20.0)]);
        let updated = replace_day(&current, &other_day, "HEAT").unwrap();
        assert!(!updated[0].contains_key(&21600));
        assert_eq!(updated[0][&25200].temperature.low, 21.0);
        assert_eq!(updated[1][&43200].temperature.low, 20.0);
        assert_eq!(updated[0][&79200].raw, copied[0][&79200].raw);
    }

    #[tokio::test]
    async fn a_day_edit_is_offered_once_and_only_confirmed_by_the_saved_week() {
        let bridge = test_bridge();
        let file = std::env::temp_dir().join(format!("nestlocal-schedule-test-{}.recovery", std::process::id()));
        let initial = recovery(21600, 1428512);
        tokio::fs::write(&file, &initial).await.unwrap();
        bridge.store.merge("shared.S", &fields! {"target_temperature_type" => "heat"}, 0, 0, true);
        let (before, _) = bridge.store.merge("schedule.S", &bucket(), 0, 0, true);
        let (mut offers, _subscription) = bridge.store.subscribe();
        let rows = [("07:00:00", 21.0), ("22:00:00", 20.0)];
        assert_eq!(apply(&bridge, &edit("stale".into(), "monday", &rows), &file).await, Err(STALE));
        assert!(offers.try_recv().is_err());

        let hash = revision(&parse(&initial).unwrap()["heat"]);
        let mut expired = edit(hash.clone(), "monday", &rows);
        expired.expires_at = 0;
        assert!(apply(&bridge, &expired, &file).await.unwrap_err().contains("expired"));
        assert!(offers.try_recv().is_err());
        let mut initial = before;
        let mut hash = hash;
        for (clock, time, temperature) in [("07:00:00", 25200, 21), ("08:00:00", 28800, 22)] {
            let request = edit(hash, "monday", &[(clock, f64::from(temperature)), ("22:00:00", 20.0)]);
            let applied = apply(&bridge, &request, &file);
            let saved = recovery(time, temperature * 65_536);
            let report = async {
                let offered = tokio::time::timeout(Duration::from_secs(3), offers.recv()).await.unwrap().unwrap();
                assert_eq!(offered[0].value["days"]["0"]["1"]["time"], time);
                assert_eq!(offered[0].value["days"]["0"]["1"]["temp"], temperature);
                assert_eq!(bridge.store.get("schedule.S").unwrap().revision, initial.revision);
                tokio::fs::write(&file, &saved).await.unwrap();
            };
            let (applied, ()) = tokio::join!(applied, report);
            assert_eq!(applied, Ok(()));
            let confirmed = bridge.store.get("schedule.S").unwrap();
            assert_eq!(confirmed.revision, initial.revision + 1);
            assert_eq!(confirmed.value["days"]["0"]["1"]["time"], time);
            assert!(offers.try_recv().is_err());
            initial = confirmed;
            hash = revision(&parse(&saved).unwrap()["heat"]);
        }
        tokio::fs::remove_file(&file).await.unwrap();
    }

    #[tokio::test]
    async fn inactive_cool_is_offered_and_confirmed_without_replacing_heat() {
        let bridge = test_bridge();
        let file = std::env::temp_dir().join(format!("nestlocal-inactive-cool-test-{}.recovery", std::process::id()));
        let heat = recovery_week("HEAT", 21600, 1428512);
        let recovery = |heat: &str, cool_time, cool_temp| {
            let cool = recovery_week("COOL", cool_time, cool_temp);
            format!(r#"ScheduleManager:"" {{ NumObjects:2, CurrentScheduleID:HEAT, {heat}, {cool}"#)
        };
        let saved = recovery(&heat, 21600, 24 * 65_536);
        tokio::fs::write(&file, &saved).await.unwrap();
        bridge.store.merge("shared.S", &fields! {"target_temperature_type" => "heat"}, 0, 0, true);
        let (before, _) = bridge.store.merge("schedule.S", &bucket(), 0, 0, true);
        let (mut offers, _subscription) = bridge.store.subscribe();
        let mut request =
            edit(revision(&parse(&saved).unwrap()["cool"]), "monday", &[("07:00:00", 25.0), ("22:00:00", 20.0)]);
        request.mode = "cool".into();
        let report = async {
            let offered = tokio::time::timeout(Duration::from_secs(3), offers.recv()).await.unwrap().unwrap();
            assert_eq!(offered.len(), 1);
            assert_eq!(offered[0].key, "schedule.S");
            assert_eq!(offered[0].value["schedule_mode"], "COOL");
            let monday = &offered[0].value["days"]["0"];
            assert_eq!(monday["1"]["time"], 25200);
            assert_eq!(monday["1"]["temp"], 25);
            assert_eq!(monday["2"]["type"], "COOL");
            assert_eq!(monday["2"]["touched_at"], 1234);
            assert_eq!(monday["2"]["touched_tzo"], -3600);
            tokio::fs::write(&file, recovery(&heat, 25200, 25 * 65_536)).await.unwrap();
        };
        let (result, ()) = tokio::join!(apply(&bridge, &request, &file), report);
        assert_eq!(result, Ok(()));
        assert_eq!(bridge.active_schedule(), Some("heat"));
        let active_bucket = bridge.store.get("schedule.S").unwrap();
        assert_eq!(active_bucket.revision, before.revision);
        assert_eq!(active_bucket.value, before.value);
        assert!(offers.try_recv().is_err());
        tokio::fs::remove_file(file).await.unwrap();
    }

    #[tokio::test]
    async fn inactive_range_midnight_saves_both_limits_without_replacing_heat() {
        let bridge = test_bridge();
        let file = std::env::temp_dir().join(format!("nestlocal-range-test-{}.recovery", std::process::id()));
        let heat = recovery_week("HEAT", 21600, 1428512);
        let range = recovery_week("RANGE", 21600, 21 * 65_536).replace("Temp:", "TempMax:1769472, Temp:");
        let recovery =
            |range: &str| format!(r#"ScheduleManager:"" {{ NumObjects:2, CurrentScheduleID:HEAT, {heat}, {range}"#);
        let saved = recovery(&range);
        tokio::fs::write(&file, &saved).await.unwrap();
        bridge.store.merge("shared.S", &fields! {"target_temperature_type" => "heat"}, 0, 0, true);
        let (before, _) = bridge.store.merge("schedule.S", &bucket(), 0, 0, true);
        let (mut offers, _subscription) = bridge.store.subscribe();
        let mut request = edit(revision(&parse(&saved).unwrap()["heat_cool"]), "saturday", &[]);
        request.mode = "heat_cool".into();
        request.setpoints =
            vec![Setpoint { time: "00:00:00".into(), temperature_c: None, heat_c: Some(18.0), cool_c: Some(19.0) }];
        assert_eq!(
            apply(&bridge, &request, &file).await,
            Err("The cooling setpoint must be at least 1.5 °C above heating.")
        );
        assert!(offers.try_recv().is_err());
        request.setpoints[0].cool_c = Some(26.0);
        let report = async {
            let offered = tokio::time::timeout(Duration::from_secs(3), offers.recv()).await.unwrap().unwrap();
            assert_eq!(offered.len(), 1);
            assert_eq!(offered[0].key, "schedule.S");
            assert_eq!(offered[0].value["schedule_mode"], "RANGE");
            let week = raw_week(&offered[0].value, "heat_cool").unwrap();
            assert_eq!(week[5][&0].raw["entry_type"], "setpoint");
            assert_eq!(week[5][&0].raw["temp-min"], 18);
            assert_eq!(week[5][&0].raw["temp-max"], 26);
            assert!(week[6][&0].continuation);
            assert_eq!(week[6][&0].temperature.high, Some(26.0));
            let reset = edit(String::new(), "saturday", &[]);
            let reverted = replace_day(&week, &reset, "RANGE").unwrap();
            assert!(reverted[5][&0].continuation);
            assert_eq!(reverted[5][&0].temperature.high, Some(27.0));
            let updated = range.replace(
                "TempMax:1769472, Temp:1310720, Day:5, Time:0, Default:1",
                "TempMax:1703936, Temp:1179648, Day:5, Time:0",
            );
            let updated = [6, 0].into_iter().fold(updated, |updated, day| {
                updated.replace(
                    &format!("TempMax:1769472, Temp:1310720, Day:{day}, Time:0, Default:1"),
                    &format!("TempMax:1703936, Temp:1179648, Day:{day}, Time:0, Default:1"),
                )
            });
            tokio::fs::write(&file, recovery(&updated)).await.unwrap();
        };
        let (result, ()) = tokio::join!(apply(&bridge, &request, &file), report);
        assert_eq!(result, Ok(()));
        assert_eq!(bridge.active_schedule(), Some("heat"));
        assert_eq!(bridge.store.get("schedule.S").unwrap().value, before.value);
        assert!(offers.try_recv().is_err());
        tokio::fs::remove_file(file).await.unwrap();
    }
}
