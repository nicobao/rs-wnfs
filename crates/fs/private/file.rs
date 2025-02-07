use chrono::{DateTime, Utc};
use semver::Version;
use serde::{de::Error as DeError, ser::Error as SerError, Deserialize, Deserializer, Serialize};

use crate::{dagcbor, Id, Metadata, NodeType};

use super::{namefilter::Namefilter, Key, PrivateNodeHeader, RatchetKey, Rng};

//--------------------------------------------------------------------------------------------------
// Type Definitions
//--------------------------------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct PrivateFile {
    pub version: Version,
    pub header: PrivateNodeHeader,
    pub metadata: Metadata,
    pub content: Vec<u8>, // TODO(appcypher): Support linked file content.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PrivateFileSerde {
    pub r#type: NodeType,
    pub version: Version,
    pub header: Vec<u8>,
    pub metadata: Metadata,
    pub content: Vec<u8>,
}

//--------------------------------------------------------------------------------------------------
// Implementations
//--------------------------------------------------------------------------------------------------

impl PrivateFile {
    pub fn new<R: Rng>(
        parent_bare_name: Namefilter,
        time: DateTime<Utc>,
        content: Vec<u8>,
        rng: &mut R,
    ) -> Self {
        Self {
            version: Version::new(0, 2, 0),
            header: PrivateNodeHeader::new(parent_bare_name, rng),
            metadata: Metadata::new(time),
            content,
        }
    }

    /// Serializes the file with provided Serde serialilzer.
    pub fn serialize<S, R: Rng>(&self, serializer: S, rng: &mut R) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let key = self
            .header
            .get_private_ref()
            .map_err(SerError::custom)?
            .ratchet_key;

        (PrivateFileSerde {
            r#type: NodeType::PrivateFile,
            version: self.version.clone(),
            header: {
                let cbor_bytes = dagcbor::encode(&self.header).map_err(SerError::custom)?;
                key.0
                    .encrypt(&Key::generate_nonce(rng), &cbor_bytes)
                    .map_err(SerError::custom)?
            },
            metadata: self.metadata.clone(),
            content: self.content.clone(),
        })
        .serialize(serializer)
    }

    /// Deserializes the file with provided Serde deserializer and key.
    pub fn deserialize<'de, D>(deserializer: D, key: &RatchetKey) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let PrivateFileSerde {
            version,
            metadata,
            header,
            content,
            ..
        } = PrivateFileSerde::deserialize(deserializer)?;

        Ok(Self {
            version,
            metadata,
            header: {
                let cbor_bytes = key.0.decrypt(&header).map_err(DeError::custom)?;
                dagcbor::decode(&cbor_bytes).map_err(DeError::custom)?
            },
            content,
        })
    }
}

impl Id for PrivateFile {
    fn get_id(&self) -> String {
        format!("{:p}", &self.header)
    }
}
