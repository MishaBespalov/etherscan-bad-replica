use anyhow::Result;
use common::db::{insert_block, insert_contract, insert_log, insert_token_transfer, insert_tx};
use common::types::ProcessedBlock;
use sqlx::postgres::PgPool;
use tokio::sync::mpsc::Receiver;

pub struct Writer {
    pub processed_block_receiver: Receiver<ProcessedBlock>,
    pub db_pool: PgPool,
}

impl Writer {
    pub async fn run(&mut self) -> Result<()> {
        let mut batch: Vec<ProcessedBlock> = Vec::with_capacity(32);
        while let Some(processed_block) = self
            .processed_block_receiver
            .recv()
            .await
        {
            batch.push(processed_block);
            if batch.len() >= 8 {
                self.write_batch(batch).await?;
                batch = Vec::with_capacity(32);
            }
        }
        if !batch.is_empty() {
            self.write_batch(batch).await?;
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
