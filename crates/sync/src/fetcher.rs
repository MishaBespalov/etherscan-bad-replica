use crate::watcher::SyncState;
use alloy::{
    eips::{BlockId, BlockNumberOrTag},
    providers::Provider,
    rpc::types::{Block as AlloyBlock, TransactionReceipt},
};
use anyhow::{Context, Result, anyhow, bail};
use common::{db::fetch_latest_block_number as pg_fetch_latest_block_number, types::RawBlockData};
use derive_builder::Builder;
use futures::{StreamExt, stream};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::{
    select,
    sync::{Semaphore, mpsc::Sender, watch},
};

#[derive(Builder)]
pub struct Fetcher<P>
where
    P: Provider + Clone + Send + Sync + 'static,
{
    provider: P,
    state_rx: watch::Receiver<SyncState>,
    sender: Sender<RawBlockData>,
    #[builder(default = "Arc::new(Semaphore::new(10))")]
    semaphore: Arc<Semaphore>,
}

impl<P> Fetcher<P>
where
    P: Provider + Clone + Send + Sync + 'static,
{
    pub async fn run(&mut self, pg_pool: PgPool) -> Result<()> {
        loop {
            select! {
                state = self.state_rx.changed() => {
                    if state.is_err() {
                         bail!("internal error while receiving the state");
                    }
                    let state = *self.state_rx.borrow_and_update();

                    match state {
                        SyncState::Reorg(ancestor) => {
                            self.process(pg_pool.clone(), Some(ancestor)).await?
                        }
                        SyncState::Sync => self.process(pg_pool.clone(), None).await?,
                        SyncState::Stop => continue,
                    }

                }
            }
        }
    }
    pub async fn process(&self, pg_pool: PgPool, ancestor: Option<u64>) -> Result<()> {
        let sub = self.provider.subscribe_blocks().await?;
        let mut stream = sub.into_stream();
        let first_block_number = stream
            .next()
            .await
            .context("failed to get the block")?
            .number;

        let from = match ancestor {
            Some(val) => val,
            None => pg_fetch_latest_block_number(&pg_pool).await?,
        };
        if first_block_number > from {
            self.backfill(from, first_block_number)
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

    pub async fn fetch_raw_block_data(&self, block_number: u64) -> Result<RawBlockData> {
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
        Ok(raw_block_data)
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
}
