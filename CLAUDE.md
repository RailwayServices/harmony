# Harmony — Discord Bot Engineering Specification & Architecture

This document is the single source of truth for building and maintaining **Harmony**, a highly scalable, hot-reloadable Discord bot built with Rust. This document reflects the true microservices architecture of the bot and should be followed strictly by any human engineer or AI coding agent.

---

## 1. Project Identity

- **Project name:** `harmony`
- **Bot name:** Harmony
- **Primary language:** Rust (2024 edition, MSRV 1.96.1)
- **Architecture style:** Microservices (Cargo workspace)
- **Discord library:** `twilight` ecosystem
- **Inter-service Transport:** Redis Pub/Sub (`RedisTransport`)
- **Persistent store:** PostgreSQL
- **Hot/ephemeral store:** Redis

---

## 2. The Microservices Architecture

Harmony is completely decoupled to ensure **zero downtime updates (True Hot-Reloading)**. The bot is split into two primary applications that communicate strictly over Redis.

### A. `harmony-gateway-app` (The Shield)
- **Role:** Maintains the persistent WebSocket connection to Discord. 
- **Behavior:** It receives raw events from Discord, wraps them, and publishes them to the Redis channel `harmony_events_worker`. It also subscribes to `harmony_events_discord` to forward outgoing payloads (like Voice State Updates or HTTP interactions) back to Discord.
- **Why?** Because it contains no bot logic, it almost **never needs to be restarted**. Pushing code updates to the bot logic will not drop the Discord connection!

### B. `harmony-bot` (The Worker/Brain)
- **Role:** Handles all business logic, slash commands, and music streaming.
- **Behavior:** Listens to `harmony_events_worker` from Redis, routes commands to `harmony-commands`, and runs background events via `harmony-modules`. When it needs to respond to Discord, it publishes payloads to `harmony_events_discord`.
- **Scaling:** You can spin up multiple `harmony-bot` Docker containers simultaneously. Redis will distribute the load among them effortlessly.

---

## 3. The Music Engine & State Persistence

Harmony uses **Lavende**, a custom, rust-native, **in-process** audio player. There is NO external Java Lavalink server. `Lavende` handles voice connections and audio decoding (`symphonia`) directly within the Rust worker process.

### Hot-Reloading Music:
Because Lavende runs inside the worker process, restarting `harmony-bot` (to push code updates) severs the voice connection. To prevent interrupting the user experience:
1. **Auto-Save:** Every time the queue mutates (play, skip, pause, filter, volume change), the entire player state is serialized and saved to Redis (`harmony:player_state:{guild_id}`).
2. **Restoration:** When `harmony-bot` boots up, it reads all `harmony:player_state:*` keys from Redis, automatically tells the Gateway to reconnect to the saved `voice_channel_id`, and explicitly calls `player.play()` to seamlessly resume streaming the audio track right where it left off!

---

## 4. Workspace File Structure

```
harmony/
├── Cargo.toml
├── docker/
│   ├── Dockerfile
│   └── compose.prod.yml
├── .env
│
├── crates/
│   ├── common/         (Types, Error Handling, AppConfig)
│   ├── database/       (Postgres connection pool, sqlx migrations)
│   ├── cache/          (Redis client, rate-limiting)
│   ├── messaging/      (Redis Pub/Sub Transport Traits)
│   ├── gateway/        (Twilight Shard Manager, EventLoop logic)
│   ├── modules/        (Background loops, Lavende music engine, State Sync)
│   └── commands/       (Slash command routing, interaction handlers)
│
└── apps/
    ├── gateway/        (Runs harmony-gateway-app binary)
    └── bot/            (Runs harmony-bot binary)
```

**Note:** The `apps/api` dashboard service was removed. The ecosystem consists purely of the Gateway and the Bot Worker.

---

## 5. Non-Negotiable Engineering Rules

1. **Zero Comments:** Code must be self-explanatory. Use descriptive variable names and function decomposition instead of `//` or `/* */` inline comments.
2. **Clippy is God:** The codebase must ALWAYS pass `cargo fmt` and `cargo clippy --workspace --all-targets --all-features -- -D warnings`. 
3. **No Blocking in Async:** Never use `std::thread::sleep` or blocking I/O on the hot path. Use `tokio` equivalents.
4. **Event-Driven Only:** The `harmony-bot` worker must NEVER connect a Twilight Shard directly. All Discord events must come from Redis, and all Discord Gateway commands must go to Redis.
5. **No `unwrap()` / `expect()`:** Use `?` propagation with the centralized `HarmonyError` enum for all fallible operations.
6. **No AI Filler:** No placeholder code (`// TODO: add logic`). Only write production-ready code.

---

## 6. Deployment & Running

### Using Docker (Production/Recommended)
```bash
docker-compose -f docker/compose.prod.yml up --build -d
```
This boots PostgreSQL, Redis, `harmony-gateway`, and `harmony-bot`.

### Local Development (Manual)
Ensure PostgreSQL and Redis are running locally and `.env` is configured. Open two terminals:
1. `cargo run --bin harmony-gateway-app`
2. `cargo run --bin harmony-bot`
