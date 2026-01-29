use crate::AppState;
use crate::error::ApiError;
use alloy_primitives::B256;
use axum::extract::{Json, Path, State};
use axum::{self};
use common::{db::fetch_tx_by_hash, types::Transaction};

pub async fn get_tx(
    State(state): State<AppState>,
    Path(hash): Path<B256>,
) -> Result<Json<Transaction>, ApiError> {
    let tx = fetch_tx_by_hash(&state.db, hash).await?;
    Ok(tx.into())
}
