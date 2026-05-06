# Comprehensive Database Migration Plan

## Fork-Friendly Approach for ZeroClaw

---

## Philosophy

**Minimal Fork Impact:** Each migration step is a self-contained change that:

- Adds new functionality without modifying existing code paths
- Provides clear fallback behavior
- Can be rebased onto upstream without conflicts
- Is independently testable

---

## Migration Phases

### Phase 0: Foundation (Current State)

**Status:** ✅ Complete

- [x] Add `src/config/db.rs` - Database layer module
- [x] Add `src/config/mod.rs` exports
- [x] Schema: profiles, providers, agents, channels, config_store, config_history
- [x] CRUD methods for all entities
- [x] Build passes

**Fork Impact:** LOW - Pure addition, no upstream conflicts

---

### Phase 1: Optional Initialization (Priority: HIGH)

**Goal:** Initialize DB alongside existing config, no runtime changes

#### 1.1 Add DB initialization wrapper

Create `src/config/db_wrapper.rs`:

```rust
pub struct ConfigState {
    pub db: Option<ConfigDatabase>,  // None = DB disabled
    pub data_dir: PathBuf,
}

impl ConfigState {
    pub fn new(data_dir: PathBuf) -> Self {
        let db = ConfigDatabase::new(&data_dir).ok();
        Self { db, data_dir }
    }

    pub fn is_enabled(&self) -> bool {
        self.db.is_some()
    }
}
```

#### 1.2 Add feature flag

In `Cargo.toml`:

```toml
[features]
default = []
db-config = ["dep:rusqlite"]
```

#### 1.3 Add CLI flag

In `src/main.rs`:

- Add `--use-db-config` flag to enable DB
- Default: disabled (uses config.toml)

**Fork Impact:** LOW - All additions, no existing code changes

---

### Phase 2: CRUD API Endpoints (Priority: HIGH)

**Goal:** Add REST API for DB entities, doesn't touch core runtime

#### 2.1 Create API module

New file: `src/gateway/api_db.rs`

Endpoints to add:

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/db/profiles` | List profiles |
| POST | `/api/db/profiles` | Create profile |
| GET | `/api/db/providers` | List providers |
| POST | `/api/db/providers` | Create provider |
| PUT | `/api/db/providers/:id` | Update provider |
| DELETE | `/api/db/providers/:id` | Delete provider |
| GET | `/api/db/channels` | List channels |
| POST | `/api/db/channels` | Create channel |
| PUT | `/api/db/channels/:id` | Update channel |
| DELETE | `/api/db/channels/:id` | Delete channel |
| GET | `/api/db/agents` | List agents |
| POST | `/api/db/agents` | Create agent |
| PUT | `/api/db/agents/:id` | Update agent |
| DELETE | `/api/db/agents/:id` | Delete agent |

#### 2.2 Wire to gateway

In `src/gateway/mod.rs`:

- Add conditional routes (only if DB enabled)
- Add `config_db: Option<Arc<ConfigDatabase>>` to AppState

#### 2.3 Frontend pages

New/updated web pages:

- `web/src/pages/Providers.tsx` - Provider management
- `web/src/pages/Channels.tsx` - Channel management
- `web/src/pages/Agents.tsx` - Agent management

**Fork Impact:** MEDIUM - New API routes, but non-breaking

---

### Phase 3: Runtime Integration - Providers (Priority: HIGH)

**Goal:** Read providers from DB at runtime, fallback to config.toml

#### 3.1 Create ProviderResolver trait

```rust
pub trait ProviderResolver {
    fn get_default_provider(&self) -> Result<ProviderConfig>;
    fn get_provider(&self, name: &str) -> Result<Option<ProviderConfig>>;
}

pub struct ConfigTomlProvider;
pub struct DatabaseProvider {
    db: Arc<ConfigDatabase>,
}

