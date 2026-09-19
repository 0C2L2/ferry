/// Progress event payload for cloud uploads.
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct CloudProgress {
    pub uploaded: u64,
    pub total: u64,
    pub part: u64,
    pub parts: u64,
}

