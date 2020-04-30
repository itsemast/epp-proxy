use chrono::prelude::*;

#[derive(Debug, Deserialize)]
pub struct EPPChangeData {
    #[serde(rename = "$attr:state", default)]
    pub state: EPPChangeState,
    #[serde(rename = "{urn:ietf:params:xml:ns:changePoll-1.0}operation")]
    pub operation: EPPChangeOperation,
    #[serde(rename = "{urn:ietf:params:xml:ns:changePoll-1.0}date", deserialize_with = "super::deserialize_datetime")]
    pub date: DateTime<Utc>,
    #[serde(rename = "{urn:ietf:params:xml:ns:changePoll-1.0}svTRID")]
    pub server_transaction_id: String,
    #[serde(rename = "{urn:ietf:params:xml:ns:changePoll-1.0}who")]
    pub who: String,
    #[serde(rename = "{urn:ietf:params:xml:ns:changePoll-1.0}caseId", default)]
    pub case_id: Option<EPPChangeCaseId>,
    #[serde(rename = "{urn:ietf:params:xml:ns:changePoll-1.0}reason", default)]
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub enum EPPChangeState {
    #[serde(rename = "before")]
    Before,
    #[serde(rename = "after")]
    After,
}

impl Default for EPPChangeState {
    fn default() -> Self {
        EPPChangeState::After
    }
}

#[derive(Debug, Deserialize)]
pub struct EPPChangeOperation {
    #[serde(rename = "$attr:op")]
    pub operation: String,
    #[serde(rename = "$value")]
    pub op_type: EPPChangeOperationType,
}

#[derive(Debug, Deserialize)]
pub enum EPPChangeOperationType {
    #[serde(rename = "create")]
    Create,
    #[serde(rename = "delete")]
    Delete,
    #[serde(rename = "renew")]
    Renew,
    #[serde(rename = "transfer")]
    Transfer,
    #[serde(rename = "update")]
    Update,
    #[serde(rename = "restore")]
    Restore,
    #[serde(rename = "autoRenew")]
    AutoRenew,
    #[serde(rename = "autoDelete")]
    AutoDelete,
    #[serde(rename = "autoPurge")]
    AutoPurge,
    #[serde(rename = "custom")]
    Custom,
}

#[derive(Debug, Deserialize)]
pub struct EPPChangeCaseId {
    #[serde(rename = "type")]
    pub case_type: EPPChangeCaseIdType,
    #[serde(rename = "$value")]
    pub case_id: String,
}

#[derive(Debug, Deserialize)]
pub enum EPPChangeCaseIdType {
    #[serde(rename = "udrp")]
    UDRP,
    #[serde(rename = "urs")]
    URS,
    #[serde(rename = "custom")]
    Custom,
}
