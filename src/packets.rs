use std::collections::HashMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::metadata::{MetadataSummary, MetadataV39};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketField {
    pub name: String,
    pub number: i32,
    pub constant_field: String,
    pub backing_field: Option<String>,
    pub type_index: Option<i32>,
    pub resolved_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketSchema {
    pub name: String,
    pub namespace: String,
    pub direction: String,
    pub type_definition_index: usize,
    pub fields: Vec<PacketField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvelopeEntry {
    pub tag: i32,
    pub field_name: String,
    pub payload_type: Option<String>,
    pub type_index: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketCatalog {
    pub packets: Vec<PacketSchema>,
    pub request_envelope: Vec<EnvelopeEntry>,
    pub response_envelope: Vec<EnvelopeEntry>,
}

impl PacketCatalog {
    pub fn from_metadata(metadata: &MetadataV39) -> Result<Self> {
        let mut packets = Vec::new();
        for ty in &metadata.types {
            let direction = if packet_prefix(&ty.name, "Req") || packet_prefix(&ty.name, "MsgReq") {
                "request"
            } else if packet_prefix(&ty.name, "Rsp") || packet_prefix(&ty.name, "MsgRsp") {
                "response"
            } else {
                continue;
            };

            let slice = if ty.field_start < 0 {
                &metadata.fields[0..0]
            } else {
                let start = ty.field_start as usize;
                let end = start.saturating_add(ty.field_count as usize).min(metadata.fields.len());
                metadata.fields.get(start..end).context("type field range out of bounds")?
            };
            let by_name: HashMap<&str, _> = slice.iter().map(|f| (f.name.as_str(), f)).collect();
            let mut packet_fields = Vec::new();

            for f in slice {
                let Some(base) = f.name.strip_suffix("FieldNumber") else { continue };
                let Some(number) = metadata.field_defaults.get(&f.index).copied() else { continue };
                if number <= 0 { continue; }
                let camel = lower_first(base);
                let candidates = [format!("{camel}_"), format!("{base}_"), format!("<{base}>k__BackingField")];
                let backing = candidates.iter().find_map(|n| by_name.get(n.as_str()).copied());
                let type_index = backing.map(|x| x.type_index);
                let resolved_type = type_index.and_then(|idx| metadata.resolve_direct_type(idx)).map(str::to_owned);
                packet_fields.push(PacketField {
                    name: camel,
                    number,
                    constant_field: f.name.clone(),
                    backing_field: backing.map(|x| x.name.clone()),
                    type_index,
                    resolved_type,
                });
            }
            packet_fields.sort_by_key(|f| f.number);
            packets.push(PacketSchema {
                name: ty.name.clone(), namespace: ty.namespace.clone(), direction: direction.to_owned(),
                type_definition_index: ty.index, fields: packet_fields,
            });
        }
        packets.sort_by(|a, b| a.name.cmp(&b.name));
        let request_envelope = envelope_entries(metadata, "Request")?;
        let response_envelope = envelope_entries(metadata, "Response")?;
        Ok(Self { packets, request_envelope, response_envelope })
    }

    pub fn load_json(path: &Path) -> Result<Self> {
        let data = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        Ok(serde_json::from_slice(&data)?)
    }

    pub fn field_name_map(&self, packet: &str) -> Result<HashMap<u32, String>> {
        let schema = self.packets.iter().find(|p| p.name == packet)
            .ok_or_else(|| anyhow::anyhow!("packet {packet:?} not found in catalog"))?;
        Ok(schema.fields.iter().filter(|f| f.number > 0).map(|f| (f.number as u32, f.name.clone())).collect())
    }
}

pub fn write_packet_outputs(output: &Path, metadata: &MetadataV39, catalog: &PacketCatalog, unity_version: Option<&str>) -> Result<()> {
    fs::create_dir_all(output)?;
    fs::write(output.join("packets.json"), serde_json::to_vec_pretty(catalog)?)?;
    fs::write(output.join("metadata_summary.json"), serde_json::to_vec_pretty(&metadata.summary())?)?;
    fs::write(output.join("packet_skeleton.proto"), proto_skeleton(catalog))?;
    fs::write(output.join("REPORT.md"), report(metadata.summary(), catalog, unity_version))?;
    Ok(())
}

fn report(summary: MetadataSummary, catalog: &PacketCatalog, unity_version: Option<&str>) -> String {
    let requests = catalog.packets.iter().filter(|p| p.direction == "request").count();
    let responses = catalog.packets.len() - requests;
    let mut s = String::new();
    s.push_str("# Monster Super League IL2CPP / packet report\n\n");
    if let Some(v) = unity_version { s.push_str(&format!("- Unity: `{v}`\n")); }
    s.push_str(&format!("- IL2CPP metadata version: `{}`\n", summary.version));
    s.push_str(&format!("- Type definitions: `{}`\n", summary.sections["typeDefinitions"].count));
    s.push_str(&format!("- Methods: `{}`\n", summary.sections["methods"].count));
    s.push_str(&format!("- Fields: `{}`\n", summary.sections["fields"].count));
    s.push_str(&format!("- Request-like protobuf types: `{requests}`\n"));
    s.push_str(&format!("- Response-like protobuf types: `{responses}`\n"));
    s.push_str(&format!("- Request envelope payload tags: `{}`\n", catalog.request_envelope.len()));
    s.push_str(&format!("- Response envelope payload tags: `{}`\n", catalog.response_envelope.len()));
    s.push_str("\nField numbers are recovered from generated `*FieldNumber` constants in IL2CPP metadata. `resolved_type` is only emitted when the build's direct TypeIndex mapping is unambiguous; generic collection instances stay as raw TypeIndex values.\n\n");

    s.push_str("## Envelope/tag examples\n\n| request tag | payload | response payload |\n|---:|---|---|\n");
    for tag in [50, 60, 61, 117, 500, 1000, 3004] {
        let req = catalog.request_envelope.iter().find(|e| e.tag == tag).and_then(|e| e.payload_type.as_deref()).unwrap_or("");
        let rsp = catalog.response_envelope.iter().find(|e| e.tag == tag).and_then(|e| e.payload_type.as_deref()).unwrap_or("");
        if !req.is_empty() || !rsp.is_empty() { s.push_str(&format!("| {tag} | `{req}` | `{rsp}` |\n")); }
    }
    s.push('\n');

    for wanted in ["ReqUserLogin", "RspUserLogin", "ReqBattleStart", "RspBattleEnd", "RspServerTime"] {
        let Some(p) = catalog.packets.iter().find(|p| p.name == wanted) else { continue };
        s.push_str(&format!("## `{}`\n\n| # | field | resolved type | TypeIndex |\n|---:|---|---|---:|\n", p.name));
        for f in &p.fields {
            s.push_str(&format!("| {} | `{}` | `{}` | {} |\n", f.number, f.name,
                f.resolved_type.as_deref().unwrap_or(""), f.type_index.map(|x| x.to_string()).unwrap_or_default()));
        }
        s.push('\n');
    }
    s
}

fn proto_skeleton(catalog: &PacketCatalog) -> String {
    let mut out = String::from("syntax = \"proto3\";\npackage msl.recovered;\n\n// Recovered field numbers. Unknown/generic IL2CPP types are represented as bytes.\n\n");
    for packet in &catalog.packets {
        out.push_str(&format!("message {} {{\n", sanitize_ident(&packet.name)));
        for f in &packet.fields {
            let (proto_type, comment) = proto_type(f.resolved_type.as_deref(), f.type_index);
            out.push_str(&format!("  {proto_type} {} = {};{}\n", sanitize_field(&f.name), f.number, comment));
        }
        out.push_str("}\n\n");
    }
    out
}

fn proto_type(resolved: Option<&str>, type_index: Option<i32>) -> (&'static str, String) {
    match resolved {
        Some("System.String") => ("string", String::new()),
        Some("System.Boolean") => ("bool", String::new()),
        Some("System.Int32") => ("int32", String::new()),
        Some("System.UInt32") => ("uint32", String::new()),
        Some("System.Int64") => ("int64", String::new()),
        Some("System.UInt64") => ("uint64", String::new()),
        Some("System.Single") => ("float", String::new()),
        Some("System.Double") => ("double", String::new()),
        Some(name) => ("bytes", format!(" // IL2CPP direct type: {name}")),
        None => ("bytes", type_index.map(|x| format!(" // unresolved TypeIndex {x}")).unwrap_or_default()),
    }
}

fn envelope_entries(metadata: &MetadataV39, envelope_name: &str) -> Result<Vec<EnvelopeEntry>> {
    let Some(ty) = metadata.types.iter().find(|t| t.namespace.is_empty() && t.name == envelope_name) else {
        return Ok(Vec::new());
    };
    if ty.field_start < 0 { return Ok(Vec::new()); }
    let start = ty.field_start as usize;
    let end = start.saturating_add(ty.field_count as usize).min(metadata.fields.len());
    let slice = metadata.fields.get(start..end).context("envelope field range out of bounds")?;
    let by_name: HashMap<&str, _> = slice.iter().map(|f| (f.name.as_str(), f)).collect();
    let mut entries = Vec::new();
    for f in slice {
        let Some(base) = f.name.strip_suffix("FieldNumber") else { continue };
        let Some(tag) = metadata.field_defaults.get(&f.index).copied() else { continue };
        // 1..5 are envelope headers (protocol version/id, sequence, token/result/ticket),
        // not payload packet discriminators.
        if tag < 50 { continue; }
        let camel = lower_first(base);
        let candidates = [format!("{camel}_"), format!("{base}_"), format!("<{base}>k__BackingField")];
        let backing = candidates.iter().find_map(|n| by_name.get(n.as_str()).copied());
        let type_index = backing.map(|x| x.type_index);
        let payload_type = type_index.and_then(|idx| metadata.resolve_direct_type(idx)).map(str::to_owned);
        entries.push(EnvelopeEntry { tag, field_name: camel, payload_type, type_index });
    }
    entries.sort_by_key(|e| e.tag);
    Ok(entries)
}

fn packet_prefix(name: &str, prefix: &str) -> bool {
    name.strip_prefix(prefix)
        .and_then(|rest| rest.chars().next())
        .is_some_and(|c| c.is_ascii_uppercase())
}

fn lower_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(c) => c.to_lowercase().collect::<String>() + chars.as_str(),
    }
}

fn sanitize_ident(s: &str) -> String {
    let mut out: String = s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) { out.insert(0, '_'); }
    out
}

fn sanitize_field(s: &str) -> String {
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_ascii_uppercase() && i > 0 { out.push('_'); }
        out.push(c.to_ascii_lowercase());
    }
    sanitize_ident(&out)
}
