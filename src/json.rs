use serde::Serialize;

pub fn serialize(value: impl Serialize) -> Result<Vec<u8>, anyhow::Error> {
    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    ");
    let mut serializer = serde_json::Serializer::with_formatter(&mut buf, formatter);
    value.serialize(&mut serializer)?;
    Ok(buf)
}
