const styles = `
:host { display: block; container-type: inline-size; }
  *, *::before, *::after { box-sizing: border-box; }
  [hidden] { display: none !important; }
  .ha-schedule {
    font-size: 14px;
    line-height: 1.45;
    --ha-surface: var(--ha-card-background, var(--card-background-color, white));
    --ha-subtle: color-mix(in srgb, var(--ha-text) 4%, var(--ha-surface));
    --ha-text: var(--primary-text-color, #21262b);
    --ha-secondary: var(--secondary-text-color, #5d6872);
    --ha-border: var(--divider-color, #e3e8ed);
    --ha-blue: light-dark(#006f99, #79cff5);
    --ha-blue-bg: color-mix(in srgb, var(--primary-color, #03a9f4) 15%, var(--ha-surface));
    --ha-track: light-dark(#edf0f2, #292b2d);
    --ha-heat: light-dark(#9e471b, #ffc3a0);
    --ha-low: light-dark(#faeadf, #413027);
    --ha-medium: light-dark(#f6dbc9, #4e3629);
    --ha-high: light-dark(#f3caab, #5b3c2b);
    --ha-cool: light-dark(#ddecf8, #203c4c);
    --ha-range: light-dark(#dff0ed, #223e3c);
    background: var(--ha-surface);
    color: var(--ha-text);
    overflow: hidden;
  }
  h2, h3, p { margin: 0; }
  .ha-header { display: flex; align-items: center; gap: 13px; padding: 20px 22px 16px; }
  .ha-header h2 { font: 500 21px/1.2 Roboto, "Helvetica Neue", Arial, sans-serif; color: var(--ha-text); }
  .ha-header p { color: var(--ha-secondary); font-size: 13px; margin-top: 4px; }
  .ha-symbol {
    width: 43px; height: 43px; flex: none; display: grid; place-items: center;
    border-radius: 50%; background: var(--ha-blue-bg); color: var(--ha-blue);
  }
  .ha-symbol ha-icon { width: 22px; height: 22px; }
  .ha-mode-bar {
    display: flex; align-items: center; justify-content: space-between; flex-wrap: wrap; gap: 9px 16px;
    padding: 0 22px 16px; border-bottom: 1px solid var(--ha-border);
  }
  .ha-modes { display: flex; gap: 3px; padding: 3px; border-radius: 22px; background: var(--ha-subtle); }
  button {
    font: inherit; color: inherit; background: transparent; border: 1px solid transparent; border-radius: 9px;
  }
  button:focus-visible, input:focus-visible {
    outline: 2px solid var(--ha-blue); outline-offset: 2px;
  }
  .ha-modes button { border-radius: 18px; padding: 6px 13px; min-height: 32px; white-space: nowrap; }
  .ha-modes button[aria-pressed="true"] { background: var(--ha-blue-bg); color: var(--ha-blue); font-weight: 600; }
  .ha-mode-status { display: flex; align-items: center; gap: 6px; color: var(--ha-secondary); font-size: 12px; }
  .ha-mode-status span { width: 7px; height: 7px; border-radius: 100%; background: var(--ha-blue); }
  .ha-workspace { display: grid; grid-template-columns: minmax(0, 1.8fr) minmax(227px, 1fr); }
  .ha-week { min-width: 0; padding: 18px 18px 21px 20px; }
  .ha-week-heading { display: flex; align-items: baseline; justify-content: space-between; gap: 8px; margin-bottom: 10px; }
  h3 { font-size: 15px; font-weight: 600; color: var(--ha-text); }
  .ha-week-heading span, .ha-axis { color: var(--ha-secondary); font-size: 12px; }
  .ha-axis { display: flex; justify-content: space-between; margin: 0 5px 5px 55px; font-variant-numeric: tabular-nums; }
  .ha-row {
    width: 100%; display: grid; grid-template-columns: 42px minmax(0, 1fr); align-items: center; gap: 7px;
    padding: 5px; margin: 1px 0; text-align: left; min-height: 35px;
  }
  .ha-row:hover { background: var(--ha-subtle); }
  .ha-row[aria-pressed="true"] { background: var(--ha-blue-bg); color: var(--ha-blue); }
  .ha-day-label { padding-left: 3px; font-size: 13px; font-weight: 500; }
  .ha-track { display: flex; height: 23px; min-width: 0; align-items: stretch; overflow: hidden; border-radius: 5px; background: var(--ha-track); }
  .ha-segment {
    flex-shrink: 0; display: flex; align-items: center; justify-content: center; overflow: hidden; white-space: nowrap;
    color: var(--ha-heat); background: var(--ha-low); box-shadow: inset -1px 0 var(--ha-surface);
    font-size: 11px; font-weight: 600; font-variant-numeric: tabular-nums;
  }
  .ha-segment:last-child { box-shadow: none; }
  .ha-segment[data-level="medium"] { background: var(--ha-medium); }
  .ha-segment[data-level="high"] { background: var(--ha-high); }
  .ha-segment[data-tone="cool"] { background: var(--ha-cool); color: var(--ha-blue); }
  .ha-segment[data-tone="range"] { background: var(--ha-range); color: light-dark(#2e635a, #a4dace); }
  .ha-day {
    min-width: 0; padding: 18px 18px 21px; border-left: 1px solid var(--ha-border); background: var(--ha-subtle);
  }
  .ha-detail-title { display: flex; justify-content: space-between; align-items: center; gap: 6px; margin-bottom: 10px; }
  .ha-action { padding: 4px 7px; color: var(--ha-blue); font-weight: 600; }
  .ha-action:hover { background: var(--ha-blue-bg); }
  .ha-action.ha-save { background: var(--ha-blue); color: light-dark(#fff, #05202c); padding: 5px 11px; }
  .ha-setpoint {
    display: grid; grid-template-columns: minmax(0,1fr) auto; align-items: center; gap: 8px;
    min-height: 40px; padding: 7px 0; border-bottom: 1px solid var(--ha-border);
    font-variant-numeric: tabular-nums;
  }
  .ha-setpoint span:first-child { color: var(--ha-secondary); }
  .ha-setpoint strong { font-weight: 600; }
  .ha-setpoint:last-child { border-bottom: 0; }
  .ha-note { color: var(--ha-secondary); margin: 6px 0; font-size: 12px; }
  .ha-edit-headings, .ha-edit-row {
    display: grid; grid-template-columns: minmax(37px, 1fr) minmax(42px, 1fr) minmax(49px, 1fr) 25px; gap: 6px;
    align-items: center; margin-bottom: 7px;
  }
  .ha-range-edit {
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr) minmax(0, 1.2fr) minmax(0, 1.2fr) 22px;
    gap: 4px;
  }
  .ha-midnight { padding-bottom: 8px; border-bottom: 1px solid var(--ha-border); }
  .ha-midnight-fields { display: grid; grid-auto-flow: column; grid-auto-columns: minmax(0, 1fr); gap: 8px; margin: 6px 0; }
  .ha-midnight-fields label { display: grid; gap: 3px; color: var(--ha-secondary); font-size: 12px; }
  .ha-edit-headings { color: var(--ha-secondary); font-size: 12px; margin-top: 9px; }
  input {
    min-width: 0; width: 100%; min-height: 34px; border: 1px solid var(--ha-border); border-radius: 7px; padding: 4px 5px;
    color: var(--ha-text); background: var(--ha-surface); font: inherit;
  }
  .ha-remove { font-size: 18px; padding: 0; color: var(--ha-secondary); min-height: 32px; }
  .ha-remove:hover { background: var(--ha-blue-bg); }
  .ha-actions { display: flex; justify-content: end; gap: 6px; margin-top: 13px; }
  .ha-error { color: light-dark(#a72f30,#ffabab); font-size: 12px; margin-top: 6px; }
  input[aria-invalid="true"] { border-color: var(--error-color, #bb4949); }
  @container (max-width: 620px) {
    .ha-workspace { grid-template-columns: minmax(0,1fr); }
    .ha-day { border-left: 0; border-top: 1px solid var(--ha-border); }
    .ha-segment[data-short="true"] span { display: none; }
  }
  @container (max-width: 380px) {
    .ha-header { padding: 15px 13px 12px; }
    .ha-mode-bar { padding: 0 13px 12px; }
    .ha-week, .ha-day { padding-inline: 12px; }
    .ha-segment[data-small="true"] span { display: none; }
  }
  @media (pointer: coarse) {
    .ha-row, .ha-modes button, .ha-action,
    input, .ha-remove { min-height: 44px; }
    input { font-size: 16px; }
  }
  .ha-empty { padding: 0 22px 20px; color: var(--ha-secondary); }
  button:disabled { opacity: .6; }
`;

