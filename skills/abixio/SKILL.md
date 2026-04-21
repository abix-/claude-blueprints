---
name: abixio
description: AbixIO Rust S3-compatible erasure-coded object server. Use when working on the abixio core (storage tiers, EC, write pool, log store, TLS, S3 protocol) or the abixio-ui benchmark harness, when running abixio-ui bench, or when updating benchmark docs.
version: "2.0"
updated: "2026-04-12"
---

# abixio

S3-compatible Rust object server. Single binary, erasure-coded storage, log-structured small-write tier, mmap GET. Built and benchmarked on Windows 10 against MinIO and RustFS.

## Repos and binaries

- **Core repo**: `C:\code\abixio` (binary crate `abixio = 0.1.0`, edition 2024)
- **UI / benchmark repo**: `C:\code\abixio-ui` (binary crate `abixio-ui = 0.2.0`, depends on abixio as library)
- **Workspace target dir**: `C:\code\endless\rust\target\release\abixio.exe` (abixio compiles into the endless workspace target). The binary resolver in `abixio-ui/src/bench/servers.rs` checks `ABIXIO_BIN` first, then the workspace path, then `C:\code\abixio\abixio.exe` as fallback.

## Build

ALWAYS use `k3sc cargo-lock` (CLAUDE.md forbids bare `cargo`).

```bash
# from C:\code\abixio
k3sc cargo-lock build --release
# binary lands at C:\code\endless\rust\target\release\abixio.exe
```

## Run abixio standalone

```bash
abixio.exe --listen 127.0.0.1:10000 --volumes /path/to/data1[,/path/to/data2,...]
# with TLS:
abixio.exe --listen 127.0.0.1:10000 --volumes /tmp/d1 \
    --tls-cert /tmp/tls-cert.pem --tls-key /tmp/tls-key.pem
# auth: ABIXIO_ACCESS_KEY / ABIXIO_SECRET_KEY env vars (or --no-auth)
# write cache: --write-cache 256 (default, MB), --write-cache 0 (disabled)
# write tier: --write-tier file|log|pool
```

## Storage tier overview

The hot write path has three tiers, picked by object size:

- **log store** (`<= 64 KB`): pre-opened segment file, append + HashMap index, mmap'd for GET
- **write pool** (`64 KB - 10 MB`): pre-opened temp file pool, concurrent data + meta write, async rename worker
- **file tier** (`> 10 MB`): direct `mkdir + File::create + write + close`

Write cache (RAM, DashMap) is a separate axis. `--write-cache <MB>` controls it. 0 disables.

Docs: `docs/write-path.md`, `docs/write-pool.md`, `docs/write-log.md`, `docs/write-cache.md`, `docs/layer-optimization.md`.

## Benchmarks

All benchmarks live in `abixio-ui/src/bench/`. Run via `abixio-ui bench`.

### Running benchmarks

```bash
# full suite
abixio-ui bench

# specific layers/sizes
abixio-ui bench --layers L1 --sizes 4KB
abixio-ui bench --layers L7 --sizes 4KB,10MB --servers abixio --clients sdk

# save JSON + compare against baseline
abixio-ui bench --output results.json
abixio-ui bench --baseline results.json
```

### CLI flags

| Flag | Values | Default |
|---|---|---|
| `--sizes` | `4KB,64KB,10MB,100MB,1GB` | all |
| `--layers` | `L1,L2,L3,L4,L5,L6,L7` | all |
| `--write-paths` | `file,log,pool` | all |
| `--write-cache` | `on,off,both` | both |
| `--servers` | `abixio,rustfs,minio` | all |
| `--clients` | `sdk,aws-cli,rclone` | all |
| `--ops` | `PUT,GET,HEAD,LIST,DELETE` | all |
| `--iters` | number | auto-scaled by size |
| `--tls` | `on,off,both` | on |
| `--output` | path | none |
| `--baseline` | path | none |

### Benchmark file layout

```
src/bench/
    mod.rs              -- run(), CLI arg routing
    stats.rs            -- BenchResult, Stats, JSON output, baseline comparison
    l1_disk.rs          -- raw disk I/O
    l2_compute.rs       -- hashing + RS encode
    l3_storage.rs       -- VolumePool put/get, streaming, multi-disk
    l3_pool_internals.rs -- pool write path internals
    l4_http.rs          -- hyper transport floor
    l5_s3proto.rs       -- s3s protocol overhead
    l6_s3storage.rs     -- s3s + real VolumePool
    l6_stack_breakdown.rs -- 5-stage latency attribution
    l7_e2e.rs           -- full SDK, child servers, competitive
    tls.rs              -- TLS cert generation
    servers.rs          -- AbixioServer, ExternalServer (RustFS/MinIO)
    clients.rs          -- AwsCliHarness, rclone helpers
```

### Fairness rules (from `docs/benchmark-requirements.md`)

1. Same warmup (20 PUT + 3 GET) for every client
2. Same I/O model (disk-backed PUT source / GET sink)
3. Same connection warming (SDK keep-alive vs CLI respawn noted)
4. Same iteration counts per (server, size)
5. Same payload bytes
6. Same auth mode (HTTPS + SigV4 + UNSIGNED-PAYLOAD)
7. Same server config (single-node, 1 disk, NTFS tmpdir, release builds)
8. Roundtrip verification (PUT then GET, size check) before timing

### Harness gotchas

- **Stuck server processes**: a panicking bench leaves child processes alive. `tasklist | grep -iE "abixio|rustfs|minio"` then `taskkill //PID <pid> //F`
- **Three-server disk explosion**: each server's TempDir is dropped before the next starts to avoid ENOSPC
- **AWS CLI v2 required**: `C:\Program Files\Amazon\AWSCLIV2\aws.exe`

### External binaries

| Binary | Default path | Env override |
|---|---|---|
| RustFS | `C:\tools\rustfs.exe` | `RUSTFS_BIN` |
| MinIO | `C:\tools\minio.exe` | `MINIO_BIN` |
| rclone | `C:\tools\rclone.exe` | `RCLONE` |
| AWS CLI | `C:\Program Files\Amazon\AWSCLIV2\aws.exe` | `AWS` |

## TLS gotcha (rustls 0.23 + two crypto providers)

abixio's dep tree pulls in both `aws-lc-rs` and `ring`. rustls 0.23 refuses to auto-pick. Fix is at the top of `main()`:

```rust
tokio_rustls::rustls::crypto::ring::default_provider()
    .install_default()
    .expect("install rustls ring crypto provider");
```

## Windows caveats

- Always use `127.0.0.1`, never `localhost` (Windows DNS adds ~200 ms)
- TCP_NODELAY must be set explicitly
- hyper needs `writev(true)` + `max_buf_size(4MB)` for optimal throughput
- Loopback TCP connect is ~0.2 ms on Windows vs ~0.03 ms on Linux

## Documentation

- `docs/benchmark-requirements.md` -- benchmark spec, file layout, CLI flags, fairness rules
- `docs/benchmarks.md` -- numeric results and matrix tables
- `docs/write-path.md` -- canonical end-to-end PUT flow
- `docs/layer-optimization.md` -- optimization history by layer
- `docs/write-cache.md`, `docs/write-log.md`, `docs/write-pool.md` -- per-tier deep dives
