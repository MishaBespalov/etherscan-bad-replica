# EtherScan-RS Project Plan

## Overview

A blockchain explorer clone written in Rust, demonstrating async patterns with Tokio, database interactions, Ethereum node communication, and multi-binary architecture. The system indexes Ethereum blockchain data and provides an API to query it.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                                                                             │
│  ┌─────────────────┐         ┌─────────────────┐                            │
│  │   Ethereum      │         │    PostgreSQL   │                            │
│  │   Node (RPC)    │         │    Database     │                            │
│  └────────┬────────┘         └────────┬────────┘                            │
│           │                           │                                     │
│           │ JSON-RPC                  │                                     │
│           │ (eth_getBlock,            │                                     │
│           │  eth_getLogs, etc)        │                                     │
│           │                           │                                     │
│           ▼                           │                                     │
│  ┌─────────────────────────────────┐  │                                     │
│  │            Sync                 │  │                                     │
│  │  ┌───────────┐ ┌─────────────┐  │  │                                     │
│  │  │  Block    │ │ Transaction │  │  │                                     │
│  │  │  Fetcher  │ │  Processor  │  │  │                                     │
│  │  │  (mpsc)   │ │  (mpsc)     │  │  │                                     │
│  │  └─────┬─────┘ └──────┬──────┘  │  │                                     │
│  │        │              │         │  │                                     │
│  │        └──────┬───────┘         │  │                                     │
│  │               ▼                 │  │                                     │
│  │        ┌─────────────┐          │  │                                     │
│  │        │   DB Writer │──────────┼──┘                                     │
│  │        │   (batch)   │          │                                        │
│  │        └─────────────┘          │                                        │
│  └─────────────────────────────────┘                                        │
│                                                                             │
│           ┌────────────────────────────┐                                    │
│           │                            │                                    │
│           ▼                            ▼                                    │
│  ┌─────────────────┐         ┌─────────────────┐                            │
│  │      API        │         │      Auth       │                            │
│  │    (axum)       │◄────────│    Service      │                            │
│  │                 │         │  (JWT/API keys) │                            │
│  └────────┬────────┘         └─────────────────┘                            │
│           │                                                                 │
│           ▼                                                                 │
│  ┌─────────────────┐         ┌─────────────────┐                            │
│  │   CLI Client    │         │ Contract Builder│                            │
│  │   (optional)    │         │   (optional)    │                            │
│  └─────────────────┘         └─────────────────┘                            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Binaries

### 1. sync
The blockchain indexer. Fetches data from Ethereum node and stores in PostgreSQL.

**Responsibilities:**
- Connect to Ethereum JSON-RPC endpoint
- Fetch blocks (historical backfill + real-time)
- Extract transactions, receipts, logs
- Decode known contract events
- Batch write to PostgreSQL
- Handle chain reorgs
- Track sync progress / resumability

**Key async patterns:**
- `mpsc` channels for processing pipeline (fetcher → processor → writer)
- `tokio::sync::Semaphore` for rate limiting RPC calls
- `tokio::select!` for multiplexing new blocks + backfill
- Connection pooling with `sqlx`
- Concurrent block fetching with bounded parallelism
- `broadcast` channel for reorg notifications

**Config:** `sync.toml`

**Ports:** None (outbound only)

---

### 2. api
HTTP API for querying indexed data.

**Responsibilities:**
- Query blocks, transactions, addresses
- Get token transfers and balances
- Contract verification status
- Transaction traces (if indexed)
- Pagination and filtering
- Rate limiting per API key

**Key async patterns:**
- Axum with shared database pool
- `tower` middleware for auth/rate-limiting
- Connection pooling
- Query timeouts via `tokio::time::timeout`

**Endpoints:**
```
GET  /api/v1/block/:number
GET  /api/v1/block/:hash
GET  /api/v1/tx/:hash
GET  /api/v1/address/:addr/txs
GET  /api/v1/address/:addr/tokens
GET  /api/v1/address/:addr/balance
GET  /api/v1/logs?address=&topic0=&fromBlock=&toBlock=
GET  /api/v1/contract/:addr
POST /api/v1/contract/:addr/verify
GET  /api/v1/stats
```

