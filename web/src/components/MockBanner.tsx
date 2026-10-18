// Visibly labels the prototype as mock so no one mistakes it for the real pipeline.
export function MockBanner({ scenario }: { scenario: string }) {
  return (
    <p className="mock-banner" role="note">
      <strong>Mock mode</strong> — responses are <em>simulated</em> and do not come
      from the Rust pipeline. Scenario: <code>{scenario}</code>. Real API wiring is
      UI-003/UI-004 and is not implemented.
    </p>
  );
}
