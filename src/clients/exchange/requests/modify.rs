use serde::{Deserialize, Serialize};

use crate::exchange::requests::{ClientOrderRequest, OrderRequest};

#[derive(Debug)]
pub struct ClientModifyRequest {
    pub oid: u64,
    pub order: ClientOrderRequest,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModifyRequest {
    pub oid: u64,
    pub order: OrderRequest,
}