**Port:** 8080

---

### 3. auth
Authentication and API key management service.

**Responsibilities:**
- Issue and validate JWT tokens
- API key creation and management
- Rate limit tracking per key
- Usage analytics

**Key async patterns:**
- Axum HTTP server
- Redis/PostgreSQL for session storage
- `watch` channel for config reload
- Token validation middleware (shared with API)

**Endpoints:**
```
POST /auth/register
POST /auth/login
POST /auth/refresh
POST /auth/api-keys          # create new API key
GET  /auth/api-keys          # list user's keys
DELETE /auth/api-keys/:id
GET  /auth/usage             # API usage stats
```

**Port:** 8081

---

### 4. contract-builder (optional)
Solidity compilation and verification service.

**Responsibilities:**
- Accept source code + compiler settings
- Compile with solc
- Match bytecode against on-chain
- Store verified source + ABI
- Decode contract interactions using ABI

**Key async patterns:**
- `mpsc` job queue for compilation tasks
- `tokio::process` for spawning solc
- Timeouts for compilation

**Port:** 8082

---

### 5. cli (optional)
Command line interface for querying the API.

**Responsibilities:**
- Query blocks, transactions, addresses
- Format output (table, JSON)
- API key management
- Watch addresses for new transactions

**Commands:**
```
etherscan-cli block <number|hash>
etherscan-cli tx <hash>
etherscan-cli address <addr> [--txs|--tokens|--balance]
etherscan-cli logs --address <addr> --topic0 <topic>
etherscan-cli watch <addr>              # live stream
etherscan-cli auth login
etherscan-cli auth api-keys list|create|delete
```

---

## Data Model

### Core Tables

```sql
-- Blocks
CREATE TABLE blocks (
    number          BIGINT PRIMARY KEY,
    hash            BYTEA UNIQUE NOT NULL,
    parent_hash     BYTEA NOT NULL,
    timestamp       TIMESTAMPTZ NOT NULL,     -- Changed: Stores block time (UTC)
    miner           BYTEA NOT NULL,
    gas_used        BIGINT NOT NULL,
    gas_limit       BIGINT NOT NULL,
    base_fee        BIGINT,
    tx_count        INT NOT NULL,
    size            INT NOT NULL,
    created_at      TIMESTAMPTZ DEFAULT NOW() -- Changed: DB insertion time
);

-- Transactions
CREATE TABLE transactions (
    hash            BYTEA PRIMARY KEY,
    block_number    BIGINT REFERENCES blocks(number),
    tx_index        INT NOT NULL,
    from_addr       BYTEA NOT NULL,
    to_addr         BYTEA,
    value           BYTEA NOT NULL,
    gas_price       BIGINT NOT NULL,
    gas_limit       BIGINT NOT NULL,
    gas_used        BIGINT NOT NULL,
    input           BYTEA,
    nonce           BIGINT NOT NULL,
    status          SMALLINT,
    created_at      TIMESTAMPTZ DEFAULT NOW() -- Changed
);

-- Logs/Events
CREATE TABLE logs (
    id              BIGSERIAL PRIMARY KEY,
    block_number    BIGINT NOT NULL,
    tx_hash         BYTEA REFERENCES transactions(hash),
    log_index       INT NOT NULL,
    address         BYTEA NOT NULL,
    topic0          BYTEA,
    topic1          BYTEA,
    topic2          BYTEA,
    topic3          BYTEA,
    data            BYTEA,
    UNIQUE(tx_hash, log_index)
);

-- Token Transfers
CREATE TABLE token_transfers (
    id              BIGSERIAL PRIMARY KEY,
    block_number    BIGINT NOT NULL,
    tx_hash         BYTEA NOT NULL,
    log_index       INT NOT NULL,
    token_address   BYTEA NOT NULL,
    from_addr       BYTEA NOT NULL,
    to_addr         BYTEA NOT NULL,
    value           BYTEA NOT NULL,
    token_id        BYTEA NOT NULL,
    token_type      SMALLINT NOT NULL,
    UNIQUE(tx_hash, log_index)
);

-- Contracts
CREATE TABLE contracts (
    address         BYTEA PRIMARY KEY,
    creator         BYTEA NOT NULL,
    creation_tx     BYTEA REFERENCES transactions(hash),
    bytecode        BYTEA,
    is_verified     BOOLEAN DEFAULT FALSE,
    name            TEXT,
    source_code     TEXT,
    abi             JSONB,
    compiler        TEXT,
    optimization    BOOLEAN,
    created_at      TIMESTAMPTZ DEFAULT NOW() -- Changed
);

-- Sync State
CREATE TABLE sync_state (
    id              INT PRIMARY KEY DEFAULT 1,
    last_block      BIGINT NOT NULL,
    last_updated    TIMESTAMPTZ DEFAULT NOW() -- Changed
);
```

