//! Room Sensors for stock Nest sensors on the gen 3.
use std::collections::BTreeMap;
use std::fmt::Write;
use std::io::{self, Read};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use crate::fields;
use crate::store::{now_ms, Store};
use crate::{logf, nlsettings, settings_ui};

const MANAGER: &str = "/media/data/rcsmanager.json";
// Also bounds the native screen's file read, even if a bad bucket holds a longer list.
const MAX_SENSORS: usize = 6;
const MAX_NAME_CHARS: usize = 24;
const MAX_ID_SUFFIX: usize = 32;
const RENAME_REQUEST: &str = "room-sensor-name.request";
const RENAME_TAKEN: &str = "room-sensor-name.taken";
const RENAME_STATUS: &str = "room-sensor-name.status";
type Names = BTreeMap<String, String>;

#[derive(Default)]
struct Page {
    summary: String,
    catalog: String,
}

pub async fn publish(store: Arc<Store>, serial: String, settings: String, state: String) {
    let path = Path::new(&state).with_extension("room-sensor-names.json");
    let mut names = load_names(&path).inspect_err(|e| logf!("room sensor names: {e}")).ok();
    let mut last = Page::default();
    let mut refresh = tokio::time::interval(Duration::from_secs(15));
    let mut requests = tokio::time::interval(Duration::from_secs(1));
    refresh.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    requests.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        let reply = tokio::select! {
            _ = refresh.tick() => None,
            _ = requests.tick() => {
                let Some(request) = take_rename() else { continue };
                rename_request(&request).map(|(token, id, name)| {
                    let settings = std::fs::read(&settings).unwrap_or_default();
                    let result = match &mut names {
                        Some(names) => rename_sensor(&store, &settings, &path, names, id, name),
                        None => Err("Name storage unavailable"),
                    };
                    format!("{token}\t{}", result.err().unwrap_or("Saved"))
                })
            }
        };
        let manager = read_manager();
        let settings = std::fs::read(&settings).unwrap_or_default();
        if let Some(names) = &names {
            sync_names(&store, &settings, names);
        }
        let page = render(&store, &serial, manager.as_ref(), &settings, names.as_ref(), now_ms() / 1000);
        replace_display("room-sensors.txt", &mut last.summary, page.summary);
        replace_display("room-sensors.catalog", &mut last.catalog, page.catalog);
        if let Some(reply) = reply {
            if let Err(e) = settings_ui::publish(RENAME_STATUS, reply.as_bytes()) {
                logf!("room sensor name status: {e}");
            }
        }
    }
}

fn replace_display(file: &str, previous: &mut String, text: String) {
    if *previous != text {
        match settings_ui::publish(file, text.as_bytes()) {
            Ok(()) => *previous = text,
            Err(e) => logf!("room sensors: {e}"),
        }
    }
}

fn take_rename() -> Option<String> {
    let directory = Path::new(settings_ui::DIR);
    let taken = directory.join(RENAME_TAKEN);
    std::fs::rename(directory.join(RENAME_REQUEST), &taken).ok()?;
    // The ring only sends a token, a sensor ID and one short UTF-8 name.
    let mut request = String::new();
    let result = std::fs::File::open(&taken).and_then(|file| file.take(256).read_to_string(&mut request));
    let _ = std::fs::remove_file(taken);
    result.ok().filter(|&len| len < 256).map(|_| request)
}

fn rename_request(request: &str) -> Option<(&str, &str, &str)> {
    let (token, payload) = request.split_once('\t')?;
    let (id, name) = payload.split_once('\t')?;
    (token.len() == 8 && token.bytes().all(|b| b.is_ascii_hexdigit())).then_some((token, id, name))
}

fn load_names(path: &Path) -> io::Result<Names> {
    match std::fs::read(path) {
        Ok(bytes) => {
            let mut names: Names = serde_json::from_slice(&bytes).map_err(io::Error::other)?;
            names.retain(|id, name| sensor_id(id) && valid_name(name));
            Ok(names)
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Names::new()),
        Err(e) => Err(e),
    }
}

fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.chars().count() <= MAX_NAME_CHARS && !name.chars().any(char::is_control)
}

fn sync_names(store: &Store, settings: &[u8], names: &Names) {
    for id in paired_ids(settings) {
        if let Some(name) = names.get(id).filter(|_| store.get(id).is_some()) {
            // A server-side bucket change reaches nlclient's sensor description
            // property, which also writes Nest's native .desc setting.
            store.merge(id, &fields! {"description" => name.as_str()}, 0, 0, false);
        }
    }
}

