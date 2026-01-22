use alloy::primitives::{Address, B256, Bytes, U256};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ----------------------
// Blocks
// ----------------------
#[derive(Debug, FromRow)]
pub struct Block {
    pub number: i64,               // BIGINT
    pub hash: B256,                // BYTEA (32 bytes)
    pub parent_hash: B256,         // BYTEA
    pub timestamp: DateTime<Utc>,  // TIMESTAMPTZ NOT NULL
    pub miner: Address,            // BYTEA (20 bytes)
    pub gas_used: i64,             // BIGINT
    pub gas_limit: i64,            // BIGINT
    pub base_fee: Option<i64>,     // BIGINT (Nullable)
    pub tx_count: i32,             // INT
    pub size: i32,                 // INT
    pub created_at: DateTime<Utc>, // TIMESTAMPTZ DEFAULT NOW()
}

// ----------------------
// Transactions
// ----------------------
#[derive(Debug, FromRow)]
pub struct Transaction {
    pub hash: B256, // BYTEA
    pub block_number: Option<i64>,
    pub tx_index: i32,
    pub from_addr: Address,       // BYTEA
    pub to_addr: Option<Address>, // BYTEA (Nullable for contract creation)
    pub value: U256,
    pub gas_price: i64,
    pub gas_limit: i64,
    pub gas_used: i64,
    pub input: Option<Bytes>, // BYTEA (Variable length)
    pub nonce: i64,
    pub status: Option<i16>,       // SMALLINT
    pub created_at: DateTime<Utc>, // TIMESTAMPTZ DEFAULT NOW()
}

// ----------------------
// Logs / Events
// ----------------------
#[derive(Debug, FromRow)]
pub struct Log {
    pub id: i64, // BIGSERIAL
    pub block_number: i64,
    pub tx_hash: Option<B256>, // BYTEA
    pub log_index: i32,
    pub address: Address, // BYTEA

    // Topics can be null, and are strictly 32 bytes
    pub topic0: Option<B256>,
    pub topic1: Option<B256>,
    pub topic2: Option<B256>,
    pub topic3: Option<B256>,

    pub data: Option<Bytes>, // BYTEA
}

#[derive(Debug, FromRow)]
pub struct TokenTransfer {
    pub id: i64,                // BIGSERIAL PRIMARY KEY,
    pub block_number: i64,      // BIGINT NOT NULL,
    pub tx_hash: Option<B256>,  // BYTEA NOT NULL,
    pub log_index: i32,         // INT NOT NULL,
    pub token_address: Address, // BYTEA NOT NULL,
    pub from_addr: Address,     // BYTEA NOT NULL,
    pub to_addr: Address,       // BYTEA NOT NULL,
    pub value: U256,            // BYTEA NOT NULL,
    pub token_id: U256,         // BYTEA NOT NULL,
    pub token_type: i32,        // SMALLINT NOT NULL,
}

#[derive(Debug, FromRow)]
pub struct SyncState {
    pub id: i64,                     // INT PRIMARY KEY DEFAULT 1,
    pub last_block: i64,             // BIGINT NOT NULL,
    pub last_updated: DateTime<Utc>, // TIMESTAMPTZ DEFAULT NOW() -- Changed
}