### Auth Tables

```sql
-- Users
CREATE TABLE users (
    id              UUID PRIMARY KEY,
    email           TEXT UNIQUE NOT NULL,
    password_hash   TEXT NOT NULL,
    created_at      TIMESTAMPTZ DEFAULT NOW() -- Changed
);

-- API Keys
CREATE TABLE api_keys (
    id              UUID PRIMARY KEY,
    user_id         UUID REFERENCES users(id),
    key_hash        TEXT UNIQUE NOT NULL,
    name            TEXT,
    rate_limit      INT DEFAULT 100,
    created_at      TIMESTAMPTZ DEFAULT NOW(), -- Changed
    last_used_at    TIMESTAMPTZ                -- Changed
);

-- Usage tracking
CREATE TABLE api_usage (
    id              BIGSERIAL PRIMARY KEY,
    api_key_id      UUID REFERENCES api_keys(id),
    endpoint        TEXT NOT NULL,
    timestamp       TIMESTAMPTZ DEFAULT NOW()  -- Changed
);
```

---

## Sync Pipeline Architecture

```
                                  Ethereum Node
                                       │
                                       │ JSON-RPC
                                       ▼
┌──────────────────────────────────────────────────────────────────┐
│                          Sync Binary                             │
│                                                                  │
│  ┌────────────────┐                                              │
│  │ Block Listener │ ◄─── newHeads subscription (WebSocket)       │
│  │ (real-time)    │                                              │
│  └───────┬────────┘                                              │
│          │                                                       │
│          │         ┌────────────────┐                            │
│          │         │ Backfill Task  │ ◄─── historical blocks     │
│          │         │ (catch-up)     │                            │
│          │         └───────┬────────┘                            │
│          │                 │                                     │
│          └────────┬────────┘                                     │
│                   │                                              │
│                   ▼                                              │
│          ┌────────────────┐      mpsc (bounded)                  │
│          │  Block Queue   │ ─────────────────────┐               │
│          │                │                      │               │
│          └────────────────┘                      ▼               │
│                                         ┌────────────────┐       │
│                                         │ Block Processor│       │
│          Semaphore ◄────────────────────│ (N workers)    │       │
│          (rate limit)                   │                │       │
│                                         └───────┬────────┘       │
│                                                 │                │
│                                                 │ mpsc           │
│                                                 ▼                │
│                                         ┌────────────────┐       │
│                                         │  Batch Writer  │       │
│                                         │  (debounced)   │       │
│                                         └───────┬────────┘       │
│                                                 │                │
└─────────────────────────────────────────────────┼────────────────┘
                                                  │
                                                  ▼
                                            PostgreSQL
```

**Channel types in Sync:**
- `mpsc` (bounded) — Block numbers to fetch → processors
- `mpsc` (bounded) — Processed data → batch writer  
- `broadcast` — Reorg notifications (all components listen)
- `watch` — Current chain head (for progress tracking)

