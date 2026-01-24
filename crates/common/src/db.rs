use crate::types::{Block, Contract, Log, TokenTransfer, Transaction};
use alloy_primitives::{Address, B256};
use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::PgExecutor;
use sqlx::postgres::PgTransaction;
use sqlx::{Error, PgPool, query};

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

