use anyhow::Context;
use serde::{de::DeserializeOwned, Serialize};

pub fn serialize(value: impl Serialize) -> Result<Vec<u8>, anyhow::Error> {
    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut serializer = serde_json::Serializer::with_formatter(&mut buf, formatter);
    value
        .serialize(&mut serializer)
        .context("Can't serialize json")?;
    Ok(buf)
}

pub fn deserialize<T: DeserializeOwned>(slice: &[u8]) -> Result<T, anyhow::Error> {
    let value = serde_json::from_slice(slice).context("Can't deserialize json")?;
    Ok(value)
}