---

## Project Structure

```
etherscan-rs/
├── Cargo.toml                    # workspace
├── README.md
├── CLAUDE.md
├── plan.md                       # this file
├── docs/
│   ├── architecture.md
│   ├── api.md                    # API documentation
│   └── database.md               # schema docs
├── migrations/                   # sqlx migrations
│   ├── 001_create_blocks.sql
│   ├── 002_create_transactions.sql
│   ├── 003_create_logs.sql
│   ├── 004_create_tokens.sql
│   ├── 005_create_contracts.sql
│   ├── 006_create_sync_state.sql
│   └── 007_create_auth_tables.sql
├── config/
│   ├── sync.example.toml
│   ├── api.example.toml
│   └── auth.example.toml
├── crates/
│   ├── common/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── types.rs          # Block, Transaction, Log, etc.
│   │       ├── eth.rs            # Ethereum primitives (Address, H256)
│   │       ├── db.rs             # shared DB types and queries
│   │       └── config.rs         # shared config types
│   ├── sync/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── rpc.rs            # Ethereum JSON-RPC client
│   │       ├── fetcher.rs        # block/tx fetching logic
│   │       ├── processor.rs      # decode logs, extract transfers
│   │       ├── writer.rs         # batch DB writes
│   │       ├── reorg.rs          # chain reorg handling
│   │       ├── backfill.rs       # historical sync
│   │       └── config.rs
│   ├── api/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── routes/
│   │       │   ├── mod.rs
│   │       │   ├── blocks.rs
│   │       │   ├── transactions.rs
│   │       │   ├── addresses.rs
│   │       │   ├── logs.rs
│   │       │   ├── contracts.rs
│   │       │   └── stats.rs
│   │       ├── middleware/
│   │       │   ├── mod.rs
│   │       │   ├── auth.rs
│   │       │   └── rate_limit.rs
│   │       ├── error.rs
│   │       └── config.rs
│   ├── auth/
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── routes.rs
│   │       ├── jwt.rs
│   │       ├── api_keys.rs
│   │       ├── password.rs       # argon2 hashing
│   │       └── config.rs
│   ├── contract-builder/         # optional
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── compiler.rs       # solc wrapper
│   │       ├── verifier.rs       # bytecode matching
│   │       └── config.rs
│   └── cli/                      # optional
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── commands/
│           │   ├── mod.rs
│           │   ├── block.rs
│           │   ├── tx.rs
│           │   ├── address.rs
│           │   └── auth.rs
│           └── client.rs         # API client
├── docker/
│   ├── Dockerfile
│   └── docker-compose.yml        # postgres + eth node + services
└── benches/
    └── sync_throughput.rs
```

---

## Tech Stack

| Purpose            | Crate                            |
|--------------------|----------------------------------|
| Async runtime      | `tokio`                          |
| HTTP API           | `axum`                           |
| HTTP client        | `reqwest`                        |
| Database           | `sqlx` (PostgreSQL)              |
| Ethereum types     | `alloy-primitives`               |
| Ethereum RPC       | `alloy-rpc-client` or `ethers`   |
| Serialization      | `serde` + `serde_json`           |
| CLI                | `clap`                           |
| Tracing            | `tracing` + `tracing-subscriber` |
| Password hashing   | `argon2`                         |
| JWT                | `jsonwebtoken`                   |
| Config             | `toml` + `serde`                 |
| Hex encoding       | `hex`                            |

---

## Build Order

### Phase 1: Foundation
- [ ] **Step 1** — Workspace setup with all crate stubs
- [ ] **Step 2** — Core types in common (Block, Transaction, Log, Address)
- [ ] **Step 3** — Database migrations, sqlx setup
- [ ] **Step 4** — Basic DB queries in common (insert/select block)