fn rename_sensor(
    store: &Store,
    settings: &[u8],
    path: &Path,
    names: &mut Names,
    id: &str,
    name: &str,
) -> Result<(), &'static str> {
    if !paired_ids(settings).any(|paired| paired == id) || store.get(id).is_none() {
        return Err("Sensor no longer paired");
    }
    let name = name.trim();
    if !valid_name(name) {
        return Err("Enter 1 to 24 characters");
    }
    if names.get(id).is_none_or(|previous| previous != name) {
        let mut updated = names.clone();
        updated.insert(id.to_owned(), name.to_owned());
        let bytes = serde_json::to_vec(&updated).map_err(|_| "Could not save name")?;
        if let Err(e) = crate::durable::replace(path, &bytes, 0o600) {
            logf!("room sensor name: {e}");
            return Err("Could not save name");
        }
        *names = updated;
    }
    store.merge(id, &fields! {"description" => name}, 0, 0, false);
    Ok(())
}

fn sensor_id(id: &str) -> bool {
    id.strip_prefix("kryptonite.").is_some_and(|suffix| {
        !suffix.is_empty() && suffix.len() <= MAX_ID_SUFFIX && suffix.bytes().all(|b| b.is_ascii_hexdigit())
    })
}

fn paired_ids(settings: &[u8]) -> impl Iterator<Item = &str> {
    setting(settings, "associated_rcs_sensors").unwrap_or_default().split(',').map(str::trim).filter(move |id| {
        sensor_id(id) && nlsettings::value(settings, &format!("{id}.key")).is_some_and(|key| !key.is_empty())
    })
}

fn setting<'a>(settings: &'a [u8], key: &str) -> Option<&'a str> {
    std::str::from_utf8(nlsettings::value(settings, key)?).ok().filter(|value| !value.is_empty())
}

pub fn saved_structure(settings: &[u8]) -> Option<&str> {
    let id = setting(settings, "structure_id")?;
    (paired_ids(settings).next().is_some()
        && id.len() == 36
        && id.bytes().enumerate().all(
            |(i, c)| {
                if matches!(i, 8 | 13 | 18 | 23) {
                    c == b'-'
                } else {
                    c.is_ascii_hexdigit()
                }
            },
        ))
    .then_some(id)
}

/// Give nlclient the local equivalents of the cloud buckets for sensors it
/// was already paired with. The original pairing keys and recovery files
/// stay with nlclient, and the existing HVAC selection stays authoritative.
pub fn seed_registry(store: &Store, serial: &str, settings: &[u8], manager: Option<&Value>) {
    let mut sensors = Vec::new();
    for id in paired_ids(settings) {
        if !sensors.contains(&id) {
            sensors.push(id);
        }
    }
    if sensors.is_empty() {
        return;
    }
    let Some(structure) = super::structure_key(store, serial).and_then(|key| store.get(&key)) else { return };
    let Some(structure_id) = structure.key.strip_prefix("structure.") else { return };
    let mut swarm = super::listed(structure.value.get("swarm"));
    for id in &sensors {
        if !swarm.iter().any(|item| item == *id) {
            swarm.push((*id).into());
        }
        if store.get(id).is_none() {
            store.merge(
                id,
                &fields! {
                    "model" => "KR1",
                    "serial_number" => id.trim_start_matches("kryptonite."),
                    "where_id" => setting(settings, &format!("{id}.whereid")).unwrap_or(super::DEFAULT_WHERE),
                    "description" => setting(settings, &format!("{id}.desc")).unwrap_or("Nest Temperature Sensor"),
                    "structure_id" => structure_id,
                    "report_period_in_seconds" => 60,
                },
                0,
                0,
                false,
            );
        }
    }
    store.merge(&structure.key, &fields! {"swarm" => swarm}, 0, 0, false);

    let key = format!("rcs_settings.{serial}");
    let prior = store.get(&key).map(|item| Value::Object(item.value));
    let mut associated = super::listed(prior.as_ref().and_then(|item| item.get("associated_rcs_sensors")));
    for id in &sensors {
        if !associated.iter().any(|item| item == *id) {
            associated.push((*id).into());
        }
    }
    let mut fields = fields! {"associated_rcs_sensors" => associated};
    if prior.is_none() {
        let active = active_sensors(manager, None)
            .unwrap_or_default()
            .into_iter()
            .filter(|id| sensors.contains(id))
            .collect::<Vec<_>>();
        fields.extend(fields! {
            "active_rcs_sensors" => active,
            "rcs_control_setting" => "OVERRIDE",
            "multiroom_active" => false,
            "thermostat_alert" => "OFF",
        });
    }
    store.merge(&key, &fields, 0, 0, false);
}

