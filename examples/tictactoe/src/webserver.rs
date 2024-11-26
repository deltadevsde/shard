use crate::node::Node;
use crate::tx::Transaction;
use axum::{extract::State as AxumState, http::StatusCode, Json};
use std::sync::Arc;

pub(crate) async fn submit_tx(
    AxumState(node): AxumState<Arc<Node>>,
    Json(tx): Json<Transaction>,
) -> Result<(), (StatusCode, String)> {
    node.queue_transaction(tx)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}
