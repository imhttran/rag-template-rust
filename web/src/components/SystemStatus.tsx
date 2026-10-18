"use client";

// Prototype status panel. Distinguishes "mock", "not checked", and "unavailable"
// rather than implying a live probe of the Rust API.
export type Availability = "mock" | "not-checked" | "up" | "down";

export function SystemStatus({
  api,
  database,
  model,
}: {
  api: Availability;
  database: Availability;
  model: Availability;
}) {
  const label: Record<Availability, string> = {
    mock: "mock",
    "not-checked": "not checked",
    up: "up",
    down: "down",
  };
  return (
    <section className="status-panel" aria-label="System status">
      <h2>System status</h2>
      <dl>
        <div><dt>API</dt><dd className={`avail ${api}`}>{label[api]}</dd></div>
        <div><dt>Database</dt><dd className={`avail ${database}`}>{label[database]}</dd></div>
        <div><dt>Model</dt><dd className={`avail ${model}`}>{label[model]}</dd></div>
      </dl>
      <p className="hint">Mock mode reports simulated availability; nothing is probed.</p>
    </section>
  );
}