impl ProviderResolver for ConfigTomlProvider { /* from config.toml */ }
impl ProviderResolver for DatabaseProvider { /* from DB */ }
```

#### 3.2 Update gateway startup

In `src/gateway/mod.rs` `run_gateway()`:

```rust
let provider_resolver: Box<dyn ProviderResolver> = if config_state.db.is_some() {
    Box::new(DatabaseProvider { db: config_state.db.clone() })
} else {
    Box::new(ConfigTomlProvider { config: config.clone() })
};
```

#### 3.3 Update agent initialization

In `src/agent/mod.rs`:

- Accept `ProviderResolver` in agent builder
- Use resolver to get provider config

**Fork Impact:** MEDIUM - Refactors provider loading, but with fallback

---

### Phase 4: Runtime Integration - Channels (Priority: MEDIUM)

**Goal:** Load channels from DB, fallback to config.toml

#### 4.1 Create ChannelLoader trait

```rust
pub trait ChannelLoader {
    fn load_channels(&self) -> Result<Vec<Box<dyn Channel>>>;
}
```

#### 4.2 Update channel initialization

In `src/channels/mod.rs`:

- Add `load_channels_from_db()` method
- Add `load_channels_from_config()` method (existing)
- Choose based on DB availability

#### 4.3 Update gateway

In `src/gateway/mod.rs`:

- Pass DB to channel initialization
- Channels read from DB if available

**Fork Impact:** MEDIUM - Modifies channel loading logic

---

### Phase 5: Runtime Integration - Agents (Priority: MEDIUM)

**Goal:** Load agent configs from DB

#### 5.1 Update agent builder

In `src/agent/mod.rs`:

- Add `load_from_db()` method to AgentBuilder
- Read agent config from DB

#### 5.2 Update gateway

In `src/gateway/mod.rs`:

- Pass DB to agent initialization

**Fork Impact:** MEDIUM - Modifies agent loading

---

### Phase 6: Onboarding Integration (Priority: LOW)

**Goal:** Write initial config to DB during onboarding

#### 6.1 Update onboarding wizard

In `src/onboard/wizard.rs`:

- Add option to write provider/agent to DB
- Add DB initialization during first-run

**Fork Impact:** LOW - Adds new behavior, doesn't break existing

---

### Phase 7: Config Migration Tool (Priority: LOW)

**Goal:** One-time migration from config.toml to DB

#### 7.1 Create migration command

```bash
zeroclaw config migrate-to-db
```

Reads config.toml, writes to DB, outputs confirmation.

**Fork Impact:** NONE - Standalone command

---

## Fork Sync Strategy

### When Syncing with Upstream

1. **Phase 1-2 (API only):**
   - Rebase cleanly
   - May need to re-add routes to gateway

1. **Phase 3-5 (Runtime):**
   - May have conflicts in gateway/agent/channels
   - Keep our resolver pattern, adapt to upstream changes
   - Test fallback behavior

### Conflict Resolution Guidelines

| Upstream Change | Our Change | Resolution |
|----------------|------------|------------|
| New channel types | DB loading | Support both |
| Provider config changes | ProviderResolver | Update trait |
| Gateway refactor | DB init | Re-add conditionally |
| New config fields | DB schema | Add columns |

### Minimal Change Principle

Each phase adds files, minimal modifications to existing code:

- Phase 1: +1 file (`db_wrapper.rs`)
- Phase 2: +1 file (`api_db.rs`), +gateway routes
- Phase 3: +1 file (`provider_resolver.rs`)
- Phase 4: +1 file (`channel_loader.rs`)
- Phase 5: Modify agent builder

Total: ~5 new files, minimal existing modifications.

---

## Rollback Strategy

Each phase can back independently be rolled:

| Phase | Rollback |
|-------|----------|
| 1 | Remove `--use-db-config` flag |
| 2 | Disable DB routes with feature flag |
| 3 | Use ConfigTomlProvider fallback |
| 4 | Use config.toml channels |
| 5 | Use config.toml agents |

---

## Testing Strategy

### Unit Tests (per phase)

- Phase 1: Test DB initialization, migrations
- Phase 2: Test API endpoints with mock DB
- Phase 3: Test provider resolution (DB vs config)
- Phase 4: Test channel loading
- Phase 5: Test agent loading

### Integration Tests

- DB + config.toml both present → prefer DB
- DB present, config.toml missing → use DB
- DB missing → fallback to config.toml (existing behavior)

---

## File Inventory

### New Files to Create

```
src/config/
  db_wrapper.rs        # Phase 1 - Optional DB wrapper

