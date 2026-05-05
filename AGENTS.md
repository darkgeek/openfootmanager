# OpenFoot Manager — AI Agent Guide

This document helps AI coding agents understand the project structure, build process, testing requirements, and data locations so they can work effectively.

## 1. Project Overview

OpenFoot Manager is a football management simulation game.

| Layer | Technology | Location |
|-------|-----------|----------|
| Backend | Rust (Tauri / Axum) | `src-tauri/` |
| Frontend | React + TypeScript + TailwindCSS + Vite | `src/` |
| Persistence | SQLite | `~/.local/share/openfootmanager/saves/` |

### Rust Workspace Crates (under `src-tauri/crates/`)

| Crate | Purpose |
|-------|---------|
| `engine` | Match simulation (instant + live), zone-based action resolution |
| `ofm_core` | Game systems: training, transfers, contracts, AI management, end-of-season, messages |
| `domain` | Pure data types: Player, Team, League, Message, NewsArticle |
| `db` | SQLite save/load repository |

## 2. Build Instructions

### Backend (Rust)

```bash
cd src-tauri

# Release build with web mode (required for start.sh)
cargo build --release --features web

# Development build
cargo build
```

### Frontend (TypeScript)

```bash
# Development server
npm run dev:web

# Production build
npm run build:web
```

### Full Stack (using start.sh)

```bash
./start.sh
```

> **Note:** Requires Node.js ≥18. Node.js v24 requires Vite ≥6.x.

## 3. Testing Requirements

Every code change MUST pass all tests before committing.

### Backend Tests (Rust)

```bash
cd src-tauri

# Run engine and game core tests
cargo test -p engine -p ofm_core

# Run all tests in workspace (some crates may have pre-existing failures)
cargo test --workspace
```

Key test files:
- `src-tauri/crates/engine/tests/simulation_tests.rs` — Instant match simulation
- `src-tauri/crates/engine/tests/live_match_tests.rs` — Live match simulation
- `src-tauri/crates/ofm_core/tests/turn_tests.rs` — Daily processing
- `src-tauri/crates/ofm_core/tests/end_of_season_tests.rs` — Season-end logic
- `src-tauri/crates/ofm_core/tests/training_tests.rs` — Training system

### Frontend Tests (TypeScript)

```bash
npx vitest run
```

### Pre-existing Failures

Some test failures exist before any changes and are unrelated to new modifications:
- `db` crate test compilation errors (missing struct fields)
- Some frontend tests with stale color class assertions
- These should NOT be introduced by new changes

## 4. Save Game Data

### Location

| OS | Path |
|----|------|
| Linux | `~/.local/share/openfootmanager/saves/` |
| macOS | `~/Library/Application Support/openfootmanager/saves/` |

### Format

SQLite `.db` files. Key tables:
- `players` — Player data including nationality, names, attributes
- `teams` — Team data including country, name
- `settings` — Game settings

### Migration

After changing data generation logic (e.g., name pools, nationality mappings), existing saves need migration. Write a migration script in `scripts/` and run it against the user's save file.

## 5. Code Navigation Guide

### Match Engine

| File | Purpose |
|------|---------|
| `crates/engine/src/engine/mod.rs` | Core instant simulation loop |
| `crates/engine/src/engine/resolution.rs` | Action resolution per zone (buildup, midfield, attacking third, shot) |
| `crates/engine/src/engine/fouls.rs` | Foul, card, penalty logic |
| `crates/engine/src/live_match/` | Live (step-by-step) match simulation |
| `crates/engine/src/live_match/zone_resolution.rs` | Live match action resolution |
| `crates/engine/src/live_match/penalty.rs` | Penalty shootout |
| `crates/engine/src/live_match/helpers.rs` | Stamina depletion, condition-adjusted skills |
| `crates/engine/src/report.rs` | MatchReport, TeamStats, PlayerMatchStats, GoalDetail |
| `crates/engine/src/types.rs` | PlayerData, TeamData, MatchConfig, Zone, Side, Position |
| `crates/engine/src/event.rs` | MatchEvent, EventType (22 event variants) |

### Game Systems (`ofm_core`)

| File | Purpose |
|------|---------|
| `src/training.rs` | Daily training, attribute growth, fitness recovery, AI training management |
| `src/end_of_season.rs` | Season-end processing: growth, retirements, contract expiry, AI replenishment |
| `src/turn/mod.rs` | `process_day()` — main daily loop orchestrating all systems |
| `src/turn/post_match.rs` | Post-match: stamina depletion, stats update, morale, form |
| `src/contracts.rs` | Contract expiry, renewals, wage negotiation |
| `src/ai_team_management.rs` | AI squad replenishment, youth signing, transfer activity |
| `src/messages.rs` | Inbox message generation and cleanup |
| `src/messages/match_messages.rs` | Match result and preview messages |
| `src/training_report.rs` | Monthly training attribute change report |
| `src/generator/` | World generation: teams, players, staff, names |
| `src/generator/generation.rs` | `country_to_iso()`, `pick_name_from_def()`, `pick_nationality_from_def()` |
| `src/generator/data.rs` | Hardcoded nationality name pools (`NATIONALITY_POOLS`) and team templates |

### Domain Types (`domain`)

| File | Purpose |
|------|---------|
| `src/player.rs` | `Player`, `PlayerAttributes`, `Position`, traits |
| `src/team.rs` | `Team`, `TrainingFocus`, `TrainingIntensity`, `TrainingSchedule` |
| `src/message.rs` | `InboxMessage`, `MessageCategory`, `MessagePriority` |
| `src/news.rs` | `NewsArticle`, `NewsCategory` |
| `src/league.rs` | `League`, `Fixture`, `StandingEntry`, `MatchResult` |

### Key Configuration Constants

| Constant | Location | Default | Description |
|----------|----------|---------|-------------|
| `MIN_GOALKEEPERS` | `ai_team_management.rs:17` | 3 | Minimum GK per AI team |
| `MIN_OUTFIELD_PLAYERS` | `ai_team_management.rs:18` | 20 | Minimum outfield per AI team |
| `MatchConfig` | `engine/src/types.rs` | — | Tuneable simulation parameters |