const MODES = { heat: "Heat", cool: "Cool", heat_cool: "Heat • Cool" };
const SINGLE_TEMPERATURE = [{ key: "temperature_c", field: "temp", label: "°C" }];
const RANGE_TEMPERATURES = [
  { key: "heat_c", field: "heat", label: "Heat °C" },
  { key: "cool_c", field: "cool", label: "Cool °C" },
];
const DAYS = ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"];
const SAVE_TIMEOUT = 40_000;

const escapeHtml = (value) =>
  String(value).replace(
    /[&<>"']/g,
    (character) =>
      ({
        "&": "&amp;",
        "<": "&lt;",
        ">": "&gt;",
        '"': "&quot;",
        "'": "&#39;",
      })[character],
  );
const displayNumber = (value) => String(Number(value.toFixed(1)));
const degrees = (point) =>
  Number.isFinite(point.temperature_c)
    ? `${displayNumber(point.temperature_c)} °C`
    : `${displayNumber(point.heat_c)}–${displayNumber(point.cool_c)} °C`;
const minutes = (time) => {
  const [hours, minutes, seconds = 0] = time.split(":").map(Number);
  return hours * 60 + minutes + seconds / 60;
};
const timeLabel = (time) => (time.endsWith(":00") ? time.slice(0, 5) : time);
const dayLabel = (day) => day[0].toUpperCase() + day.slice(1);

function timePart(value, maximum) {
  const digits = value.trim();
  return /^\d{1,2}$/.test(digits) && Number(digits) <= maximum ? digits.padStart(2, "0") : undefined;
}

class NestScheduleCard extends HTMLElement {
  constructor() {
    super();
    this.attachShadow({ mode: "open" });
    this.day = (new Date().getDay() + 6) % 7;
  }

  static getStubConfig(hass) {
    const entity = Object.values(hass.states).find((state) => state.attributes.schedules && state.attributes.topic);
    return { entity: entity?.entity_id ?? "sensor.nestlocal_weekly_schedule" };
  }

  setConfig(config) {
    if (!config.entity?.startsWith("sensor.")) {
      throw new Error("Choose the thermostat's Weekly schedule sensor.");
    }
    if (this.config?.entity !== config.entity) {
      this.mode = undefined;
      this.draft = undefined;
      this.message = undefined;
      this.clearPending();
    }
    this.config = config;
    if (this.homeAssistant) {
      this.entity = undefined;
      this.hass = this.homeAssistant;
    } else {
      this.render();
    }
  }

  set hass(hass) {
    this.homeAssistant = hass;
    const entity = hass.states[this.config?.entity];
    if (this.entity === entity) return;
    this.entity = entity;
    const result = this.attributes.last_edit;
    if (this.pending && result?.request_id === this.pending) {
      this.clearPending();
      if (result.status === "saved") {
        this.draft = undefined;
        this.message = { text: "Saved on the Nest." };
      } else {
        this.message = { text: result.message || "The Nest could not save the schedule.", error: true };
      }
    } else if (this.draft) {
      // Home Assistant pushes state while the user types. Keep the live input and
      // the original revision until Save or Cancel.
      return;
    }
    if (!this.schedule(this.mode)) {
      this.mode = this.schedule(this.attributes.mode)
        ? this.attributes.mode
        : Object.keys(MODES).find((mode) => this.schedule(mode));
    }
    this.render();
  }

  getCardSize() {
    return 7;
  }

  getGridOptions() {
    return { columns: 12, rows: "auto", min_columns: 6 };
  }

  get attributes() {
    return this.entity?.attributes ?? {};
  }

  schedule(mode) {
    return typeof mode === "string" && Object.hasOwn(MODES, mode) ? this.attributes.schedules?.[mode] : undefined;
  }

  dayPoints(index) {
    const points = this.schedule(this.mode)?.days?.[DAYS[index]];
    return Array.isArray(points)
      ? points.filter(
          (point) =>
            typeof point?.time === "string" &&
            /^\d\d:\d\d:\d\d$/.test(point.time) &&
            minutes(point.time) >= 0 &&
            minutes(point.time) < 1440 &&
            (Number.isFinite(point.temperature_c) || (Number.isFinite(point.heat_c) && Number.isFinite(point.cool_c))),
        )
      : [];
  }

  get editRevision() {
    const revisions = this.attributes.edit_revisions;
    if (revisions) return revisions[this.mode];
    return this.mode === this.attributes.mode ? this.attributes.edit_revision : undefined;
  }

  canEdit() {
    return (
      typeof this.editRevision === "string" &&
      this.entity?.state !== "unavailable" &&
      typeof this.attributes.topic === "string"
    );
  }

  selectMode(mode, control) {
    this.mode = mode;
    this.message = undefined;
    this.render();
    this.shadowRoot.querySelector(control)?.focus();
  }

  render() {
    const schedule = this.schedule(this.mode);
    this.shadowRoot.innerHTML = `
      <style>${styles}</style>
      <ha-card class="ha-schedule">
        <header class="ha-header">
          <div class="ha-symbol"><ha-icon icon="mdi:calendar-clock" aria-hidden="true"></ha-icon></div>
          <div><h2>Temperature schedule</h2><p>Nest Learning Thermostat</p></div>
        </header>
        ${schedule ? this.scheduleView() : '<p class="ha-empty" role="status">Waiting for the thermostat’s weekly schedule.</p>'}
      </ha-card>`;
    if (!schedule) return;
    for (const button of this.shadowRoot.querySelectorAll("[data-mode]")) {
      button.addEventListener("click", () =>
        this.selectMode(button.dataset.mode, `[data-mode="${button.dataset.mode}"]`),
      );
    }
    for (const button of this.shadowRoot.querySelectorAll("[data-day]")) {
      button.addEventListener("click", () => {
        this.day = Number(button.dataset.day);
        this.message = undefined;
        this.render();
        this.shadowRoot.querySelector(`[data-day="${this.day}"]`)?.focus();
      });
    }
    this.bindDay();
  }

  scheduleView() {
    const active = this.mode === this.attributes.mode;
    const status =
      this.entity?.state === "unavailable" ? "Thermostat unavailable" : active ? "Active schedule" : "Saved schedule";
    return `
      <div class="ha-mode-bar">
        <div class="ha-modes" role="group" aria-label="Schedule mode">
          ${Object.entries(MODES)
            .map(
              ([mode, name]) =>
                `<button type="button" data-mode="${mode}" aria-pressed="${this.mode === mode}" ${!this.schedule(mode) || this.draft ? "disabled" : ""}>${name}</button>`,
            )
            .join("")}
        </div>
        <span class="ha-mode-status">${active && this.entity?.state !== "unavailable" ? "<span aria-hidden='true'></span>" : ""}${status}</span>
      </div>
      <div class="ha-workspace">
        <div class="ha-week">
          <div class="ha-week-heading"><h3>Weekly overview</h3><span>Local time</span></div>
          <div class="ha-axis" aria-hidden="true"><span>00</span><span>06</span><span>12</span><span>18</span><span>24</span></div>
          ${DAYS.map(
            (name, index) => `
            <button class="ha-row" type="button" data-day="${index}" aria-pressed="${index === this.day}" aria-label="${dayLabel(name)} schedule" ${this.draft ? "disabled" : ""}>
              <span class="ha-day-label">${dayLabel(name).slice(0, 3)}</span>
              <span class="ha-track" aria-hidden="true">${this.segments(this.dayPoints(index))}</span>
            </button>`,
          ).join("")}
        </div>
        <section class="ha-day" aria-label="Selected day">${this.dayView()}</section>
      </div>`;
  }

  segments(points) {
    return points
      .map((point, index) => {
        const start = minutes(point.time);
        const width = ((points[index + 1] ? minutes(points[index + 1].time) : 1440) - start) / 14.4;
        const level = point.temperature_c >= 22.5 ? "high" : point.temperature_c >= 21 ? "medium" : "low";
        const value = width >= 8 && (!index || degrees(points[index - 1]) !== degrees(point)) ? degrees(point) : "";
        const margin = index === 0 ? start / 14.4 : 0;
        return `<span class="ha-segment" data-tone="${this.mode === "heat_cool" ? "range" : this.mode}" data-level="${level}" data-small="${width < 25}" data-short="${width < 12}" style="flex-basis:${width}%;margin-left:${margin}%"><span>${value}</span></span>`;
      })
      .join("");
  }

  startingPoint(points) {
    return points.find((point) => point.time === "00:00:00");
  }

  get temperatureFields() {
    return this.mode === "heat_cool" ? RANGE_TEMPERATURES : SINGLE_TEMPERATURE;
  }

  temperatureDraft(point) {
    return Object.fromEntries(
      this.temperatureFields.map(({ key }) => [key, { original: point[key], display: displayNumber(point[key]) }]),
    );
  }

  temperatureInput(field, row, index, context, disabled) {
    const value = escapeHtml(row[field.key].display);
    return `<input type="text" inputmode="decimal" autocomplete="off" value="${value}" aria-label="${field.label} ${context}" data-${field.field}="${index}" required ${disabled}>`;
  }

  midnightEditor(disabled) {
    const midnight = this.draft.midnight;
    if (!midnight) {
      return `<div class="ha-midnight">
        <p class="ha-note">Uses the previous day’s target at midnight</p>
        <button type="button" class="ha-action" data-add-midnight ${disabled}>Set midnight target</button>
      </div>`;
    }
    return `<div class="ha-midnight">
      <h3>Starts at midnight</h3>
      <div class="ha-midnight-fields">
        ${this.temperatureFields.map((field) => `<label>${field.label}${this.temperatureInput(field, midnight.temperatures, "midnight", "at midnight", disabled)}</label>`).join("")}
      </div>
      <button type="button" class="ha-action" data-clear-midnight ${disabled}>Use previous day’s target</button>
    </div>`;
  }

  dayView() {
    const name = dayLabel(DAYS[this.day]);
    const points = this.dayPoints(this.day);
    const start = this.startingPoint(points);
    const starting = start
      ? `<div class="ha-setpoint"><span>Starts at midnight</span><strong>${degrees(start)}</strong></div>`
      : "";
    const feedback = this.message
      ? `<p id="ha-schedule-feedback" class="${this.message.error ? "ha-error" : "ha-note"}" role="${this.message.error ? "alert" : "status"}">${escapeHtml(this.message.text)}</p>`
      : "";
    if (this.draft) {
      const disabled = this.pending ? "disabled" : "";
      const rangeClass = this.mode === "heat_cool" ? " ha-range-edit" : "";
      return `
        <div class="ha-detail-title"><h3>Edit ${name}</h3></div>
        <form novalidate>
          ${this.midnightEditor(disabled)}
          <div class="ha-edit-headings${rangeClass}" aria-hidden="true"><span>Hour</span><span>Minute</span>${this.temperatureFields.map(({ label }) => `<span>${label}</span>`).join("")}</div>
          ${this.draft.points
            .map(
              (point, index) => `
            <div class="ha-edit-row${rangeClass}">
              <input type="text" inputmode="numeric" maxlength="2" placeholder="HH" autocomplete="off" value="${escapeHtml(point.hour)}" aria-label="Hour (0 to 23) for setpoint ${index + 1}" data-hour="${index}" required ${disabled}>
              <input type="text" inputmode="numeric" maxlength="2" placeholder="MM" autocomplete="off" value="${escapeHtml(point.minute)}" aria-label="Minute (0 to 59) for setpoint ${index + 1}" data-minute="${index}" required ${disabled}>
              ${this.temperatureFields.map((field) => this.temperatureInput(field, point.temperatures, index, `for setpoint ${index + 1}`, disabled)).join("")}
              <button class="ha-remove" type="button" aria-label="Remove setpoint ${index + 1}" data-remove="${index}" ${disabled}>×</button>
            </div>`,
            )
            .join("")}
          <button type="button" class="ha-action" data-add ${disabled}>+ Add setpoint</button>
          ${this.pending ? '<p class="ha-note" role="status">Waiting for the Nest to confirm…</p>' : feedback}
          <div class="ha-actions">
            <button class="ha-action" type="button" data-cancel ${disabled}>Cancel</button>
            <button class="ha-action ha-save" type="submit" ${disabled}>Save</button>
          </div>
        </form>`;
    }
    const scheduled = points.filter((point) => point.time !== "00:00:00");
    return `
      <div class="ha-detail-title"><h3>${name}</h3>${this.canEdit() ? '<button type="button" class="ha-action" data-edit>Edit</button>' : ""}</div>
      ${starting}
      ${scheduled.map((point) => `<div class="ha-setpoint"><span>${timeLabel(point.time)}</span><strong>${degrees(point)}</strong></div>`).join("")}
      ${!scheduled.length ? `<p class="ha-note">${start ? "Holds all day" : "No setpoints saved"}</p>` : ""}
      ${feedback}`;
  }

  bindDay() {
    const panel = this.shadowRoot.querySelector(".ha-day");
    panel.querySelector("[data-edit]")?.addEventListener("click", () => {
      const points = this.dayPoints(this.day);
      const start = this.startingPoint(points);
      this.message = undefined;
      this.draft = {
        revision: this.editRevision,
        midnight: start ? { temperatures: this.temperatureDraft(start), explicit: !start.continuation } : undefined,
        points: points
          .filter((point) => point.time !== "00:00:00")
          .map((point) => ({
            hour: point.time.slice(0, 2),
            minute: point.time.slice(3, 5),
            savedTime: point.time,
            temperatures: this.temperatureDraft(point),
          })),
      };
      this.render();
      this.shadowRoot.querySelector(".ha-day input, [data-add]")?.focus();
    });
    if (!this.draft) return;
    panel.querySelector("[data-clear-midnight]")?.addEventListener("click", () => {
      this.captureDraft();
      this.draft.midnight = undefined;
      this.message = undefined;
      this.render();
      this.shadowRoot.querySelector("[data-add-midnight]")?.focus();
    });
    panel.querySelector("[data-add-midnight]")?.addEventListener("click", () => {
      this.captureDraft();
      const previous = this.dayPoints((this.day + DAYS.length - 1) % DAYS.length).at(-1);
      this.draft.midnight = {
        temperatures: this.temperatureDraft(previous ?? this.defaultTemperatures()),
        explicit: true,
      };
      this.message = undefined;
      this.render();
      this.shadowRoot.querySelector(".ha-midnight input")?.focus();
    });
    for (const button of panel.querySelectorAll("[data-remove]")) {
      button.addEventListener("click", () => {
        this.captureDraft();
        this.draft.points.splice(Number(button.dataset.remove), 1);
        this.message = undefined;
        this.render();
        this.shadowRoot.querySelector("[data-add]")?.focus();
      });
    }
    panel.querySelector("[data-add]").addEventListener("click", () => {
      this.captureDraft();
      const start = this.startingPoint(this.dayPoints(this.day)) ?? this.defaultTemperatures();
      this.draft.points.push({ hour: "", minute: "", temperatures: this.temperatureDraft(start) });
      this.message = undefined;
      this.render();
      const inputs = this.shadowRoot.querySelectorAll("[data-hour]");
      inputs[inputs.length - 1]?.focus();
    });
    panel.querySelector("[data-cancel]").addEventListener("click", () => {
      this.draft = undefined;
      this.message = undefined;
      this.render();
      this.shadowRoot.querySelector("[data-edit]")?.focus();
    });
    panel.querySelector("form").addEventListener("submit", (event) => {
      event.preventDefault();
      this.save();
    });
  }

  defaultTemperatures() {
    return this.mode === "heat_cool" ? { heat_c: 20, cool_c: 24 } : { temperature_c: 20 };
  }

  captureTemperatures(row, index) {
    for (const { key, field } of this.temperatureFields) {
      row[key].display = this.shadowRoot.querySelector(`[data-${field}="${index}"]`).value;
    }
  }

  captureDraft() {
    if (this.draft.midnight) this.captureTemperatures(this.draft.midnight.temperatures, "midnight");
    for (const [index, point] of this.draft.points.entries()) {
      point.hour = this.shadowRoot.querySelector(`[data-hour="${index}"]`).value;
      point.minute = this.shadowRoot.querySelector(`[data-minute="${index}"]`).value;
      this.captureTemperatures(point.temperatures, index);
    }
  }

  invalidSetpoint(selector, text) {
    this.message = { text, error: true };
    this.render();
    const input = this.shadowRoot.querySelector(selector);
    input.setAttribute("aria-invalid", "true");
    input.setAttribute("aria-describedby", "ha-schedule-feedback");
    input.focus();
  }

  clearPending() {
    clearTimeout(this.timer);
    this.pending = undefined;
  }

  readTemperatures(row, index, context) {
    const result = {};
    for (const { key, field, label } of this.temperatureFields) {
      const { original, display } = row[key];
      const entered = display.trim();
      // Preserve the stored precision when its displayed value is unchanged.
      const temperature = entered === displayNumber(original) ? original : Number(entered.replace(",", "."));
      if (!/^[+-]?(?:\d+(?:[.,]\d*)?|[.,]\d+)$/.test(entered) || !Number.isFinite(temperature)) {
        this.invalidSetpoint(`[data-${field}="${index}"]`, `Enter a temperature in ${label} ${context}.`);
        return undefined;
      }
      result[key] = temperature;
    }
    if (
      this.mode === "heat_cool" &&
      result.cool_c - result.heat_c < 1.5 &&
      (result.heat_c !== row.heat_c.original || result.cool_c !== row.cool_c.original)
    ) {
      this.invalidSetpoint(`[data-cool="${index}"]`, `Cooling must be at least 1.5 °C above heating ${context}.`);
      return undefined;
    }
    return result;
  }

  async save() {
    if (this.pending || !this.draft) return;
    this.captureDraft();
    if (!this.canEdit() || this.editRevision !== this.draft.revision) {
      this.message = {
        text: "This schedule changed or is unavailable. Reopen the day before saving.",
        error: true,
      };
      this.render();
      return;
    }
    const setpoints = [];
    const times = new Set();
    const midnight = this.draft.midnight;
    if (midnight) {
      const target = this.readTemperatures(midnight.temperatures, "midnight", "at midnight");
      if (!target) return;
      if (
        midnight.explicit ||
        this.temperatureFields.some(({ key }) => target[key] !== midnight.temperatures[key].original)
      ) {
        setpoints.push({ time: "00:00:00", ...target });
      }
    }
    for (const [index, point] of this.draft.points.entries()) {
      const hour = timePart(point.hour, 23);
      if (hour === undefined) {
        this.invalidSetpoint(`[data-hour="${index}"]`, `Setpoint ${index + 1} needs an hour from 0 to 23.`);
        return;
      }
      const minute = timePart(point.minute, 59);
      if (minute === undefined) {
        this.invalidSetpoint(`[data-minute="${index}"]`, `Setpoint ${index + 1} needs a minute from 0 to 59.`);
        return;
      }
      const clock = `${hour}:${minute}:`;
      const time = point.savedTime?.startsWith(clock) ? point.savedTime : `${clock}00`;
      if (time === "00:00:00") {
        this.invalidSetpoint(`[data-hour="${index}"]`, "Use Starts at midnight to set the 00:00 target.");
        return;
      }
      if (times.has(time)) {
        this.invalidSetpoint(`[data-hour="${index}"]`, `Setpoint ${index + 1} uses the same time as another setpoint.`);
        return;
      }
      times.add(time);
      const target = this.readTemperatures(point.temperatures, index, `for setpoint ${index + 1}`);
      if (!target) return;
      setpoints.push({ time, ...target });
    }
    const request = {
      request_id: globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random().toString(36).slice(2)}`,
      mode: this.mode,
      expires_at: Math.floor(Date.now() / 1000) + 30,
      day: DAYS[this.day],
      revision: this.draft.revision,
      setpoints,
    };
    this.pending = request.request_id;
    this.timer = setTimeout(() => {
      this.clearPending();
      this.message = { text: "Confirmation timed out. Check the Nest schedule before retrying.", error: true };
      this.render();
    }, SAVE_TIMEOUT);
    this.render();
    try {
      await this.homeAssistant.callService("mqtt", "publish", {
        topic: `${this.attributes.topic}/set/schedule`,
        payload: JSON.stringify(request),
        qos: 0,
        retain: false,
      });
    } catch {
      if (this.pending !== request.request_id) return;
      this.clearPending();
      this.message = { text: "Home Assistant could not send the edit.", error: true };
      this.render();
    }
  }
}

if (!customElements.get("nest-schedule-card")) {
  customElements.define("nest-schedule-card", NestScheduleCard);
  window.customCards ??= [];
  window.customCards.push({
    type: "nest-schedule-card",
    name: "Nest temperature schedule",
    description: "View and edit the weekly Heat, Cool and Heat • Cool schedules.",
  });
}
