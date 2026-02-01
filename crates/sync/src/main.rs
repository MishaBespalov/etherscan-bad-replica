use alloy::providers::ProviderBuilder;
use anyhow::Result;
use common::types::{ProcessedBlock, RawBlockData};
use sqlx::postgres::PgPoolOptions;
use tokio::{
    join, select,
    signal::unix::{SignalKind, signal},
    sync::{mpsc, watch},
    task::JoinSet,
};
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, error, info};

use sync::{
    fetcher::FetcherBuilder,
    processor::Processor,
    watcher::{SyncState, Watcher},
    writer::Writer,
};

#[tokio::main]
async fn main() -> Result<()> {
    let mut set = JoinSet::new();
    let token = CancellationToken::new();
    tracing_subscriber::fmt::init();
    let (fetcher_tx, watcher_rx) = mpsc::channel::<RawBlockData>(32);
    let (watcher_tx, processor_rx) = mpsc::channel::<RawBlockData>(32);
    let (processor_tx, writer_rx) = mpsc::channel::<ProcessedBlock>(32);
    let (sync_state_tx, sync_state_rx) = watch::channel(SyncState::Sync);
    let db_pool = PgPoolOptions::new()
        .max_connections(69)
        .connect("postgres://user:password@127.0.0.1:5432/etherscan?sslmode=disable")
        .await?;
    let provider = ProviderBuilder::new()
        .connect("https://eth.latticenode.io")
        .await?;
    let fetcher = FetcherBuilder::default()
        .provider(provider.clone())
        .sender(fetcher_tx)
        .state_rx(sync_state_rx.clone())
        .build()?;
    let mut watcher = Watcher::new(
        db_pool.clone(),
        sync_state_tx,
        provider,
        watcher_rx,
        watcher_tx,
    )
    .await?;

    let mut processor = Processor {
        block_receiver: processor_rx,
        processed_block_sender: processor_tx,
    };
    let mut writer = Writer {
        processed_block_receiver: writer_rx,
        db_pool: db_pool.clone(),
        state_check: sync_state_rx.clone(),
    };

    //TODO ● Current select! aborts mid-work. For cleaner shutdown, pass CancellationToken into each run() and check
    //   token.is_cancelled() between loop iterations—finishes current block before stopping.

    let shutdown_token = token.clone();
    tokio::spawn(async move {
        let mut sigterm =
            signal(SignalKind::terminate()).expect("failed to register sigterm handler");
        select! {
            _ = sigterm.recv() => info!("SIGTERM received")
        };
        shutdown_token.cancel();
    });

    let fetcher_token = token.clone();
    set.spawn(
        async move {
            select! {
                       res = fetcher.process(db_pool.clone(), None) => {
                       if let Err(e) = res  {
                           error!(error = ?e, "Fetcher encountered a fatal error");
                       }
                   }
                       _ = fetcher_token.cancelled() => {
            info!( "Fetcher stopping (cancellation requested)");
                           }
                   }
        }
        .instrument(tracing::info_span!("fetcher_handle")),
    );

    let watcher_token = token.clone();
    set.spawn(
        async move {
            select! {
                       res = watcher.process() => {
                       if let Err(e) = res  {
                           error!(error = ?e, "Watcher encountered a fatal error");
                       }
                   }
                       _ = watcher_token.cancelled() => {
            info!( "Watcher stopping (cancellation requested)");
                           }
                   }
        }
        .instrument(tracing::info_span!("watcher_handle")),
    );

    let processor_token = token.clone();
    set.spawn(
        async move {
            select! {
                            res = processor.run() => {
                            if let Err(e) = res {
                            error!(error = ?e, "Processor encountered a fatal error");
                            }
                        }
                           _ = processor_token.cancelled() => {
            info!("Processor stopping (cancellation requested)");
                        }
                    }
        }
        .instrument(tracing::info_span!("processor_handle")),
    );
    let writer_token = token.clone();
    set.spawn(
        async move {
            select! {
                res = writer.run() => {
                if let Err(e) = res {
                error!(error = ?e, "Writer encountered a fatal error");
                }
            }
                _ = writer_token.cancelled() =>{
            info!("Writer stopping (cancellation requested)");
                    }
            }
        }
        .instrument(tracing::info_span!("writer_handle")),
    );

    while let Some(res) = set.join_next().await {
        if let Err(e) = res {
            error!(error = ?e, "A service task panicked or was cancelled");
        }
    }
    info!("All services shut down successfully");
    Ok(())
}
