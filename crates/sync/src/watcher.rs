use alloy::primitives::B256;
use alloy::providers::Provider;
use sqlx::PgPool;
use std::collections::VecDeque;

use anyhow::{Result, anyhow, bail};
use common::{
    db::{fetch_block_history, fetch_block_history_until},
    types::{BlockData, RawBlockData},
};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    watch,
};

#[derive(Clone, Copy)]
pub enum SyncState {
    Sync,
    Stop,
    Reorg(u64),
}

pub struct Watcher<P>
where
    P: Provider + Clone + Send + Sync + 'static,
{
    pub db_pool: PgPool,
    pub state_tx: watch::Sender<SyncState>,
    pub provider: P,
    pub receiver: Receiver<RawBlockData>,
    pub sender: Sender<RawBlockData>,
    pub history: VecDeque<BlockData>,
}

impl<P> Watcher<P>
where
    P: Provider + Clone + Send + Sync + 'static,
{
    pub async fn new(
        db_pool: PgPool,
        state_tx: watch::Sender<SyncState>,
        provider: P,
        receiver: Receiver<RawBlockData>,
        sender: Sender<RawBlockData>,
    ) -> Result<Self> {
        let history = fetch_block_history(&db_pool).await?;
        Ok(Watcher {
            db_pool,
            state_tx,
            provider,
            receiver,
            sender,
            history,
        })
    }
    pub async fn search_ancestor(&mut self) -> Result<u64> {
        let mut last_checked_number = None;

        loop {
            // 1. Refill if empty
            if self.history.is_empty() {
                if let Some(number) = last_checked_number {
                    let new_history = fetch_block_history_until(&self.db_pool, number).await?;

                    if new_history.is_empty() {
                        return Err(anyhow!("Exhausted local history without finding ancestor"));
                    }

                    self.history.extend(new_history);
                } else {
                    // Handle case where history was empty from the start (if possible)
                    return Err(anyhow!("History empty and no previous block to fetch from"));
                }
            }

            // 2. Pop and Check
            if let Some(block) = self.history.pop_back() {
                last_checked_number = Some(block.number);

                let provider_block = self
                    .provider
                    .get_block_by_number(alloy::eips::BlockNumberOrTag::Number(block.number + 1))
                    .await
                    .map_err(|e| anyhow!("RPC error: {}", e))?
                    .ok_or_else(|| anyhow!("Block {} not found on provider", block.number + 1))?;

                if block.hash == provider_block.header.parent_hash {
                    let number = block.number;
                    self.history.push_back(block);
                    return Ok(number);
                }
            }
        }
    }

    pub async fn process(&mut self) -> Result<()> {
        while let Some(block) = self.receiver.recv().await {
            if block.raw_block.header.parent_hash
                == self
                    .history
                    .pop_back()
                    .expect("failed to pop history block")
                    .hash
            {
                push_with_limit(
                    &mut self.history,
                    BlockData {
                        number: block.raw_block.number(),
                        hash: block.raw_block.hash(),
                        parent_hash: block.raw_block.header.parent_hash,
                    },
                );
                if let Err(e) = self.sender.send(block).await {
                    bail!("Error while sending: {}", e);
                }
            } else {
                _ = self.state_tx.send(SyncState::Stop);
                let ancestor = self.search_ancestor().await?;
                if let Err(e) = self
                    .state_tx
                    .send(SyncState::Reorg(ancestor))
                {
                    bail!("Error while sending: {}", e);
                }
            }
        }

        Ok(())
    }
}

fn push_with_limit<T>(queue: &mut VecDeque<T>, item: T) {
    if queue.len() == 128 {
        queue.pop_front();
    }
    queue.push_back(item);
}
