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
    pub number: i64,           // BIGINT
    pub hash: B256,            // BYTEA (32 bytes)
    pub parent_hash: B256,     // BYTEA
    pub timestamp: i64,        // BIGINT
    pub miner: Address,        // BYTEA (20 bytes)
    pub gas_used: i64,         // BIGINT
    pub gas_limit: i64,        // BIGINT
    pub base_fee: Option<i64>, // BIGINT (Nullable)
    pub tx_count: i32,         // INT
    pub size: i32,             // INT
    pub created_at: Option<NaiveDateTime>,
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

// blocks (
//     number          BIGINT PRIMARY KEY,
//     hash            BYTEA UNIQUE NOT NULL,
//     parent_hash     BYTEA NOT NULL,
//     timestamp       BIGINT NOT NULL,
//     miner           BYTEA NOT NULL,
//     gas_used        BIGINT NOT NULL,
//     gas_limit       BIGINT NOT NULL,
//     base_fee        BIGINT,
//     tx_count        INT NOT NULL,
//     size            INT NOT NULL,
//     created_at      TIMESTAMPTZ DEFAULT NOW()
// )
//
#[derive(Serialize, Deserialize)]
struct Blocks {
    number: u64,
    hash: String,
    parent_hash: String,
    timestamp: u64,
    miner: String,
    gas_used: u64,
    gas_limit: u64,
    base_fee: u64,
    tx_count: u64,
    size: u64,
    created_at: u64,
}

struct Transactions {
    hash: u64,
    block_number: u64,
    tx_index: String,
    from_addr: Address,
    to_addr: String,
    value: u64,
    gas_price: u64,
    gas_limit: u64,
    gas_used: u64,
    input: String,
    nonce: String,
    status: String,
    created_at: u64,
}

// transactions (
//     hash            BYTEA PRIMARY KEY,
//     block_number    BIGINT REFERENCES blocks(number),
//     tx_index        INT NOT NULL,
//     from_addr       BYTEA NOT NULL,
//     to_addr         BYTEA,                    -- NULL for contract creation
//     value           NUMERIC(78, 0) NOT NULL,  -- wei
//     gas_price       BIGINT NOT NULL,
//     gas_limit       BIGINT NOT NULL,
//     gas_used        BIGINT NOT NULL,
//     input           BYTEA,
//     nonce           BIGINT NOT NULL,
//     status          SMALLINT,                 -- 0=fail, 1=success
//     created_at      TIMESTAMPTZ DEFAULT NOW()
// )
