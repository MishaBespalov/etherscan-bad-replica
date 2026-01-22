use alloy::primitives::{Address, B256, Bytes, U256};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ----------------------
// Blocks
// ----------------------
#[derive(Debug, FromRow)]
pub struct Block {
    pub number: i64,                       // BIGINT
    pub hash: B256,                        // BYTEA (32 bytes)
    pub parent_hash: B256,                 // BYTEA
    pub timestamp: i64,                    // TIMESTAMPTZ NOT NULL
    pub miner: Address,                    // BYTEA (20 bytes)
    pub gas_used: i64,                     // BIGINT
    pub gas_limit: i64,                    // BIGINT
    pub base_fee: Option<i64>,             // BIGINT (Nullable)
    pub tx_count: i32,                     // INT
    pub size: i32,                         // INT
    pub created_at: Option<NaiveDateTime>, // TIMESTAMPTZ DEFAULT NOW() -- Changed
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

    // Critical: Mapping NUMERIC to U256
    // The "postgres" feature flag handles the conversion from BigDecimal/Numeric
    pub value: U256,

    pub gas_price: i64,
    pub gas_limit: i64,
    pub gas_used: i64,
    pub input: Option<Bytes>, // BYTEA (Variable length)
    pub nonce: i64,
    pub status: Option<i16>, // SMALLINT
    pub created_at: Option<NaiveDateTime>,
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
