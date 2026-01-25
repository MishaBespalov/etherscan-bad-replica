use alloy::consensus::BlockHeader;
use alloy::eips::{BlockId, BlockNumberOrTag};
use alloy::providers::Provider;
use alloy::rpc::types::Block as AlloyBlock;
use alloy::rpc::types::TransactionReceipt;
use anyhow::{Context, Result, anyhow, bail};
use common::db::fetch_latest_block_number as pg_fetch_latest_block_number;
use common::types::{Block, RawBlockData};
use derive_builder::Builder;
use futures::StreamExt;
use futures::stream;
use sqlx::PgPool;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::Semaphore;
use tokio::sync::mpsc::Sender;

#[derive(Clone, Debug)]
pub struct BlockTip {
    humber: u64,
    hash: B256,
    parent_hash: B256,
}
pub enum ChainEvent {
    NewBlock(u64),
    Reorg(u64),
}

#[derive(Builder)]
pub struct Fetcher<P>
where
    P: Provider + Clone + Send + Sync + 'static,
{
    provider: P,
    sender: Sender<RawBlockData>,
    #[builder(default = "Arc::new(Semaphore::new(10))")]
    semaphore: Arc<Semaphore>,
    LocalTip: VecDeque<BlockTip>,
}

impl<P> Fetcher<P>
where
    P: Provider + Clone + Send + Sync + 'static,
{
    pub async fn next_event(&mut self, local_tip: LocalTip, db_pool: PgPool) -> Result<ChainEvent> {
        let next_height = local_tip.height + 1;
        let next_remote_parent_hash = self
            .fetch_remote_block_parent_hash(next_height)
            .await?;

        if next_remote_parent_hash == local_tip.hash {
            return Ok(ChainEvent::NewBlock(next_height));
        }

        let new_head_hash = self
            .fetch_remote_block_hash(next_height)
            .await?;
        let mut check_height = local_tip.height;

        while check_height > 0 {
            let remote_parent_hash = self
                .fetch_remote_block_parent_hash(check_height)
                .await?;

            let local_prev_hash = pg_fetch_block_hash(&db_pool, check_height - 1).await?;

            if remote_parent_hash == local_prev_hash {
                let common_ancestor_height = check_height - 1;

                return Ok(ChainEvent::ForkDetected {
                    stale_head: local_tip.hash, // You can pass 'local_tip' here if needed
                    new_head: new_head_hash,
                    common_ancestor: common_ancestor_height,
                });
            }

            check_height -= 1;
        }

        Err(anyhow!("Critical error no common ancestor found!"))
    }

    pub async fn run(&self, pg_pool: PgPool) -> Result<()> {
        self.subscribe(pg_pool).await?;
        Ok(())
    }
    pub async fn subscribe(&self, pg_pool: PgPool) -> Result<()> {
        let sub = self.provider.subscribe_blocks().await?;
        let mut stream = sub.into_stream();
        let first_block_number = stream
            .next()
            .await
            .context("failed to get the block")?
            .number;
        let pg_latest_block_number = pg_fetch_latest_block_number(&pg_pool).await?;
        if first_block_number > pg_latest_block_number {
            self.backfill(pg_latest_block_number, first_block_number - 1)
                .await?;

            self.send_block_by_number(first_block_number)
                .await?;
        }
        let mut stream = stream
            .map(|block| async move {
                self.send_block_by_number(block.number)
                    .await
            })
            .buffered(20);

        while let Some(result) = stream.next().await {
            if let Err(e) = result {
                bail!("Error processing block: {}", e);
            }
        }
        Ok(())
    }
    pub async fn backfill(&self, start: u64, end: u64) -> Result<()> {
        let range = (start + 1)..=end;

        let mut stream = stream::iter(range)
            .map(|block_number| async move {
                self.send_block_by_number(block_number)
                    .await
            })
            .buffered(20);

        while let Some(result) = stream.next().await {
            if let Err(e) = result {
                bail!("Error processing block: {}", e);
            }
        }
        Ok(())
    }
    pub async fn send_block_by_number(&self, block_number: u64) -> Result<()> {
        let block = self
            .fetch_alloy_block(block_number)
            .await?;
        let tx_receipts = self
            .fetch_block_receipts(block_number)
            .await?;
        let raw_block_data = RawBlockData {
            raw_block: block.clone(),
            tx_receipts,
        };
        if let Err(e) = self.sender.send(raw_block_data).await {
            bail!("Error while sending: {}", e);
        }
        Ok(())
    }

    pub async fn fetch_block_receipts(&self, block_number: u64) -> Result<Vec<TransactionReceipt>> {
        let _permit = self.semaphore.acquire().await;
        let num = BlockId::number(block_number);
        let tx_receipt = self
            .provider
            .get_block_receipts(num)
            .await
            .map_err(|e| anyhow!("RPC error: {}", e))? // Handle RPC transport errors
            .ok_or_else(|| anyhow!("Block {} not found", block_number))?; // Handle null/None result

        Ok(tx_receipt)
    }

    pub async fn fetch_alloy_block(&self, block_number: u64) -> Result<AlloyBlock> {
        let _permit = self.semaphore.acquire().await;
        let num = BlockNumberOrTag::Number(block_number);
        let alloy_block: AlloyBlock = self
            .provider
            .get_block_by_number(num)
            .full()
            .await
            .map_err(|e| anyhow!("RPC error: {}", e))?
            .ok_or_else(|| anyhow!("Block {} not found", block_number))?;

        Ok(alloy_block)
    }

    pub async fn fetch_block(&self, block_number: u64) -> Result<Block> {
        let num = BlockNumberOrTag::Number(block_number);
        let alloy_block: AlloyBlock = self
            .provider
            .get_block_by_number(num)
            .full()
            .await
            .map_err(|e| anyhow!("RPC error: {}", e))? // Handle RPC transport errors
            .ok_or_else(|| anyhow!("Block {} not found", block_number))?; // Handle null/None result

        // 3. Convert
        // This relies on: impl From<alloy::rpc::types::Block> for common::types::Block
        Ok(alloy_block.into())
    }

    pub async fn fetch_remote_latest_block_number(&self) -> Result<u64> {
        let block_number = self.provider.get_block_number().await?;
        Ok(block_number)
    }

    pub async fn fetch_latest_block_full_data(&self) -> Result<Block> {
        let block: AlloyBlock = self
            .provider
            .get_block_by_number(alloy::eips::BlockNumberOrTag::Latest)
            .full()
            .await
            .map_err(|e| anyhow!("RPC error: {}", e))?
            .ok_or_else(|| anyhow!("Couldn't fetch the latest block"))?;
        Ok(block.into())
    }
}
