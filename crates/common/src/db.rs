use crate::types::{Block, BlockData, Contract, Log, TokenTransfer, Transaction};

use alloy_primitives::{Address, B256, Bytes, FixedBytes, U256};
use anyhow::Result;
use sqlx::{
    PgExecutor, {Error, query},
};
use std::collections::VecDeque;

pub async fn fetch_addresses_txs<'a, E>(
    executor: E,
    addr: Address,
    limit: i64,
    offset: i64,
) -> Result<Vec<Transaction>, sqlx::Error>
where
    E: PgExecutor<'a>,
{
    let rows = query!(
        r#"
            SELECT hash, block_number, tx_index, from_addr, to_addr, value,
                   gas_price, gas_limit, gas_used, input, nonce, status, created_at
            FROM transactions
            WHERE from_addr = $1 OR to_addr = $1
            ORDER BY block_number DESC
            LIMIT $2 OFFSET $3
            "#,
        addr.as_slice(),
        limit,
        offset,
    )
    .fetch_all(executor)
    .await?;

    let txs = rows
        .into_iter()
        .map(|r| Transaction {
            hash: B256::from_slice(&r.hash),
            block_number: r.block_number,
            tx_index: r.tx_index,
            from_addr: Address::from_slice(&r.from_addr),
            to_addr: r
                .to_addr
                .map(|a| Address::from_slice(&a)),
            value: U256::from_be_slice(&r.value),
            gas_price: r.gas_price,
            gas_limit: r.gas_limit,
            gas_used: r.gas_used,
            input: r.input.map(Bytes::from),
            nonce: r.nonce,
            status: r.status,
            created_at: r.created_at.expect("unreachable"),
        })
        .collect();

    Ok(txs)
}

pub async fn fetch_block_by_number<'a, E>(
    executor: E,
    block_number: u64,
) -> Result<Block, sqlx::Error>
where
    E: PgExecutor<'a>,
{
    let record = query!(
        r#"
        SELECT * FROM blocks WHERE number = $1;

    "#,
        block_number as i64
    )
    .fetch_one(executor)
    .await?;
    let block = Block {
        number: record.number as u64,
        hash: FixedBytes::from_slice(&record.hash),
        parent_hash: FixedBytes::from_slice(&record.parent_hash),
        gas_used: record.gas_used as u64,
        gas_limit: record.gas_limit as u64,
        miner: Address::from_slice(&record.miner),
        tx_count: record.tx_count as u32,
        timestamp: record.timestamp,
        base_fee: record.base_fee,
        created_at: record
            .created_at
            .expect("this should never happen"),
        size: record.size as u32,
    };

    Ok(block)
}

pub async fn fetch_tx_by_hash<'a, E>(executor: E, hash: B256) -> Result<Transaction, sqlx::Error>
where
    E: PgExecutor<'a>,
{
    let record = query!(
        r#"
        SELECT * FROM transactions WHERE hash = $1;

    "#,
        hash.as_slice()
    )
    .fetch_one(executor)
    .await?;
    let transaction = Transaction {
        hash: FixedBytes::from_slice(&record.hash),
        block_number: record.block_number,
        tx_index: record.tx_index,
        from_addr: Address::from_slice(&record.from_addr),
        to_addr: record
            .to_addr
            .map(|v| Address::from_slice(&v)),
        value: U256::from_be_slice(&record.value),
        gas_used: record.gas_used,
        gas_price: record.gas_price,
        gas_limit: record.gas_limit,
        input: record.input.map(Bytes::from),
        nonce: record.nonce,
        status: record.status,
        created_at: record.created_at.expect("unreachable"),
    };
    Ok(transaction)
}

pub async fn fetch_block_history<'a, E>(executor: E) -> Result<VecDeque<BlockData>>
where
    E: PgExecutor<'a>,
{
    let record = query!(
        r#"
        SELECT number, hash, parent_hash FROM blocks ORDER BY number DESC LIMIT 128;
        "#
    )
    .fetch_all(executor)
    .await?;
    let result = VecDeque::from_iter(record.iter().map(|row| BlockData {
        number: row.number as u64,
        hash: B256::from_slice(&row.hash),
        parent_hash: B256::from_slice(&row.parent_hash),
    }));
    Ok(result)
}

pub async fn fetch_block_history_until<'a, E>(
    executor: E,
    until: u64,
) -> Result<VecDeque<BlockData>>
where
    E: PgExecutor<'a>,
{
    let record = query!(
        r#"
        SELECT number, hash, parent_hash FROM blocks WHERE number < $1 ORDER BY number DESC LIMIT 128;
        "#
    ,until as i64
    )
    .fetch_all(executor)
    .await?;
    let result = VecDeque::from_iter(record.iter().map(|row| BlockData {
        number: row.number as u64,
        hash: B256::from_slice(&row.hash),
        parent_hash: B256::from_slice(&row.parent_hash),
    }));
    Ok(result)
}

