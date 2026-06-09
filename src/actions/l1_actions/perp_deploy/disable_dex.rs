use hl_rs_derive::L1Action;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Disable a perp DEX.
///
/// Serialized payload shape:
/// `{ "type": "perpDeploy", "disableDex": "<dex name>" }`
#[derive(Debug, Clone, L1Action)]
#[action(action_type = "perpDeploy", payload_key = "disableDex")]
pub struct DisableDex {
    pub dex: String,
    pub nonce: Option<u64>,
}

impl DisableDex {
    pub fn new(dex: impl Into<String>) -> Self {
        Self {
            dex: dex.into(),
            nonce: None,
        }
    }
}

impl Serialize for DisableDex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.dex)
    }
}

impl<'de> Deserialize<'de> for DisableDex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let dex = String::deserialize(deserializer)?;
        Ok(Self { dex, nonce: None })
    }
}
