use std::collections::HashMap;

use anyhow::{Result, bail, ensure};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct WireField {
    pub number: u32,
    pub name: Option<String>,
    pub wire_type: u8,
    pub value: WireValue,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum WireValue {
    Varint(u64),
    Fixed64(u64),
    LengthDelimited { len: usize, utf8: Option<String>, hex: String },
    Fixed32(u32),
}

pub fn decode_message(data: &[u8], names: Option<&HashMap<u32, String>>) -> Result<Vec<WireField>> {
    let mut p = 0usize;
    let mut out = Vec::new();
    while p < data.len() {
        let key = read_varint(data, &mut p)?;
        ensure!(key != 0, "protobuf field key cannot be zero");
        let number = (key >> 3) as u32;
        let wire_type = (key & 7) as u8;
        ensure!(number != 0, "protobuf field number cannot be zero");
        let value = match wire_type {
            0 => WireValue::Varint(read_varint(data, &mut p)?),
            1 => {
                ensure!(p + 8 <= data.len(), "fixed64 field {number} truncated");
                let v = u64::from_le_bytes(data[p..p + 8].try_into().unwrap()); p += 8;
                WireValue::Fixed64(v)
            }
            2 => {
                let len = read_varint(data, &mut p)? as usize;
                ensure!(len <= data.len().saturating_sub(p), "length-delimited field {number} truncated");
                let bytes = &data[p..p + len]; p += len;
                let utf8 = std::str::from_utf8(bytes).ok().filter(|s| s.chars().all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))).map(str::to_owned);
                WireValue::LengthDelimited { len, utf8, hex: hex::encode(bytes) }
            }
            5 => {
                ensure!(p + 4 <= data.len(), "fixed32 field {number} truncated");
                let v = u32::from_le_bytes(data[p..p + 4].try_into().unwrap()); p += 4;
                WireValue::Fixed32(v)
            }
            3 | 4 => bail!("deprecated protobuf group wire type {wire_type} is not supported"),
            _ => bail!("invalid protobuf wire type {wire_type}"),
        };
        out.push(WireField { number, name: names.and_then(|m| m.get(&number).cloned()), wire_type, value });
    }
    Ok(out)
}

fn read_varint(data: &[u8], p: &mut usize) -> Result<u64> {
    let mut value = 0u64;
    for shift in (0..70).step_by(7) {
        let Some(&b) = data.get(*p) else { bail!("truncated protobuf varint") };
        *p += 1;
        if shift == 63 && b > 1 { bail!("protobuf varint overflows u64") }
        value |= ((b & 0x7f) as u64) << shift;
        if b & 0x80 == 0 { return Ok(value); }
    }
    bail!("protobuf varint exceeds 10 bytes")
}