pub fn read_manager() -> Option<Value> {
    super::read_json(Path::new(MANAGER))
}

fn sensor_name(sensor: Option<&Value>, rooms: Option<&Value>, custom: Option<&String>, index: usize) -> String {
    if let Some(name) = custom {
        return name.clone();
    }
    let room = sensor.and_then(|value| value["where_id"].as_str()).and_then(|id| {
        rooms?
            .get("wheres")?
            .as_array()?
            .iter()
            .find(|entry| entry["where_id"].as_str() == Some(id))?
            .get("name")?
            .as_str()
    });
    let description = sensor
        .and_then(|value| value["description"].as_str())
        .filter(|description| !description.eq_ignore_ascii_case("Nest Temperature Sensor"));
    let name = room
        .or(description)
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_NAME_CHARS)
        .collect::<String>();
    if name.is_empty() {
        format!("Sensor {}", index + 1)
    } else {
        name
    }
}

fn reading_age(sensor: &Value, record: Option<&Value>, now: i64) -> Option<i64> {
    // A bucket revision can change for a rename or a battery update. Prefer
    // the stock RCS manager's reading time when the sensor has no sample time.
    let timestamp =
        sensor["time_last_sensor_reading"].as_i64().or_else(|| record?.get("time_latest_temperature")?.as_i64())?;
    let seconds = if timestamp > 100_000_000_000 { timestamp / 1000 } else { timestamp };
    (seconds > 0 && seconds <= now).then(|| now - seconds)
}

fn active_sensors<'a>(manager: Option<&'a Value>, registry: Option<&'a Value>) -> Option<Vec<&'a str>> {
    if let Some(manager) = manager {
        if manager["selected_sensor_type"].as_str() == Some("BACKPLATE") {
            return Some(Vec::new());
        }
        if let Some(active) = manager["active_sensors"].as_str() {
            let sensors: Vec<_> = active.split(',').map(str::trim).filter(|id| sensor_id(id)).collect();
            if !sensors.is_empty() {
                return Some(sensors);
            }
        }
    }
    registry?.get("active_rcs_sensors")?.as_array().map(|ids| ids.iter().filter_map(Value::as_str).collect())
}

