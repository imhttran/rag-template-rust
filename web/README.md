# RAG Web UI (prototype) — UI-002

A Next.js App Router prototype of the RAG web UI. It runs on **mock data only**:
responses are simulated and do **not** come from the Rust pipeline (real wiring is
UI-003/UI-004). A visible "Mock mode" banner labels this on every screen.

Conventions are reused from the `inventory` reference (AppShell / PageHeader /
PageTitle / PageFooter / FormStatus / SystemStatus, `@/*` import paths, and the
same-origin `/api/*` proxy in `next.config.ts`).

## Supported Node and npm

Declared in `package.json` (`engines`); **tested with Node 24.21.0 and npm 11.19.0**:

| Tool | Supported |
| --- | --- |
| Node | `>=20.0.0 <25.0.0` |
| npm | `>=10.0.0 <12.0.0` |

Every web command validates both versions first. The toolchain is resolved, in
order, from **explicit overrides** (`NODE_BIN`, `NPM_BIN`), then **`PATH`**, then
**Zed's bundled Node** (any versioned install, no user name hard-coded). If the
tools are missing or out of range the command stops with setup instructions; it
never installs or upgrades anything.

## Run

```sh
cd web
npm install
npm run dev          # http://127.0.0.1:3000
npm run typecheck    # tsc --noEmit
npm run build        # next build
```

Both `dev` and `start` bind `127.0.0.1`. `API_URL` (server-side, default
`http://127.0.0.1:8080`) is used only by the server-side rewrite; it is never sent
to the browser.

## Makefile targets (run from the repository root)

```sh
make web-check-env   # resolve and validate the Node/npm toolchain
make web-install     # install web dependencies with that toolchain
make web-start       # start the prototype in the background on 127.0.0.1:3000
make web-status      # report state, URL, and log path
make web-stop        # stop the prototype started by make web-start
make web-typecheck   # tsc --noEmit
make web-build       # production build (into .next-build)
```

`make web-start` refuses to start a duplicate, and refuses to start when port 3000
is already in use by another service (it never stops that service). It finds
Node/npm on `PATH`, or the Zed-bundled installation, or an explicit override
(`NODE_BIN=... NPM_BIN=...`).

The PID file and log live under `web/.run/` (`web.pid`, `web.log`), which is
git-ignored. `make web-stop` stops only this project's server and its child
processes, and handles a stale PID file safely.

## Mock scenarios

Choose a scenario in the UI to demonstrate: `success`, `empty`,
`insufficient-evidence`, `connection-failure`, `invalid-file`, `incomplete-pdf`,
`loading`.
