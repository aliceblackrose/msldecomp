use std::collections::HashMap;

use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;

const MAGIC: u32 = 0xFAB1_1BAF;
const HEADER_ENTRIES: [&str; 31] = [
    "stringLiteral", "stringLiteralData", "string", "events", "properties", "methods",
    "parameterDefaultValues", "fieldDefaultValues", "fieldAndParameterDefaultValueData",
    "fieldMarshaledSizes", "parameters", "fields", "genericParameters",
    "genericParameterConstraints", "genericContainers", "nestedTypes", "interfaces",
    "vtableMethods", "interfaceOffsets", "typeDefinitions", "images", "assemblies",
    "fieldRefs", "referencedAssemblies", "attributeData", "attributeDataRange",
    "unresolvedVirtualCallParameterTypes", "unresolvedVirtualCallParameterRanges",
    "windowsRuntimeTypeNames", "windowsRuntimeStrings", "exportedTypeDefinitions",
];

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Section {
    pub offset: u32,
    pub size: u32,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldDef {
    pub index: usize,
    pub name: String,
    pub type_index: i32,
    pub token: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeDef {
    pub index: usize,
    pub name: String,
    pub namespace: String,
    pub byval_type_index: i32,
    pub field_start: i32,
    pub method_start: i32,
    pub method_count: u16,
    pub field_count: u16,
    pub token: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetadataSummary {
    pub version: u32,
    pub byte_len: usize,
    pub sections: HashMap<String, Section>,
    pub type_index_width: usize,
    pub type_definition_index_width: usize,
    pub generic_container_index_width: usize,
    pub parameter_definition_index_width: usize,
}

#[derive(Debug)]
pub struct MetadataV39 {
    data: Vec<u8>,
    version: u32,
    sections: HashMap<&'static str, Section>,
    pub fields: Vec<FieldDef>,
    pub types: Vec<TypeDef>,
    pub field_defaults: HashMap<usize, i32>,
    pub type_index_width: usize,
    pub type_definition_index_width: usize,
    pub generic_container_index_width: usize,
    pub parameter_definition_index_width: usize,
    direct_type_names: HashMap<i32, String>,
}

impl MetadataV39 {
    pub fn parse(data: Vec<u8>) -> Result<Self> {
        ensure!(data.len() >= 8 + HEADER_ENTRIES.len() * 12, "metadata header is truncated");
        let sanity = u32_at(&data, 0)?;
        ensure!(sanity == MAGIC, "not global-metadata.dat: magic 0x{sanity:08X}");
        let version = u32_at(&data, 4)?;
        ensure!(version == 39, "this extractor currently targets metadata v39; found v{version}");

        let mut sections = HashMap::new();
        for (i, name) in HEADER_ENTRIES.iter().enumerate() {
            let p = 8 + i * 12;
            let s = Section {
                offset: u32_at(&data, p)?,
                size: u32_at(&data, p + 4)?,
                count: u32_at(&data, p + 8)?,
            };
            validate_section(&data, name, s)?;
            sections.insert(*name, s);
        }

        let fields_s = sections["fields"];
        ensure!(fields_s.count > 0, "metadata contains no fields");
        let field_record_size = fields_s.size as usize / fields_s.count as usize;
        ensure!(field_record_size >= 8, "invalid field record size {field_record_size}");
        let type_index_width = field_record_size - 8;
        ensure!([2, 4].contains(&type_index_width), "unsupported TypeIndex width {type_index_width}");

        let type_definition_index_width = index_width(sections["typeDefinitions"].count);
        let generic_container_index_width = index_width(sections["genericContainers"].count);
        let parameter_definition_index_width = index_width(sections["parameters"].count);

        let string_s = sections["string"];
        let get_string = |index: u32| -> Result<String> {
            if index == 0 { return Ok(String::new()); }
            let start = string_s.offset as usize + index as usize;
            let end_limit = string_s.offset as usize + string_s.size as usize;
            ensure!(start < end_limit && end_limit <= data.len(), "metadata string index out of range: {index}");
            let rel_end = data[start..end_limit].iter().position(|&b| b == 0).unwrap_or(end_limit - start);
            Ok(String::from_utf8_lossy(&data[start..start + rel_end]).into_owned())
        };

        let mut fields = Vec::with_capacity(fields_s.count as usize);
        for i in 0..fields_s.count as usize {
            let mut p = fields_s.offset as usize + i * field_record_size;
            let name_index = u32_at(&data, p)?; p += 4;
            let type_index = variable_index(&data, p, type_index_width)?; p += type_index_width;
            let token = u32_at(&data, p)?;
            fields.push(FieldDef { index: i, name: get_string(name_index)?, type_index, token });
        }

        let type_s = sections["typeDefinitions"];
        let type_record_size = type_s.size as usize / type_s.count as usize;
        let expected_type_size = 8 + type_index_width * 3 + generic_container_index_width + 4 + 32 + 16 + 8;
        ensure!(type_record_size == expected_type_size,
            "unexpected v39 type record size {type_record_size}; expected {expected_type_size}");

        let mut types = Vec::with_capacity(type_s.count as usize);
        let mut direct_type_names = HashMap::new();
        for i in 0..type_s.count as usize {
            let mut p = type_s.offset as usize + i * type_record_size;
            let name_index = u32_at(&data, p)?; p += 4;
            let namespace_index = u32_at(&data, p)?; p += 4;
            let byval_type_index = variable_index(&data, p, type_index_width)?; p += type_index_width;
            p += type_index_width; // declaringTypeIndex
            p += type_index_width; // parentIndex
            p += generic_container_index_width;
            p += 4; // flags
            let field_start = i32_at(&data, p)?; p += 4;
            let method_start = i32_at(&data, p)?; p += 4;
            p += 24; // event/property/nested/interface/vtable/interfaceOffsets starts
            let method_count = u16_at(&data, p)?; p += 2;
            p += 2; // property_count
            let field_count = u16_at(&data, p)?; p += 2;
            p += 10; // event/nested/vtable/interfaces/interface_offsets counts
            p += 4; // bitfield
            let token = u32_at(&data, p)?;
            let name = get_string(name_index)?;
            let namespace = get_string(namespace_index)?;
            let full_name = if namespace.is_empty() { name.clone() } else { format!("{namespace}.{name}") };
            // In this Unity 6000.3 / metadata-v39 build, field TypeIndex entries for direct
            // types are the adjacent by-ref slot (byval + 1). Generic instances do not obey this.
            direct_type_names.insert(byval_type_index.saturating_add(1), full_name);
            types.push(TypeDef { index: i, name, namespace, byval_type_index, field_start, method_start, method_count, field_count, token });
        }

        let field_defaults = parse_field_defaults(&data, &sections, fields.len())?;

        Ok(Self {
            data, version, sections, fields, types, field_defaults,
            type_index_width, type_definition_index_width, generic_container_index_width,
            parameter_definition_index_width, direct_type_names,
        })
    }

    pub fn version(&self) -> u32 { self.version }

    pub fn resolve_direct_type(&self, type_index: i32) -> Option<&str> {
        self.direct_type_names.get(&type_index).map(String::as_str)
    }

    pub fn summary(&self) -> MetadataSummary {
        MetadataSummary {
            version: self.version,
            byte_len: self.data.len(),
            sections: self.sections.iter().map(|(k, v)| ((*k).to_owned(), *v)).collect(),
            type_index_width: self.type_index_width,
            type_definition_index_width: self.type_definition_index_width,
            generic_container_index_width: self.generic_container_index_width,
            parameter_definition_index_width: self.parameter_definition_index_width,
        }
    }
}

fn parse_field_defaults(data: &[u8], sections: &HashMap<&'static str, Section>, field_count: usize) -> Result<HashMap<usize, i32>> {
    let defs = sections["fieldDefaultValues"];
    let values = sections["fieldAndParameterDefaultValueData"];
    if defs.count == 0 { return Ok(HashMap::new()); }
    ensure!(defs.size as usize / defs.count as usize == 12, "unexpected field-default record size");
    let mut out = HashMap::new();
    for i in 0..defs.count as usize {
        let p = defs.offset as usize + i * 12;
        let field_index = i32_at(data, p)?;
        let data_index = i32_at(data, p + 8)?;
        if field_index < 0 || field_index as usize >= field_count || data_index < 0 { continue; }
        let value_pos = values.offset as usize + data_index as usize;
        if value_pos >= values.offset as usize + values.size as usize { continue; }
        if let Ok((encoded, _)) = read_compressed_u32(data, value_pos) {
            let decoded = if encoded == u32::MAX {
                i32::MIN
            } else if encoded & 1 != 0 {
                -(((encoded >> 1) as i64 + 1) as i32)
            } else {
                (encoded >> 1) as i32
            };
            out.insert(field_index as usize, decoded);
        }
    }
    Ok(out)
}

fn read_compressed_u32(data: &[u8], p: usize) -> Result<(u32, usize)> {
    let read = *data.get(p).context("compressed integer truncated")?;
    if read & 0x80 == 0 {
        Ok((read as u32, 1))
    } else if read & 0xC0 == 0x80 {
        let b1 = *data.get(p + 1).context("compressed integer truncated")?;
        Ok((((read as u32 & !0x80) << 8) | b1 as u32, 2))
    } else if read & 0xE0 == 0xC0 {
        let b1 = *data.get(p + 1).context("compressed integer truncated")? as u32;
        let b2 = *data.get(p + 2).context("compressed integer truncated")? as u32;
        let b3 = *data.get(p + 3).context("compressed integer truncated")? as u32;
        Ok((((read as u32 & !0xC0) << 24) | (b1 << 16) | (b2 << 8) | b3, 4))
    } else if read == 0xF0 {
        Ok((u32_at(data, p + 1)?, 5))
    } else if read == 0xFE {
        Ok((u32::MAX - 1, 1))
    } else if read == 0xFF {
        Ok((u32::MAX, 1))
    } else {
        bail!("invalid IL2CPP compressed integer prefix 0x{read:02X}")
    }
}

fn index_width(count: u32) -> usize {
    if count <= u8::MAX as u32 { 1 } else if count <= u16::MAX as u32 { 2 } else { 4 }
}

fn validate_section(data: &[u8], name: &str, section: Section) -> Result<()> {
    let end = section.offset as usize + section.size as usize;
    ensure!(end <= data.len(), "metadata section {name} extends past EOF");
    Ok(())
}

fn variable_index(data: &[u8], p: usize, width: usize) -> Result<i32> {
    let raw = match width {
        1 => *data.get(p).context("index truncated")? as u32,
        2 => u16_at(data, p)? as u32,
        4 => u32_at(data, p)?,
        _ => bail!("unsupported variable index width {width}"),
    };
    let sentinel = match width { 1 => 0xFF, 2 => 0xFFFF, 4 => 0xFFFF_FFFF, _ => unreachable!() };
    if raw == sentinel { Ok(-1) } else { Ok(raw as i32) }
}

fn u16_at(data: &[u8], p: usize) -> Result<u16> {
    let bytes: [u8; 2] = data.get(p..p + 2).context("u16 truncated")?.try_into().unwrap();
    Ok(u16::from_le_bytes(bytes))
}
fn u32_at(data: &[u8], p: usize) -> Result<u32> {
    let bytes: [u8; 4] = data.get(p..p + 4).context("u32 truncated")?.try_into().unwrap();
    Ok(u32::from_le_bytes(bytes))
}
fn i32_at(data: &[u8], p: usize) -> Result<i32> { Ok(u32_at(data, p)? as i32) }
