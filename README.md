# Daily Team Huddle

Tauri desktop app for reviewing patient quality measures and medication adherence data.

## Stack

- **Tauri** — lightweight desktop shell (WebView2 on Windows)
- **React + TypeScript** — UI
- **Vite** — dev server & build
- **Framer Motion** — animations
- **TanStack Table** — data table
- **Axios** — API client (cookie-based auth)
- **Zustand** — state management

## Prerequisites

- **Node.js** 20+
- **Rust** — [rustup.rs](https://rustup.rs/). The project pins **Rust 1.89** via `src-tauri/rust-toolchain.toml` (run `rustup` once; it auto-selects the right version).
- **Windows:** WebView2 (included on Windows 10/11; Tauri installer can bundle it if missing)

## Getting Started

```bash
npm install
npm run dev
```

This starts the Vite dev server and opens the Tauri window.

### Browser-only dev

```bash
npm run dev:web
```

Open [http://localhost:5173](http://localhost:5173). API calls are proxied to `VITE_API_PROXY_TARGET` from your `.env` file.

### Why browser login can fail (but the desktop app works)

In the browser, the app runs on `http://localhost:5173` and the API is on another host. That is a **cross-origin** request unless you use the dev proxy.

**Dev fix:** set `VITE_API_PROXY_TARGET` in `.env` (same as your backend). `.env.development` enables the Vite proxy automatically.

**Production / packaged app:** uses `VITE_API_BASE_URL` from `.env` at build time — rebuild after changing it.

### Demo credentials (mock mode)

When `VITE_USE_MOCK=true` in `.env`:

| Step | Value |
|------|-------|
| Email | `demo@dailyhuddle.com` |
| Password | `password123` |
| Security answer | `roswell` |

## Environment

Copy `.env.example` to `.env` and set your backend URLs:

| Variable | Description |
|----------|-------------|
| `VITE_API_BASE_URL` | Backend API base URL (used in **production** / packaged builds) |
| `VITE_API_PROXY_TARGET` | Backend URL for the **Vite dev proxy** (`npm run dev`, `npm run dev:web`) |
| `VITE_USE_MOCK` | `true` to use mock auth/data |

Both API variables should normally point at the same server, e.g. `https://ehr.example.com`.

**Development:** `.env.development` sets `VITE_API_USE_PROXY=true` so the app calls `/daily-huddle/*` on the Vite server, which proxies to `VITE_API_PROXY_TARGET` from your `.env`.

**Production (`npm run build` / `npm run dist`):** the app calls `VITE_API_BASE_URL` directly. Rebuild after changing `.env`.

## API

Auth uses a clinic access code (`POST /daily-huddle/auth`). The token is stored in session storage and sent on every request as `Authorization: <token>` (raw value, no `Bearer` prefix).

### POST `/login/login`

Request: `{ email, password }`

| Response `status` | Meaning |
|-------------------|---------|
| `success` | Returns `userid`, `type`, `fname`, `lname`, `email`, `clinic`, `permissions`, `token` |
| `approve` | Account not approved (`message`: e.g. `"not approved"`) |
| `failed` | Login failed (`message`: e.g. `"failed"`) |

### POST `/login/security`

Called **immediately after successful login**, using `userid` from the login response.

Request: `{ userid }`

Success:

```json
{
  "data": {
    "id": 1,
    "question": "Your security question?"
  }
}
```

`id` is used as `qid` for verification.

### POST `/login/checksecurity`

Request: `{ userid, qid, answer }`

| Response `status` | Meaning |
|-------------------|---------|
| `success` | Returns `chosen_clinic`, `loginid` — proceed to dashboard |
| `failed` | Verification failed |

### POST `/login/logout`

Request: `{ userid, loginid }`

Clears the server session. `loginid` is taken from the security check success response.

### Other

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/patients` | Merged HEDIS + Med Adh patient rows |

## Scripts

| Command | Description |
|---------|-------------|
| `npm run dev` | Dev mode (Vite + Tauri window) |
| `npm run dev:web` | Browser-only dev server |
| `npm run build` | Build frontend to `dist/` |
| `npm run dist` | Package installer → `release-build/` |

## Installer size

| | Electron (before) | Tauri (now) |
|---|-------------------|-------------|
| Installer (`.exe`) | ~85 MB | **~2 MB** |
| App binary | ~299 MB unpacked | **~8 MB** |

After `npm run dist`, installers are copied to:

```
release-build/Daily Team Huddle Setup 0.1.0.exe
release-build/Daily Team Huddle 0.1.0.msi
```

Rust build artifacts live in `%USERPROFILE%\.cargo\daily-huddle-target\` (see `src-tauri/.cargo/config.toml`) to avoid Windows file-lock issues during rebuilds.