pub async fn fetch_latest_block_number<'a, E>(executor: E) -> Result<u64>
where
    E: PgExecutor<'a>,
{
    let record = query!(
        r#"
        SELECT number FROM blocks ORDER BY number DESC LIMIT 1;
    "#
    )
    .fetch_one(executor)
    .await?;
    Ok(record.number as u64)
}

pub async fn drop_blocks_data_until<'a, E>(executor: E, block_number: u64) -> Result<(), Error>
where
    E: PgExecutor<'a>,
{
    query!(
        r#"
            WITH del_tx AS (
                DELETE FROM transactions
                WHERE block_number > $1
                RETURNING hash
            ),
            del_contracts AS (
                DELETE FROM contracts
                WHERE creation_tx IN (SELECT hash FROM del_tx)
            ),
            del_logs AS (
                DELETE FROM logs
                WHERE block_number > $1
            ),
            del_transfers AS (
                DELETE FROM token_transfers
                WHERE block_number > $1
            )
            DELETE FROM blocks
            WHERE number > $1
            "#,
        block_number as i64
    )
    .execute(executor)
    .await?;

    Ok(())
}

pub async fn insert_block<'a, E>(executor: E, block: Block) -> Result<(), Error>
where
    E: PgExecutor<'a>,
{
    query!(
        r#"
    INSERT INTO blocks (number, hash, parent_hash, timestamp, miner, gas_used, gas_limit, base_fee, tx_count, size)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
    "#,
        block.number as i64,
        block.hash.as_slice(),
        block.parent_hash.as_slice(),
        block.timestamp,
        block.miner.as_slice(),
        block.gas_used as i64,
        block.gas_limit as i64,
        block.base_fee,
        block.tx_count as i64,
        block.size as i64,
        )
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn insert_log<'a, E>(executor: E, log: Log) -> Result<(), Error>
where
    E: PgExecutor<'a>,
{
    query!(
          r#"
      INSERT INTO logs (block_number, tx_hash, log_index, address, topic0, topic1, topic2, topic3, data)
      VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
      "#,
          log.block_number,
          log.tx_hash.as_ref().map(|h| h.as_slice()),
          log.log_index,
          log.address.as_slice(),
          log.topic0.as_ref().map(|t| t.as_slice()),
          log.topic1.as_ref().map(|t| t.as_slice()),
          log.topic2.as_ref().map(|t| t.as_slice()),
          log.topic3.as_ref().map(|t| t.as_slice()),
          log.data.as_ref().map(|d| d.as_ref()),
      )
      .execute(executor)
      .await?;
    Ok(())
}

pub async fn insert_token_transfer<'a, E>(executor: E, transfer: TokenTransfer) -> Result<(), Error>
where
    E: PgExecutor<'a>,
{
    query!(
          r#"
      INSERT INTO token_transfers (block_number, tx_hash, log_index, token_address, from_addr, to_addr, value,
  token_id, token_type)
      VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
      "#,
          transfer.block_number,
          transfer.tx_hash.as_ref().map(|h| h.as_slice()),
          transfer.log_index,
          transfer.token_address.as_slice(),
          transfer.from_addr.as_slice(),
          transfer.to_addr.as_slice(),
          &transfer.value.to_be_bytes::<32>(),
          &transfer.token_id.to_be_bytes::<32>(),
          transfer.token_type as i16,
      )
      .execute(executor)
      .await?;
    Ok(())
}

pub async fn insert_contract<'a, E>(executor: E, contract: Contract) -> Result<(), Error>
where
    E: PgExecutor<'a>,
{
    query!(
          r#"
      INSERT INTO contracts (address, creator, creation_tx, bytecode, is_verified, name, source_code, abi,
  compiler, optimization)
      VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
      "#,
          contract.address.as_slice(),
          contract.creator.as_slice(),
          contract.creation_tx.as_ref().map(|h| h.as_slice()),
          contract.bytecode.as_ref().map(|b| b.as_ref()),
          contract.is_verified,
          contract.name.as_deref(),
          contract.source_code.as_deref(),
          contract.abi,
          contract.compiler.as_deref(),
          contract.optimization,
      )
      .execute(executor)
      .await?;
    Ok(())
}

pub async fn insert_tx<'a, E>(executor: E, tx: Transaction) -> Result<(), Error>
where
    E: PgExecutor<'a>,
{
    query!(
        r#"
    INSERT INTO transactions (hash, block_number, tx_index, from_addr, to_addr, value, gas_price, gas_limit, gas_used, input, nonce, status)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
    "#,
        tx.hash.as_slice(),
        tx.block_number,
        tx.tx_index,
        tx.from_addr.as_slice(),
        tx.to_addr
            .as_ref()
            .map(|a| a.as_slice()),
        &tx.value.to_be_bytes::<32>(),
        tx.gas_price,
        tx.gas_limit,
        tx.gas_used,
        tx.input.as_ref().map(|i| i.as_ref()),
        tx.nonce,
        tx.status,
        )
    .execute(executor)
    .await?;
    Ok(())
}