src/gateway/
  api_db.rs            # Phase 2 - CRUD API

src/agent/
  provider_resolver.rs # Phase 3 - Provider resolution trait
  channel_loader.rs    # Phase 4 - Channel loading trait
```

### Modified Files

```
src/config/mod.rs     # Phase 1 - Export db_wrapper
src/gateway/mod.rs    # Phase 1,2,3 - DB init, routes, resolver
src/agent/mod.rs      # Phase 3,5 - Use resolver
src/channels/mod.rs   # Phase 4 - Use channel loader
src/main.rs           # Phase 1 - CLI flag
Cargo.toml            # Phase 1 - Feature flag
```

### Web Files (Phase 2)

```
web/src/pages/Providers.tsx   # NEW
web/src/pages/Channels.tsx    # NEW
web/src/pages/Agents.tsx       # NEW
```

---

## Timeline Recommendation

| Phase | Effort | Priority | Start |
|-------|--------|----------|-------|
| Phase 1 | 2-3 hours | HIGH | Week 1 |
| Phase 2 | 4-6 hours | HIGH | Week 1-2 |
| Phase 3 | 3-4 hours | HIGH | Week 2 |
| Phase 4 | 2-3 hours | MEDIUM | Week 2-3 |
| Phase 5 | 2-3 hours | MEDIUM | Week 3 |
| Phase 6 | 1-2 hours | LOW | Week 3 |
| Phase 7 | 2-3 hours | LOW | Week 4 |

Total: ~16-24 hours over 4 weeks.

---

## Success Criteria

1. ✅ Build passes at each phase
2. ✅ Existing config.toml behavior unchanged when DB disabled
3. ✅ DB features work when enabled
4. ✅ Clean rebase onto upstream main
5. ✅ All CRUD operations functional via API
6. ✅ Runtime can optionally use DB for config

---

## Current Blocker Assessment

**Previous Attempts Failed Because:**

1. Tried to replace config.toml entirely → massive conflicts
1. Modified too many files at once → impossible to debug
1. No fallback strategy → build failures = complete failure

**This Plan Fixes By:**

1. Optional, additive approach
1. One component at a time
1. Always maintain config.toml fallback
1. Testable at each phase
1. Fork-friendly minimal changes

---

## Implementation Status

**Branch:** `integration/vps-upstream-merge`

**Status:** ✅ ALL PHASES COMPLETE

### Completed Implementation

| Phase | Status | Commit | Description |
|-------|--------|--------|-------------|
| Phase 1 | ✅ | `d3d6f453` | Optional DB wrapper + CLI flag (`--use-db-config`) |
| Phase 2 | ✅ | `c5c1b8ad` | CRUD API endpoints (`/api/db/*`) |
| Phase 3 | ✅ | `e112223a` | ProviderResolver trait |
| Phase 4 | ✅ | `3c1fe456` | ChannelLoader trait |
| Phase 5 | ✅ | `c2786357` | Agent DB configuration |
| Phase 6 | ✅ | `87c87d67` | Onboarding → DB |
| Phase 7 | ✅ | `19583a90` | Migration command |

### New Files Created

- `src/config/db_wrapper.rs` - Optional DB state wrapper
- `src/agent/provider_resolver.rs` - Provider resolution trait
- `src/channels/loader.rs` - Channel loading trait
- `src/gateway/api_db.rs` - REST API endpoints

### Modified Files

- `src/config/mod.rs` - Exports
- `src/main.rs` - CLI flag + config-migrate command
- `src/onboard/wizard.rs` - DB writing during onboarding
- `src/gateway/mod.rs` - API module

### Usage

```bash
# Enable DB-backed config
zeroclaw --use-db-config daemon

# Migrate existing config.toml to DB
zeroclaw config-migrate

# DB API endpoints (when DB enabled)
curl http://localhost:8000/api/db/providers
curl -X POST http://localhost:8000/api/db/providers \
  -H "Content-Type: application/json" \
  -d '{"name": "openai", "api_key": "sk-..."}'
```
