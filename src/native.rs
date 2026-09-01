use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail, ensure};
use il2cpp_dumper_engine::config::Config;
use il2cpp_dumper_engine::executor::Il2CppExecutor;
use il2cpp_dumper_engine::formats::elf::Elf;
use il2cpp_dumper_engine::il2cpp::base::Il2Cpp;
use il2cpp_dumper_engine::il2cpp::metadata::Metadata;
use il2cpp_dumper_engine::output::decompiler::Il2CppDecompiler;
use il2cpp_dumper_engine::output::struct_generator::StructGenerator;

/// Generate dump.cs + native headers from an ELF libil2cpp.so and global-metadata.dat.
/// This mirrors the pinned Rust dumper's ELF registration-discovery path but keeps MSL's
/// packet-schema extraction separate from native code recovery.
pub fn dump_elf(binary_path: &Path, metadata_path: &Path, output: &Path, unity_version: Option<&str>) -> Result<()> {
    fs::create_dir_all(output)?;
    let binary = fs::read(binary_path).with_context(|| format!("reading {}", binary_path.display()))?;
    ensure!(binary.get(0..4) == Some(b"\x7fELF"), "native mode currently supports ELF libil2cpp.so files");
    let is_32bit = binary.get(4).copied() == Some(1);

    let metadata_bytes = fs::read(metadata_path).with_context(|| format!("reading {}", metadata_path.display()))?;
    let mut metadata = Metadata::new_with_options(metadata_bytes, unity_version, false)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;

    let mut config = Config::default();
    config.require_any_key = false;
    config.generate_dummy_dll = false;
    config.dump_disassembly = false;

    let mut elf = Elf::new(binary, is_32bit).map_err(|e| anyhow::anyhow!(e.to_string()))?;
    elf.set_properties(metadata.version, metadata.metadata_usages_count as u64);

    let method_count = metadata.method_defs.iter().filter(|m| m.method_index >= 0).count();
    let type_count = metadata.type_defs.len();
    let image_count = metadata.image_defs.len();
    let mut helper = elf.get_section_helper(method_count, type_count, image_count);
    let code_reg = helper.find_code_registration();
    let metadata_reg = helper.find_metadata_registration();
    drop(helper);

    let mut initialized = elf.auto_plus_init(code_reg, metadata_reg).unwrap_or(false);
    if !initialized {
        if let Ok(Some((cr, mr))) = elf.symbol_search() {
            initialized = elf.init_with_auto_plus(cr, mr).is_ok();
        }
    }
    if !initialized {
        bail!("could not auto-locate CodeRegistration/MetadataRegistration in {}", binary_path.display());
    }

    let exports = elf.list_exported_symbols().unwrap_or_default();
    let mut il2cpp = Il2Cpp::from_elf(&elf);
    il2cpp.exported_symbols = exports.iter().map(|(name, _)| name.clone()).collect();
    for (name, addr) in exports {
        if name.starts_with("il2cpp_") || name.starts_with("mono_") {
            let rva = il2cpp.get_rva(addr);
            il2cpp.api_export_rvas.insert(name, rva);
        }
    }

    let mut executor = Il2CppExecutor::new(&metadata, &mut il2cpp)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    let out = output.to_string_lossy().into_owned();
    Il2CppDecompiler::decompile(&mut executor, &mut metadata, &mut il2cpp, &config, &out, |_| {})
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    StructGenerator::write_all(&mut executor, &mut metadata, &mut il2cpp, &config, &out, None)
        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
    Ok(())
}
