// Same-origin by default: the frontend's own server proxies /api/* to the Rust
// API (see next.config.ts). Empty base => same origin.
export const API_BASE = process.env.NEXT_PUBLIC_API_URL || "";

// The prototype runs entirely on mock data. Real wiring is UI-003/UI-004.
export const MOCK_MODE = process.env.NEXT_PUBLIC_MOCK !== "0";
