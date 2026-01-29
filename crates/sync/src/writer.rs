use crate::watcher::SyncState;
use anyhow::{Result, bail};
use common::{
    db::{
        drop_blocks_data_until, insert_block, insert_contract, insert_log, insert_token_transfer,
        insert_tx,
    },
    types::ProcessedBlock,
};
use sqlx::postgres::PgPool;
use tokio::{
    select,
    sync::{mpsc::Receiver, watch},
};

pub struct Writer {
    pub processed_block_receiver: Receiver<ProcessedBlock>,
    pub db_pool: PgPool,
    pub state_check: watch::Receiver<SyncState>,
}

impl Writer {
    pub async fn run(&mut self) -> Result<()> {
        let mut batch = Vec::with_capacity(32);

        loop {
            select! {
                maybe_block = self.processed_block_receiver.recv() => {
                    match maybe_block {
                        Some(processed_block) => {
                            batch.push(processed_block);

                            if batch.len() >= 8 {
                                self.write_batch(batch).await?;
                                batch = Vec::with_capacity(32); // Reset
                            }
                        }
                        None => {
                            if !batch.is_empty() {
                                self.write_batch(batch).await?;
                            }
                            break;
                        }
                    }
                }

                result = self.state_check.changed() => {
                    if result.is_err() {
                         bail!("internal error while receiving the state");
                    }

                    let state = *self.state_check.borrow_and_update();

                    match state {
                        SyncState::Reorg(number) => {
                            drop_blocks_data_until(&self.db_pool, number).await?;
                            batch.clear();
                        }
                        SyncState::Sync => continue,
                        SyncState::Stop => continue,
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn write_batch(&self, batch: Vec<ProcessedBlock>) -> Result<()> {
        let mut tx = self.db_pool.begin().await?;
        for block in batch {
            insert_block(&mut *tx, block.block).await?;
            for transaction in block.transactions {
                insert_tx(&mut *tx, transaction).await?;
            }
            for log in block.logs {
                insert_log(&mut *tx, log).await?;
            }
            for contract in block.contracts {
                insert_contract(&mut *tx, contract).await?;
            }
            for token_transfer in block.token_transfers {
                insert_token_transfer(&mut *tx, token_transfer).await?;
            }
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn write_block(&self, processed_block: ProcessedBlock) -> Result<()> {
        let mut tx = self.db_pool.begin().await?;
        insert_block(&mut *tx, processed_block.block).await?;
        for transaction in processed_block.transactions {
            insert_tx(&mut *tx, transaction).await?;
        }
        for log in processed_block.logs {
            insert_log(&mut *tx, log).await?;
        }
        for contract in processed_block.contracts {
            insert_contract(&mut *tx, contract).await?;
        }
        for token_transfer in processed_block.token_transfers {
            insert_token_transfer(&mut *tx, token_transfer).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