fn render(
    store: &Store,
    serial: &str,
    manager: Option<&Value>,
    settings: &[u8],
    aliases: Option<&Names>,
    now: i64,
) -> Page {
    let registry = store.get(&format!("rcs_settings.{serial}")).map(|item| Value::Object(item.value));
    let saved = nlsettings::value(settings, "associated_rcs_sensors").and_then(|b| std::str::from_utf8(b).ok());
    let associated = registry.as_ref().and_then(|value| value.get("associated_rcs_sensors"));
    let mut sensors = Vec::new();
    let candidates: Vec<&str> = if let Some(Value::Array(ids)) = associated {
        ids.iter().filter_map(Value::as_str).collect()
    } else {
        saved.unwrap_or_default().split(',').map(str::trim).collect()
    };
    for id in candidates.into_iter().filter(|id| sensor_id(id)) {
        if !sensors.contains(&id) {
            sensors.push(id);
            if sensors.len() == MAX_SENSORS {
                break;
            }
        }
    }

    let rooms = super::structure_key(store, serial)
        .and_then(|key| store.get(&format!("where.{}", key.strip_prefix("structure.").unwrap_or(&key))))
        .map(|item| Value::Object(item.value));
    let fahrenheit = store
        .get(&format!("device.{serial}"))
        .is_some_and(|item| item.value.get("temperature_scale").and_then(Value::as_str) == Some("F"));
    let records = manager.and_then(|state| state["associated_sensor_info"].as_array());
    let active = active_sensors(manager, registry.as_ref());
    let names: Vec<_> = sensors
        .iter()
        .enumerate()
        .map(|(index, id)| {
            let sensor = store.get(id).map(|item| Value::Object(item.value));
            let name = sensor_name(sensor.as_ref(), rooms.as_ref(), aliases.and_then(|names| names.get(*id)), index);
            (name, sensor)
        })
        .collect();
    let control = match active.as_deref() {
        Some([]) => "Controlling: Thermostat".into(),
        Some([id]) if sensors.contains(id) => {
            format!("Controlling: {}", names[sensors.iter().position(|sensor| sensor == id).unwrap()].0)
        }
        Some(ids) if !ids.is_empty() => "Controlling: Room sensors".into(),
        _ => "Controlling source unavailable".into(),
    };
    let mut text = control;
    let mut catalog = String::new();
    if sensors.is_empty() {
        text.push_str("\n\nNo paired room sensors");
    }
    for (id, (name, sensor)) in sensors.iter().zip(names) {
        let _ = writeln!(catalog, "{id}\t{name}");
        let _ = write!(text, "\n\n{name}\n");
        let temperature = sensor.as_ref().and_then(|sensor| {
            sensor["current_temperature"]
                .as_f64()
                .or_else(|| sensor["temperature"].as_f64())
                .filter(|value| (-50.0..=80.0).contains(value))
        });
        let Some(temperature) = temperature else {
            text.push_str("No temperature reported");
            continue;
        };
        if fahrenheit {
            let _ = writeln!(text, "{:.1} °F", temperature * 1.8 + 32.0);
        } else {
            let _ = writeln!(text, "{temperature:.1} °C");
        }
        let record = records.and_then(|entries| entries.iter().find(|entry| entry["ID"].as_str() == Some(id)));
        let age = sensor.as_ref().and_then(|value| reading_age(value, record, now));
        match age {
            Some(age) if age >= 3600 => {
                let _ = write!(text, "{}h {}m ago", age / 3600, age / 60 % 60);
            }
            Some(age) if age >= 60 => {
                let _ = write!(text, "{}m ago", age / 60);
            }
            Some(_) => text.push_str("Less than 1m ago"),
            None => text.push_str("Reading time unavailable"),
        }
    }
    Page { summary: text, catalog }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_fresh_local_store_adopts_existing_stock_pairings_and_keeps_the_current_selection() {
        let store = crate::store::tests::temp_store();
        let settings = br#"<settings>
            <s key="structure_id" value="01234567-89ab-cdef-0123-456789abcdef"/>
            <s key="associated_rcs_sensors" value="kryptonite.AB,kryptonite.CD,kryptonite.EF,"/>
            <s key="kryptonite.AB.key" value="test-key-one"/>
            <s key="kryptonite.AB.whereid" value="office"/>
            <s key="kryptonite.AB.desc" value="Office"/>
            <s key="kryptonite.CD.key" value="test-key-two"/>
        </settings>"#;
        let home = saved_structure(settings).unwrap();
        crate::structure::ensure_structure(&store, "S", Some(home));
        let manager = json!({"active_sensors":"kryptonite.CD"});
        seed_registry(&store, "S", settings, Some(&manager));
        assert_eq!(store.get("link.S").unwrap().value["structure"], json!(format!("structure.{home}")));
        assert_eq!(
            store.get(&format!("structure.{home}")).unwrap().value["swarm"],
            json!(["kryptonite.AB", "kryptonite.CD"])
        );
        let sensor = store.get("kryptonite.AB").unwrap();
        assert_eq!(sensor.value["where_id"], "office");
        assert_eq!(sensor.value["description"], "Office");
        assert!(store.get("kryptonite.EF").is_none());
        assert!(!serde_json::to_string(&store.all()).unwrap().contains("test-key"));
        let registry = store.get("rcs_settings.S").unwrap();
        assert_eq!(registry.value["associated_rcs_sensors"], json!(["kryptonite.AB", "kryptonite.CD"]));
        assert_eq!(registry.value["active_rcs_sensors"], json!(["kryptonite.CD"]));

        store.merge("kryptonite.AB", &fields! {"current_temperature" => 21.2, "description" => "Renamed"}, 0, 0, true);
        seed_registry(&store, "S", settings, Some(&json!({"selected_sensor_type":"BACKPLATE"})));
        assert_eq!(store.get("rcs_settings.S").unwrap().revision, registry.revision);
        let sensor = store.get("kryptonite.AB").unwrap();
        assert_eq!(sensor.value["description"], "Renamed");
        assert_eq!(sensor.value["current_temperature"], 21.2);
    }

    #[test]
    fn room_name_reading_age_and_source_come_from_the_stock_records() {
        let store = crate::store::tests::temp_store();
        store.merge("link.S", &fields! {"structure" => "structure.home"}, 0, 0, false);
        store.merge("where.home", &fields! {"wheres" => json!([{"where_id":"a","name":"Office"}])}, 0, 0, false);
        store.merge("device.S", &fields! {"temperature_scale" => "F"}, 0, 0, false);
        store.merge("kryptonite.AB", &fields! {"current_temperature" => 22.2, "where_id" => "a"}, 0, 0, true);
        store.merge("kryptonite.CD", &fields! {"current_temperature" => 23.0}, 0, 0, true);
        store.merge(
            "rcs_settings.S",
            &fields! {
                "associated_rcs_sensors" => json!(["kryptonite.AB","kryptonite.CD"]),
                "active_rcs_sensors" => json!(["kryptonite.CD"]),
            },
            0,
            0,
            true,
        );
        let manager = json!({"selected_sensor_type":"BACKPLATE","associated_sensor_info":[
            {"ID":"kryptonite.AB","time_latest_temperature":700},
            {"ID":"kryptonite.CD","time_latest_temperature":985},
        ]});
        assert_eq!(
            render(&store, "S", Some(&manager), b"", None, 1000).summary,
            "Controlling: Thermostat\n\nOffice\n72.0 °F\n5m ago\n\nSensor 2\n73.4 °F\nLess than 1m ago"
        );
        assert_eq!(
            render(&store, "S", Some(&manager), b"", None, 4300).summary,
            "Controlling: Thermostat\n\nOffice\n72.0 °F\n1h 0m ago\n\nSensor 2\n73.4 °F\n55m ago"
        );
    }

    #[test]
    fn missing_registry_uses_saved_associations_without_inventing_a_reading() {
        let store = crate::store::tests::temp_store();
        let settings = br#"<s key="associated_rcs_sensors" value="kryptonite.AB,kryptonite.AB,other.CD,"/>"#;
        assert_eq!(
            render(&store, "S", None, settings, None, 1000).summary,
            "Controlling source unavailable\n\nSensor 1\nNo temperature reported"
        );
        store.merge(
            "rcs_settings.S",
            &fields! {"associated_rcs_sensors" => json!([]), "active_rcs_sensors" => json!([])},
            0,
            0,
            true,
        );
        assert_eq!(
            render(&store, "S", None, settings, None, 1000).summary,
            "Controlling: Thermostat\n\nNo paired room sensors"
        );
    }

    #[test]
    fn rename_persists_and_sends_only_the_description_to_stock_subscribers() {
        let store = crate::store::tests::temp_store();
        let settings = br#"<settings>
            <s key="associated_rcs_sensors" value="kryptonite.AB,kryptonite.CD,"/>
            <s key="kryptonite.AB.key" value="test-key"/>
        </settings>"#;
        let dir = std::env::temp_dir().join(format!("nestlocal-room-name-{}-{}", std::process::id(), now_ms()));
        std::fs::create_dir(&dir).unwrap();
        let path = dir.join("names.json");
        let id = "kryptonite.AB";
        store.merge("link.S", &fields! {"structure" => "structure.home"}, 0, 0, false);
        store.merge("where.home", &fields! {"wheres" => json!([{"where_id":"office","name":"Office"}])}, 0, 0, false);
        let (original, _) = store.merge(
            id,
            &fields! {
                "current_temperature" => 22.2,
                "where_id" => "office",
                "description" => "Nest Temperature Sensor",
            },
            0,
            0,
            true,
        );
        let (mut changes, _subscription) = store.subscribe();
        let mut names = load_names(&path).unwrap();
        assert_eq!(
            rename_sensor(&store, settings, &path, &mut names, "kryptonite.CD", "Unpaired"),
            Err("Sensor no longer paired")
        );
        assert_eq!(
            rename_sensor(&store, settings, &path, &mut names, id, "Bad\tname"),
            Err("Enter 1 to 24 characters")
        );
        assert!(!path.exists());

        rename_sensor(&store, settings, &path, &mut names, id, " Upstairs ").unwrap();
        let persisted = load_names(&path).unwrap();
        assert_eq!(persisted.get(id).map(String::as_str), Some("Upstairs"));
        let buckets = changes.try_recv().unwrap();
        let mut expected = original.value;
        expected.insert("description".into(), json!("Upstairs"));
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].key, id);
        assert_eq!(buckets[0].value, expected);
        assert!(store.get("rcs_settings.S").is_none());
        let manager = json!({"active_sensors":id});
        let page = render(&store, "S", Some(&manager), settings, Some(&persisted), 1000);
        assert!(page.summary.starts_with("Controlling: Upstairs\n\nUpstairs\n22.2 °C"));
        assert!(page.catalog.starts_with("kryptonite.AB\tUpstairs\n"));

        store.merge(id, &fields! {"description" => "Old label"}, 0, 0, true);
        sync_names(&store, settings, &persisted);
        assert_eq!(changes.try_recv().unwrap()[0].value, expected);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
