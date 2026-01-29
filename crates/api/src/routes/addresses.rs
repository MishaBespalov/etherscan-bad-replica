use crate::AppState;
use crate::error::ApiError;
use alloy_primitives::Address;
use axum::extract::{Json, Path, Query, State};
use axum::{self};
use common::db::fetch_addresses_txs;
use common::types::Transaction;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Pagination {
    pub page: Option<u32>,
    pub per_page: Option<u32>,
}

impl Pagination {
    pub fn to_limit_offset(&self) -> (i64, i64) {
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page.unwrap_or(10).min(100);
        let limit = per_page as i64;
        let offset = ((page - 1) * per_page) as i64;
        (limit, offset)
    }
}

pub async fn get_addresses_txs(
    State(state): State<AppState>,
    Path(addr): Path<Address>,
    Query(pagination): Query<Pagination>,
) -> Result<Json<Vec<Transaction>>, ApiError> {
    let (limit, offset) = pagination.to_limit_offset();
    let block = fetch_addresses_txs(&state.db, addr, limit, offset).await?;
    Ok(block.into())
}
