//! Internal identity and external IFC identity are separate semantic fields.
use crate::{DomainError, validate_name};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const IFC_ALPHABET: &[u8; 64] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz_$";

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(try_from = "String", into = "String")]
pub struct IfcGuid(String);
impl IfcGuid {
    pub fn from_uuid(id: Uuid) -> Self {
        let mut value = id.as_u128();
        let mut bytes = [b'0'; 22];
        for index in (0..22).rev() {
            bytes[index] = IFC_ALPHABET[(value & 63) as usize];
            value >>= 6;
        }
        Self(String::from_utf8(bytes.to_vec()).expect("IFC alphabet is ASCII"))
    }
    pub fn to_uuid(&self) -> Uuid {
        let mut value = 0u128;
        for byte in self.0.bytes() {
            value = (value << 6)
                | IFC_ALPHABET
                    .iter()
                    .position(|x| *x == byte)
                    .expect("validated IFC alphabet") as u128;
        }
        Uuid::from_u128(value)
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for IfcGuid {
    type Error = DomainError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        if value.len() != 22
            || !matches!(value.as_bytes().first(), Some(b'0'..=b'3'))
            || !value.bytes().all(|x| IFC_ALPHABET.contains(&x))
        {
            return Err(DomainError::InvalidIfcGuid);
        }
        Ok(Self(value))
    }
}
impl From<IfcGuid> for String {
    fn from(value: IfcGuid) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceOrigin {
    Native {},
    Imported {
        system: String,
        reference: String,
        external_id: String,
    },
    Derived {
        operation: String,
    },
}
impl SourceOrigin {
    pub fn validate(&self) -> Result<(), DomainError> {
        match self {
            Self::Native {} => Ok(()),
            Self::Imported {
                system,
                reference,
                external_id,
            } => {
                validate_name(system)?;
                validate_name(reference)?;
                validate_name(external_id)
            }
            Self::Derived { operation } => validate_name(operation),
        }
    }
}

impl Default for SourceOrigin {
    fn default() -> Self {
        Self::Native {}
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EntityIdentity {
    pub ifc_guid: Option<IfcGuid>,
    pub origin: SourceOrigin,
    pub created_revision: Option<u64>,
    pub created_by: Option<String>,
    pub retired_revision: Option<u64>,
}
