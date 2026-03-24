use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Metadata stored in JSONB for soft deletes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeletedBy {
    pub user_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub reason: Option<String>,
}

impl DeletedBy {
    pub fn new(user_id: Uuid) -> Self {
        Self {
            user_id,
            timestamp: Utc::now(),
            reason: None,
        }
    }

    pub fn with_reason(mut self, reason: String) -> Self {
        self.reason = Some(reason);
        self
    }
}
