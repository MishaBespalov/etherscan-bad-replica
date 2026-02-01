use crate::{ApiState, error::ApiError};

use alloy_primitives::B256;
use axum::{
    self,
    extract::{Json, Path, State},
};
use common::{db::fetch_tx_by_hash, types::Transaction};

pub async fn get_tx(
    State(state): State<ApiState>,
    Path(hash): Path<B256>,
) -> Result<Json<Transaction>, ApiError> {
    let tx = fetch_tx_by_hash(&state.db, hash).await?;
    Ok(tx.into())
}
