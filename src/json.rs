use crate::errors::BogrepError;
use serde::{de::DeserializeOwned, Serialize};

pub fn serialize(value: impl Serialize) -> Result<Vec<u8>, BogrepError> {
    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut serializer = serde_json::Serializer::with_formatter(&mut buf, formatter);
    value
        .serialize(&mut serializer)
        .map_err(BogrepError::SerializeJson)?;
    Ok(buf)
}

pub fn deserialize<T: DeserializeOwned>(slice: &[u8]) -> Result<T, BogrepError> {
    let value = serde_json::from_slice(slice).map_err(BogrepError::DeserializeJson)?;
    Ok(value)
}
