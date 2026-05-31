# Nerevar

**Morrowind multiplayer, without the mod-list headaches.**

Nerevar is a desktop companion for [TES3MP](https://tes3mp.com/) that keeps everyone on the same mods, load order, and game data. Server owners can change a mod list and push updates to their group automatically. Players connect once and stay in sync. Nerevar handles the setup that used to require manual file copying, config editing, and hours of troubleshooting.

---

## Important disclaimer notice — read before hosting or sharing mods

> **Server owners and instance hosts are solely responsible for any mods, assets, or game data distributed through their Nerevar instances.**
>
> Nerevar provides tools to share files between a host and connected players. **It does not grant permission to redistribute third-party content.** Many mods are governed by terms on sites such as [Nexus Mods](https://www.nexusmods.com/) and by individual author licenses that restrict or prohibit redistribution, require permission, or impose other conditions.
>
> **Before you host a server, publish a manifest, or allow players to sync your mod list, you must:**
>
> - Confirm that you have the **permissive rights** to share every mod and asset in that list with every person who connects.
> - Comply with **Nexus Mods terms** if applicable, author permissions, and any other applicable licenses or contracts if applicable.
> - Understand that **violating those terms is your responsibility**, not Nerevar’s. Nerevar is distributed as-is and is offered without any warranty or support and takes no responsibility for any actions taken by users of the application.
>
> Nerevar is built to make cooperative play easier. It is **not** a workaround for mod redistribution rules. When in doubt, contact mod authors or use only content you are explicitly allowed to share.

---

<p align="center">
  <img src="src/assets/screenshot-dark-mode.png" alt="Nerevar dashboard in dark mode" width="700" />
</p>

<p align="center">
  <img src="src/assets/screenshot-light-mode.png" alt="Nerevar dashboard in light mode" width="700" />
</p>

---

## Table of contents

- [Overview](#overview)
- [Why Nerevar exists](#why-nerevar-exists)
- [How it works](#how-it-works)
- [For server owners](#for-server-owners)
- [For players joining a server](#for-players-joining-a-server)
- [First-time setup](#first-time-setup)
- [Dashboard and instances](#dashboard-and-instances)
- [Mod Organizer 2](#mod-organizer-2)
- [Settings](#settings)
- [Technical overview](#technical-overview)
- [Architecture](#architecture)
- [How sync works under the hood](#how-sync-works-under-the-hood)
- [Instance types](#instance-types)
- [Building from source](#building-from-source)
- [License](#license)

---

## Overview

Nerevar wraps the complexity of TES3MP mod management in a single application. You pick a data folder, point it at your Morrowind installation, and work from a dashboard instead of scattered config files and Discord links.

There are two roles, and you can do both on the same machine:

| Role             | What you do in Nerevar                                                                                                             |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| **Server owner** | Create an **owned instance**, build a mod list, host a manifest, run the TES3MP server, and share connection details with friends. |
| **Player**       | Create a **synced instance** (saved connection), pull the host’s mod list, and launch the client when you want to play.            |

When the host updates mods, players sync the changes on their next launch or manual sync. No one has to manually rebuild load orders or chase down missing files.

---

## Why Nerevar exists

Playing Morrowind with friends on TES3MP should not require everyone to be a power user. Before Nerevar, keeping a group aligned often meant:

- Sending mod lists and hoping everyone installed the right versions
- Manually updating every player when the host changed one plugin
- Debugging mismatched load orders and corrupt installs over voice chat

Nerevar was built so **non-technical players can join and play**, and **server owners can experiment with mod lists in multiplayer** without becoming full-time IT for their friend group. You focus on the game; Nerevar handles instance setup, OpenMW configuration, manifest hosting, file sync, validation, and launching TES3MP from one place.

---

## How it works

At a high level, every session follows the same idea:

1. **The host defines the mod list** — folders, plugins, and load order stored as a manifest.
2. **Nerevar shares that manifest and the files** — over a local sync server using the port you chose during setup.
3. **Players download only what changed** — Nerevar verifies checksums and load order before launch.
4. **Everyone launches through Nerevar** — client and server use matching data so TES3MP can connect cleanly.

You do not need to understand manifests, HTTP, or OpenMW cfg files to use Nerevar day to day. The app walks you through setup and exposes only what you need on the dashboard.

---

## For server owners

1. **Create an owned instance** from the dashboard (name, game port, optional password).
2. **Open the data manager** — scan mod folders, set load order, or import from Mod Organizer 2.
3. **Save and host your manifest** — this activates sync for that instance and publishes your list to connected players.
4. **Launch the TES3MP server** from the instance page.
5. **Share** your public IP, Nerevar sync port, TES3MP game port, and password (if set) with players.

> **Note:** Generating a manifest from the data manager will take a while depending on the size of your mod list, so be patient. Nerevar syncing works through checking the checksum of files so those checksums have to be generated. Nerevar will do this process in parallel and use all available CPU power but it can still take some time.

When you change mods, save and host again. Players pick up deltas on their next sync or launch.

You can also **launch the client from the same owned instance** to connect to your own server locally — Nerevar keeps client and server configs aligned.

---

## For players joining a server

1. **Create a new connection** (synced instance) with the host’s address, Nerevar sync port, and sync password if required.
2. **Wait for the initial sync** — first connect may take a while depending on mod list size.
3. **Launch the client** from the instance page — Nerevar checks for updates, syncs if needed, then starts TES3MP.
4. **Stay up to date** — use **Sync from host** or simply launch again after the host updates mods.

Connection details can be edited later from the instance settings page without recreating the connection.

---

## First-time setup

On first launch, Nerevar guides you through:

1. **Data directory** — where instances, TES3MP files, and synced mod data live (separate from the app itself).
2. **Morrowind installation** — path to your Data Files / `Morrowind.esm` for OpenMW scaffolding.
3. **Sync port** — default `25567`; used by Nerevar’s local sync server (not the TES3MP game port).
4. **In-app guide** — overview of hosting vs joining; read both paths before finishing setup.

The sync server starts **after** onboarding completes, on the port you selected.

---

## Dashboard and instances

The dashboard is the home screen after setup:

- **Owned instances** — full control: data manager, manifest hosting, server launch, client launch, connection settings, and delete options.
- **Synced instances** — saved connections to remote hosts: sync, validate, and launch client.
- **Settings** — Nerevar-wide options such as sync port.
- **MO2 plugin** — workflow for exporting a Mod Organizer 2 mod list into Nerevar.

Each instance keeps its own TES3MP copy, data directory, and OpenMW launch configuration so different servers never overwrite each other.

---

## Mod Organizer 2

If you use MO2, Nerevar includes an export plugin workflow (see the **MO2 Plugin** card on the dashboard). Export your mod list with directory paths, import the CSV in the instance data manager, and build your hosted manifest from there instead of hand-picking folders.

---

## Settings

From **Settings** you can change Nerevar’s sync port and related configuration. Changing the sync port restarts the local sync server so connected workflows use the new value.

---

## Technical overview

The sections below are for contributors, server operators who want deeper context, or anyone curious about how Nerevar is built.

**Stack**

| Layer         | Technology                                         |
| ------------- | -------------------------------------------------- |
| Desktop shell | [Tauri 2](https://v2.tauri.app/)                   |
| UI            | React 19, TypeScript, Vite, Tailwind CSS           |
| Backend       | Rust                                               |
| Multiplayer   | TES3MP (bundled per instance)                      |
| Engine config | OpenMW / OpenMW cfg generation and launch overlays |

**Repository:** [github.com/kyaustad/nerevar-rs](https://github.com/kyaustad/nerevar-rs)

---

## Architecture

Nerevar splits responsibilities between a React frontend and a Rust backend invoked through Tauri commands.

**Frontend (`src/`)**

- Dashboard, onboarding, instance detail pages, data manager, settings, MO2 plugin UI
- Config state driven by `on_config_change` events from the backend
- Process status and sync progress via Tauri event listeners

**Backend (`src-tauri/`)**

| Module                | Role                                                                    |
| --------------------- | ----------------------------------------------------------------------- |
| `nerevar_server`      | Local HTTP sync server (manifest + file routes)                         |
| `sync_host`           | Active hosting instance and manifest publication                        |
| `sync_client`         | Remote fetch, incremental download, validation, cancel                  |
| `instance_data`       | Manifest build/load, load order, MO2 CSV import, OpenMW resolution      |
| `instance_setup`      | TES3MP server/client default cfg, instance layout                       |
| `process_manager`     | Launch/stop TES3MP client and server, stdout/stderr streaming           |
| `openmw_ini_importer` | Global OpenMW scaffold and per-instance launch cfg                      |
| `port_conflict`       | Detect processes blocking sync or game ports                            |
| `config`              | Persistent `config.json` (instances, paths, sync port, onboarding flag) |

User data lives under a configurable **root path** (chosen in onboarding). Each instance has a root folder, a `tes3mp` subdirectory, and a dedicated **data directory** for mod packages and manifests.

---

## How sync works under the hood

1. **Manifest** — JSON describing packages (mod folders), file paths, SHA-256 hashes, plugin load order, TES3MP server port, and optional password metadata.
2. **Hosting** — When the host uses **Save & host manifest**, Nerevar writes the manifest and registers the instance as the active sync host. The local sync server serves manifest and file endpoints to authenticated clients.
3. **Client sync** — Synced instances call the remote host’s Nerevar sync port (with optional `X-Nerevar-Sync-Password`). The client compares remote manifest hash and `last_synced_at`; if outdated, it downloads only changed or missing files.
4. **Validation** — Before launch, Nerevar verifies on-disk files against the manifest and resolves OpenMW load order from the synced data.
5. **Launch** — OpenMW launch cfg is written to an instance overlay; TES3MP client cfg is updated with host, game port, and password. For owned instances connecting locally, client cfg targets `127.0.0.1` with settings read from the instance server cfg.

Sync is incremental: hosts republish after changes; clients pull deltas rather than full redownloads when possible.

**Ports (typical)**

| Port                                          | Purpose                              |
| --------------------------------------------- | ------------------------------------ |
| Nerevar sync port (default `25567`)           | HTTP sync server on the host machine |
| TES3MP game port (per instance, e.g. `25565`) | Actual multiplayer game connection   |

These are different ports serving different roles.

---

## Instance types

**Owned instance**

- Created locally with full server configuration
- Host manifest, run TES3MP server, manage mods via data manager
- Can launch client against the same instance (local server owner workflow)
- Stored in config as `ownedInstances`

**Synced instance**

- Created by connecting to a remote Nerevar host
- Stores remote host, sync port, sync password, and local copy of synced files
- Sync and launch client; no server hosting from this instance type
- Stored in config as `syncedInstances`

Both types use isolated directories under the user’s Nerevar data root so configs and mod files never collide.

---

## Building from source

**Prerequisites**

- [Node.js](https://nodejs.org/) (LTS recommended)
- [pnpm](https://pnpm.io/)
- [Rust](https://rustup.rs/) (stable toolchain)
- Platform dependencies for [Tauri 2](https://v2.tauri.app/start/prerequisites/)

**Development**

```bash
git clone https://github.com/kyaustad/nerevar-rs.git
cd nerevar-rs
pnpm install
pnpm tauri dev
```

**Production build**

```bash
pnpm tauri build
```

Installers and binaries are produced under `src-tauri/target/release/bundle/` (exact artifacts depend on OS and Tauri bundle targets).

**Useful scripts**

| Command            | Description                     |
| ------------------ | ------------------------------- |
| `pnpm dev`         | Vite frontend only              |
| `pnpm build`       | Typecheck and build frontend    |
| `pnpm tauri dev`   | Full desktop app in development |
| `pnpm tauri build` | Release build                   |
| `pnpm typecheck`   | TypeScript check                |

---

## License

Nerevar is licensed under the **GNU General Public License v3.0**. See [LICENSE](LICENSE) for the full text.

TES3MP, OpenMW, and Morrowind are separate projects with their own licenses and terms. Nerevar integrates with them but is not affiliated with or endorsed by their respective authors or rights holders.

---

_Nerevar is named for the figure at the center of Morrowind’s prophecy — the one who unites the Chimer and whose reincarnation brings the Ashlander tribes together against Dagoth Ur. This project tries to do something smaller but in the same spirit: bring a scattered group onto the same path so you can walk it together._
