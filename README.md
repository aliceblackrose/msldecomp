# msldecomp

Rust tooling for reverse-engineering **Monster Super League** Unity/IL2CPP builds from an APK/XAPK you already possess.

The current implementation targets the uploaded `Monster Super League_1.0.260722032` Android build and focuses on reproducible static analysis:

- extract `global-metadata.dat` and the requested `libil2cpp.so` ABI from APK/XAPK containers;
- parse Unity 6 **IL2CPP metadata v39** without assuming the old v24/v29 record layout;
- recover generated Google.Protobuf `Req*`, `Rsp*`, and `MsgRsp*` message schemas and their field numbers;
- recover top-level `Request` / `Response` envelope tags (the protocol packet IDs);
- passively decode plaintext protobuf payloads from hex/files;
- optionally generate `dump.cs`, `script.json`, `il2cpp.h`, and related native output using a pinned Rust IL2CPP dumper engine.

It deliberately does **not** include certificate-pinning bypasses, credential extraction, authentication bypasses, or code that sends/replays packets against the live service.

## Findings for MSL 1.0.260722032

| Item | Recovered value |
|---|---:|
| Unity | `6000.3.12f1` |
| IL2CPP metadata | `v39` |
| Type definitions | `25,134` |
| Method definitions | `188,146` |
| Fields | `115,768` |
| Generated request packet schemas | `353` |
| Generated response/message-response schemas | `442` |
| Request-envelope payload tags | `346` |
| Response-envelope payload tags | `401` |

Examples from the actual envelope metadata:

| Tag | Request payload | Response payload |
|---:|---|---|
| 50 | `ReqUserLogin` | `RspUserLogin` |
| 60 | `ReqBattleStart` | `RspBattleStart` |
| 61 | `ReqBattleEnd` | `RspBattleEnd` |
| 117 | `ReqServerTime` | `RspServerTime` |

The full build-specific snapshot is in [`docs/msl-1.0.260722032-packets.json`](docs/msl-1.0.260722032-packets.json).

## Usage

### Inspect an XAPK

```bash
cargo run --release -- inspect \
  "Monster Super League_1.0.260722032_apkcombo.com.xapk" \
  --output out \
  --abi arm64-v8a
```

Outputs:

```text
out/
├── extracted/
│   ├── global-metadata.dat
│   └── libil2cpp.so
├── metadata_summary.json
├── packets.json
├── packet_skeleton.proto
└── REPORT.md
```

`packet_skeleton.proto` preserves recovered field numbers. Types that cannot be proven from metadata alone (especially closed generic `RepeatedField<T>`/map instances) are emitted as `bytes` with their `TypeIndex` in a comment instead of being guessed.

### Parse metadata directly

```bash
cargo run --release -- packets global-metadata.dat --output out
```

### Decode a protobuf payload

Generic wire decode:

```bash
cargo run --release -- wire "0a05616c6963651001"
```

Annotate field numbers with a recovered packet schema:

```bash
cargo run --release -- wire payload.bin --file \
  --schema out/packets.json \
  --packet ReqUserLogin
```

This decoder reads plaintext bytes supplied by you; it does not intercept TLS or connect to MSL servers.

### Full native IL2CPP dump

The optional `native` feature pins [`rodroidmods/il2cpp-dumper-rs`](https://github.com/rodroidmods/il2cpp-dumper-rs) at a known commit with Unity 6/v39 support.

```bash
cargo run --release --features native -- native \
  out/extracted/libil2cpp.so \
  out/extracted/global-metadata.dat \
  --unity-version 6000.3.12f1 \
  --output native-out
```

The native path auto-locates `CodeRegistration` and `MetadataRegistration` in the ELF and then generates the same core reverse-engineering outputs as the Rust dumper engine, including `dump.cs` and C/C++ struct headers.

## v39 notes

Metadata v39 is not layout-compatible with many older IL2CPP parsers. In this build the relevant variable-width indices are:

- `TypeIndex`: 4 bytes
- `TypeDefinitionIndex`: 2 bytes
- `GenericContainerIndex`: 2 bytes
- `ParameterDefinitionIndex`: 4 bytes

Field-number constants use Unity's IL2CPP compressed integer format followed by signed zig-zag decoding. They are **not** protobuf varints inside `global-metadata.dat`.

For direct generated fields in this build, the backing-field `TypeIndex` resolves through the adjacent by-ref slot (`byvalTypeIndex + 1`). Closed generic instances are intentionally left unresolved until native type-registration data is available.

## Repository layout

```text
src/
├── main.rs       # CLI
├── xapk.rs       # APK/XAPK extraction + Unity version scan
├── metadata.rs   # self-contained metadata-v39 reader
├── packets.rs    # protobuf schemas + Request/Response tag maps
├── wire.rs       # passive protobuf wire decoder
└── native.rs     # optional full IL2CPP dump integration

docs/
├── MSL_1.0.260722032.md
└── msl-1.0.260722032-packets.json
```

## Scope

This project is for static analysis, interoperability research, preservation, and debugging of software/data you are authorized to inspect. Game updates can change protobuf schemas, envelope tags, IL2CPP metadata layout, or registration patterns, so regenerate the catalog for each build rather than assuming tags are stable.
