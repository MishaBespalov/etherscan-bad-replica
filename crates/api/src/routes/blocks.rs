use crate::AppState;
use crate::error::ApiError;
use axum::extract::{Json, Path, State};
use axum::{self};
use common::{db::fetch_block_by_number, types::Block};

pub async fn get_block(
    State(state): State<AppState>,
    Path(block_number): Path<u64>,
) -> Result<Json<Block>, ApiError> {
    let block = fetch_block_by_number(&state.db, block_number).await?;
    Ok(block.into())
}
