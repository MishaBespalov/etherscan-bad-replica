mod fetcher;
mod processor;
mod writer;
use alloy::providers::ProviderBuilder;
use anyhow::Result;
use common::types::{ProcessedBlock, RawBlockData};
use sqlx::postgres::PgPoolOptions;
use tokio::signal::unix::{SignalKind, signal};
use tokio::{join, select, sync::mpsc};
use tokio_util::sync::CancellationToken;
use tracing::{Instrument, error, info};

use fetcher::FetcherBuilder;
use processor::Processor;
use writer::Writer;

#[tokio::main]
async fn main() -> Result<()> {
    let token = CancellationToken::new();
    tracing_subscriber::fmt::init();
    let (fetcher_tx, processor_rx) = mpsc::channel::<RawBlockData>(32);
    let (processor_tx, writer_rx) = mpsc::channel::<ProcessedBlock>(32);
    let db_pool = PgPoolOptions::new()
        .max_connections(69)
        .connect("postgres://user:password@127.0.0.1:5432/etherscan?sslmode=disable")
        .await?;
    let provider = ProviderBuilder::new()
        .connect("https://eth.latticenode.io")
        .await?;
    let fetcher = FetcherBuilder::default()
        .provider(provider)
        .sender(fetcher_tx)
        .build()?;
    let mut processor = Processor {
        block_receiver: processor_rx,
        processed_block_sender: processor_tx,
    };
    let mut writer = Writer {
        processed_block_receiver: writer_rx,
        db_pool: db_pool.clone(),
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
    let fetcher_handle = tokio::spawn(
        async move {
            select! {
                       res = fetcher.run(db_pool.clone()) => {
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

    let processor_token = token.clone();
    let processor_handle = tokio::spawn(
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
    let writer_handle = tokio::spawn(
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

    match join!(fetcher_handle, writer_handle, processor_handle) {
        (Ok(_), Ok(_), Ok(_)) => info!("All services shut down successfully"),
        (Err(e), _, _) => error!(error = ?e, "Fetcher task panicked or was cancelled"),
        (_, Err(e), _) => error!(error = ?e, "Writer task panicked or was cancelled"),
        (_, _, Err(e)) => error!(error = ?e, "Processor task panicked or was cancelled"),
    }
    Ok(())
}
