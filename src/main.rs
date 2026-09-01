mod metadata;
mod packets;
mod wire;
mod xapk;

#[cfg(feature = "native")]
mod native;

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};

use crate::metadata::MetadataV39;
use crate::packets::{PacketCatalog, write_packet_outputs};

#[derive(Parser, Debug)]
#[command(name = "msldecomp", version, about = "MSL IL2CPP + protobuf reverse-engineering toolkit")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Inspect an XAPK/APK, extract IL2CPP inputs, and recover packet schemas.
    Inspect {
        input: PathBuf,
        #[arg(short, long, default_value = "out")]
        output: PathBuf,
        #[arg(long, default_value = "arm64-v8a")]
        abi: String,
    },

    /// Parse a Unity 6 / IL2CPP v39 global-metadata.dat and recover Req*/Rsp* schemas.
    Packets {
        metadata: PathBuf,
        #[arg(short, long, default_value = "out")]
        output: PathBuf,
    },

    /// Decode a plaintext protobuf payload without connecting to the game service.
    Wire {
        /// Hex payload (whitespace/0x allowed) or a path when --file is used.
        input: String,
        #[arg(long)]
        file: bool,
        #[arg(long)]
        schema: Option<PathBuf>,
        #[arg(long)]
        packet: Option<String>,
    },

    /// Full IL2CPP dump.cs/native struct generation using the pinned Rust engine.
    #[cfg(feature = "native")]
    Native {
        binary: PathBuf,
        metadata: PathBuf,
        #[arg(short, long, default_value = "native-out")]
        output: PathBuf,
        #[arg(long)]
        unity_version: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Inspect { input, output, abi } => inspect(input, output, &abi),
        Command::Packets { metadata, output } => parse_packets(metadata, output),
        Command::Wire { input, file, schema, packet } => {
            let data = if file {
                fs::read(&input).with_context(|| format!("reading payload file {input}"))?
            } else {
                let clean: String = input
                    .trim()
                    .trim_start_matches("0x")
                    .chars()
                    .filter(|c| !c.is_whitespace() && *c != ':' && *c != '-')
                    .collect();
                hex::decode(&clean).context("invalid hexadecimal payload")?
            };
            let catalog = match schema {
                Some(path) => Some(PacketCatalog::load_json(&path)?),
                None => None,
            };
            let annotations = match (&catalog, packet.as_deref()) {
                (Some(c), Some(name)) => Some(c.field_name_map(name)?),
                (None, Some(_)) => bail!("--packet requires --schema packets.json"),
                _ => None,
            };
            let decoded = wire::decode_message(&data, annotations.as_ref())?;
            println!("{}", serde_json::to_string_pretty(&decoded)?);
            Ok(())
        }
        #[cfg(feature = "native")]
        Command::Native { binary, metadata, output, unity_version } => {
            native::dump_elf(&binary, &metadata, &output, unity_version.as_deref())
        }
    }
}

fn inspect(input: PathBuf, output: PathBuf, abi: &str) -> Result<()> {
    fs::create_dir_all(&output)?;
    let extracted = xapk::extract_il2cpp_inputs(&input, abi)?;

    let extracted_dir = output.join("extracted");
    fs::create_dir_all(&extracted_dir)?;
    let metadata_path = extracted_dir.join("global-metadata.dat");
    fs::write(&metadata_path, &extracted.metadata)?;

    if let Some(binary) = &extracted.il2cpp {
        fs::write(extracted_dir.join("libil2cpp.so"), binary)?;
    }

    let metadata = MetadataV39::parse(extracted.metadata)?;
    let catalog = PacketCatalog::from_metadata(&metadata)?;
    write_packet_outputs(&output, &metadata, &catalog, extracted.unity_version.as_deref())?;

    println!("metadata v{}", metadata.version());
    if let Some(version) = extracted.unity_version.as_deref() {
        println!("Unity {version}");
    }
    println!("{} packet-like protobuf types", catalog.packets.len());
    println!("wrote {}", output.display());
    Ok(())
}

fn parse_packets(metadata_path: PathBuf, output: PathBuf) -> Result<()> {
    fs::create_dir_all(&output)?;
    let data = fs::read(&metadata_path)
        .with_context(|| format!("reading {}", metadata_path.display()))?;
    let metadata = MetadataV39::parse(data)?;
    let catalog = PacketCatalog::from_metadata(&metadata)?;
    write_packet_outputs(&output, &metadata, &catalog, None)?;
    println!("recovered {} packet-like types", catalog.packets.len());
    Ok(())
}
