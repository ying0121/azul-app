# Daily Huddle

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

Open [http://localhost:5173](http://localhost:5173). API calls use the Vite proxy (see below).

### Why browser login can fail (but the desktop app works)

In the browser, the app runs on `http://localhost:5173` and the API is on `http://localhost:5001`. That is a **cross-origin** request. Browsers block it unless the API server allows **CORS**.

**Dev fix (already configured):** `.env.development` sets an empty `VITE_API_BASE_URL` so requests go through the **Vite proxy** (`/login/*` → `http://localhost:5001`). Use `npm run dev:web` and open `http://localhost:5173`.

**Production / packaged app:** uses `.env` with the full API URL (`http://localhost:5001`) — no browser CORS involved.

### Demo credentials (mock mode)

When `VITE_USE_MOCK=true` in `.env`:

| Step | Value |
|------|-------|
| Email | `demo@dailyhuddle.com` |
| Password | `password123` |
| Security answer | `roswell` |

## Environment

Copy `.env.example` to `.env`:

| Variable | Description |
|----------|-------------|
| `VITE_API_BASE_URL` | Backend API base URL |
| `VITE_USE_MOCK` | `true` to use mock auth/data |

## API Endpoints (production)

Set `VITE_USE_MOCK=false` and `VITE_API_BASE_URL` to your server (e.g. `http://localhost:5001`).

The app sends `withCredentials: true` for cookies and attaches the login token on every request as `Authorization: <token>` (raw token value, no `Bearer` prefix).

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
release-build/Daily Huddle Setup 0.1.0.exe
release-build/Daily Huddle 0.1.0.msi
```

Rust build artifacts live in `%USERPROFILE%\.cargo\daily-huddle-target\` (see `src-tauri/.cargo/config.toml`) to avoid Windows file-lock issues during rebuilds.
