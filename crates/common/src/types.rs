use alloy::primitives::{Address, B256, Bytes, U256};
use alloy::rpc::types::Block as AlloyBlock;
use alloy::rpc::types::TransactionReceipt;
use sqlx::types::JsonValue;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Clone)]
pub struct BlockData {
    pub number: u64,
    pub hash: B256,
    pub parent_hash: B256,
}

#[derive(Clone)]
pub struct RawBlockData {
    pub raw_block: AlloyBlock,
    pub tx_receipts: Vec<TransactionReceipt>,
}

pub struct ProcessedBlock {
    pub block: Block,
    pub transactions: Vec<Transaction>,
    pub logs: Vec<Log>,
    pub token_transfers: Vec<TokenTransfer>,
    pub contracts: Vec<Contract>,
}

// ----------------------
// Blocks
// ----------------------
#[derive(Debug, FromRow, Clone, Serialize, Deserialize)]
pub struct Block {
    pub number: u64,               // BIGINT
    pub hash: B256,                // BYTEA (32 bytes)
    pub parent_hash: B256,         // BYTEA
    pub timestamp: DateTime<Utc>,  // TIMESTAMPTZ NOT NULL
    pub miner: Address,            // BYTEA (20 bytes)
    pub gas_used: u64,             // BIGINT
    pub gas_limit: u64,            // BIGINT
    pub base_fee: Option<i64>,     // BIGINT (Nullable)
    pub tx_count: u32,             // INT
    pub size: u32,                 // INT
    pub created_at: DateTime<Utc>, // TIMESTAMPTZ DEFAULT NOW()
}

impl From<alloy::rpc::types::Block> for Block {
    fn from(value: alloy::rpc::types::Block) -> Self {
        // Accessing the inner header
        let header = &value.header;

        Block {
            number: value.number(),
            hash: value.hash(),
            parent_hash: header.parent_hash,

            timestamp: DateTime::from_timestamp(header.timestamp as i64, 0)
                .unwrap_or_else(Utc::now),

            miner: header.beneficiary,

            gas_used: header.gas_used,
            gas_limit: header.gas_limit,

            base_fee: header
                .base_fee_per_gas
                .map(|f| f as i64),

            tx_count: value.transactions.len() as u32,

            size: value
                .header
                .size
                .map(|s| s.to::<u32>())
                .unwrap_or(0),

            created_at: Utc::now(),
        }
    }
}

// ----------------------
// Transactions
// ----------------------
#[derive(Debug, FromRow, Serialize, Deserialize)]
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

#[derive(Debug, FromRow)]
pub struct Contract {
    pub address: Address,                  //      BYTEA PRIMARY KEY,
    pub creator: Address,                  //      BYTEA NOT NULL,
    pub creation_tx: Option<B256>,         //      BYTEA REFERENCES transactions(hash),
    pub bytecode: Option<Bytes>,           //      BYTEA,
    pub is_verified: Option<bool>,         //      BOOLEAN DEFAULT FALSE,
    pub name: Option<String>,              //      TEXT,
    pub source_code: Option<String>,       //      TEXT,
    pub abi: Option<JsonValue>,            //      JSONB,
    pub compiler: Option<String>,          //      TEXT,
    pub optimization: Option<bool>,        //      BOOLEAN,
    pub created_at: Option<DateTime<Utc>>, //      TIMESTAMPTZ DEFAULT NOW() -- Changed
}