### Phase 2: Sync - Basic Pipeline
- [ ] **Step 5** — RPC client: connect to Ethereum node, fetch one block
- [ ] **Step 6** — Fetch block + transactions, print to stdout
- [ ] **Step 7** — Write single block to database
- [ ] **Step 8** — Sync loop: fetch blocks sequentially, track progress
- [ ] **Step 9** — Add mpsc channel: fetcher task → writer task

### Phase 3: Sync - Production Ready
- [ ] **Step 10** — Concurrent fetching with semaphore rate limiting
- [ ] **Step 11** — Batch writes (accumulate N blocks, write together)
- [ ] **Step 12** — Backfill mode: sync historical blocks
- [ ] **Step 13** — Real-time mode: subscribe to new blocks
- [ ] **Step 14** — Reorg handling: detect and rollback

### Phase 4: Sync - Data Enrichment
- [ ] **Step 15** — Decode logs: extract ERC20/721 transfers
- [ ] **Step 16** — Index token transfers table
- [ ] **Step 17** — Detect contract creations

### Phase 5: API - Basic
- [ ] **Step 18** — Axum server with health endpoint
- [ ] **Step 19** — GET /block/:number endpoint
- [ ] **Step 20** — GET /tx/:hash endpoint
- [ ] **Step 21** — GET /address/:addr/txs with pagination

### Phase 6: API - Full
- [ ] **Step 22** — GET /address/:addr/tokens
- [ ] **Step 23** — GET /logs with filters
- [ ] **Step 24** — GET /contract/:addr
- [ ] **Step 25** — GET /stats (total blocks, txs, etc.)

### Phase 7: Auth
- [ ] **Step 26** — Auth service: register/login endpoints
- [ ] **Step 27** — JWT token generation and validation
- [ ] **Step 28** — API key creation and management
- [ ] **Step 29** — Rate limiting middleware
- [ ] **Step 30** — Integrate auth with API service

### Phase 8: Polish
- [ ] **Step 31** — Graceful shutdown across all services
- [ ] **Step 32** — Structured logging with tracing
- [ ] **Step 33** — Error handling cleanup
- [ ] **Step 34** — Configuration validation

### Phase 9: Optional Features
- [ ] **Step 35** — CLI: basic query commands
- [ ] **Step 36** — CLI: watch command (live updates)
- [ ] **Step 37** — Contract builder: solc compilation
- [ ] **Step 38** — Contract builder: verification endpoint

### Phase 10: Documentation & Deployment
- [ ] **Step 39** — API documentation
- [ ] **Step 40** — Docker compose for full stack
- [ ] **Step 41** — README with setup instructions

---

## Checkpoints

After each checkpoint, the system should be demonstrably working:

1. **After Step 9:** Blocks flowing from node → sync → database
2. **After Step 14:** Sync handles real-time + backfill + reorgs
3. **After Step 17:** Full transaction data including token transfers indexed
4. **After Step 25:** API can query all indexed data
5. **After Step 30:** Auth working, API protected with rate limits
6. **After Step 34:** Production-quality error handling and observability

---

## Quality Goals

**Must have:**
- Backpressure handling (bounded channels)
- Graceful shutdown (finish in-flight writes)
- Resumable sync (track last block)
- Reorg handling (at least basic)
- Connection pooling for DB
- Rate limiting for RPC calls
- Structured logging

**Nice to have:**
- Benchmarks (blocks/sec sync rate)
- WebSocket subscriptions for live data
- Internal txs / traces
- ENS resolution
- Token metadata caching

**Out of scope (for v1):**
- Full EVM tracing
- State diffs
- Advanced analytics
- Frontend UI

---

## Environment Setup

**Required services:**
- PostgreSQL 15+
- Ethereum node with JSON-RPC (Geth, Erigon, or Infura/Alchemy)

**Environment variables:**
```bash
DATABASE_URL=postgres://user:pass@localhost/etherscan
ETH_RPC_URL=http://localhost:8545
# or
ETH_RPC_URL=https://mainnet.infura.io/v3/YOUR_KEY
```

---

## Current Progress

**Current step:** Not started

**Notes:**
(Add notes here as you progress)
