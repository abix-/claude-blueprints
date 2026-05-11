---
name: abixio
description: AbixIO Rust S3-compatible erasure-coded object server. Single binary, two-tier write path (WAL + file), per-object FTT, multi-volume + multi-node, openraft control plane. Lives in [`abix-/abixio`](https://github.com/abix-/abixio) (server) and [`abix-/abixio-ui`](https://github.com/abix-/abixio-ui) (CLI + benchmark harness). Use when working on the core (storage, EC, WAL, write cache, read cache, TLS, S3 protocol, raft, metrics, lifecycle, healing), the abixio-ui CLI / benchmark harness, or the docs.
version: "3.0"
updated: "2026-05-11"
---
# abixio

S3-compatible Rust object server. Single binary, erasure-coded
storage, write-ahead-log small-object tier, mmap GET, openraft
control plane, optional read + write caches. Built and benchmarked
on Windows against MinIO and RustFS.

> Experimental research project. Not production-ready. Expect
> breaking changes and data loss. Repo [`abix-/abixio`](https://github.com/abix-/abixio) carries the
> warning prominently in its README.

## Repos + where things live

Two public GitHub repos, one workspace:

| Repo                 | Crate            | Role                                                  |
| -------------------- | ---------------- | ----------------------------------------------------- |
| [`abix-/abixio`](https://github.com/abix-/abixio)       | `abixio`         | Server binary + library. Storage, S3 protocol, raft, EC, WAL, caches, healing, lifecycle. |
| [`abix-/abixio-ui`](https://github.com/abix-/abixio-ui)    | `abixio-ui`      | CLI + benchmark harness. Depends on `abixio` as a path dep. |

### Documentation index (in [`abix-/abixio`](https://github.com/abix-/abixio) repo)

`docs/` is authoritative: read these first for whichever subject
you are touching:

| File                              | Authoritative on                                          |
| --------------------------------- | --------------------------------------------------------- |
| `docs/index.md`                   | Overview + status + what works / what's rough             |
| `docs/architecture.md`            | End-to-end design + module map                            |
| `docs/status.md`                  | Scorecard (consensus, clustering, etc.): current %      |
| `docs/write-path.md`              | Canonical PUT flow, tier routing, per-tier timings        |
| `docs/write-wal.md`               | WAL design + perf (zero-copy hot path, mmap segment, ack-after-append) |
| `docs/write-cache.md`             | RAM write cache, peer replication semantics               |
| `docs/storage-layout.md`          | On-disk layout, volume format version                     |
| `docs/per-object-ec.md`           | Per-object FTT, 1+0 fast path on single disk              |
| `docs/healing.md`                 | Background healing, MRF queue, integrity scanner          |
| `docs/cluster.md`                 | Node identity, membership, quorum, fencing                |
| `docs/raft.md`                    | openraft control plane design + status                    |
| `docs/admin-api.md`               | `/_admin/*` + `/raft/*` endpoints                         |
| `docs/metrics.md`                 | Prometheus `/_admin/metrics` surface                      |
| `docs/s3-compliance.md`           | 41-of-72 S3 API coverage matrix                           |
| `docs/conditional-requests.md`    | If-Match / If-None-Match / If-Modified-Since semantics    |
| `docs/versioning.md`              | Bucket versioning, version response headers               |
| `docs/tagging.md`                 | Object tagging                                            |
| `docs/multipart-upload.md`        | Multipart UploadPart / CompleteMultipartUpload            |
| `docs/presigned-urls.md`          | SigV4 presigned URL handling                              |
| `docs/encryption.md`              | At-rest / in-transit encryption                           |
| `docs/error-responses.md`         | S3-compatible error XML envelopes                         |
| `docs/bucket-policy.md`           | Bucket policy enforcement (kovarex #7)                    |
| `docs/distributed-placement-testing.md` | Cross-node placement test harness                   |
| `docs/layer-optimization.md`      | Layer-by-layer optimization history                       |
| `docs/benchmarks.md`              | Numeric results, canonical-stack tables, competitive matrix |
| `docs/benchmark-requirements.md`  | Fairness rules + bench spec                               |
| `docs/comparison.md`              | abixio vs MinIO vs RustFS vs SeaweedFS                    |
| `docs/security-review.md`         | Threat model + kovarex review notes                       |
| `docs/todo.md`                    | Open work, ranked                                         |
| `docs/img/`                       | Diagrams                                                  |

### Source layout ([`abix-/abixio`](https://github.com/abix-/abixio))

| Module                         | Subject                                                |
| ------------------------------ | ------------------------------------------------------ |
| `src/main.rs`                  | CLI parse + service wiring (TLS provider install here) |
| `src/server/*`                 | HTTP server, s3s integration                           |
| `src/storage/local/*`          | LocalVolume, write_shard, open_shard_writer routing    |
| `src/storage/wal/*`            | WAL: append-only segment + materialize worker          |
| `src/storage/cache/write_cache.rs` | RAM write cache (DashMap) + peer replication       |
| `src/storage/cache/read_cache.rs`  | RAM read cache, warm-on-write                      |
| `src/storage/needle.rs`        | Zero-alloc `serialize_into` to mmap                    |
| `src/storage/segment.rs`       | mmap-backed segment writes                             |
| `src/storage/healing/*`        | MRF queue + integrity scanner                          |
| `src/storage/lifecycle.rs`     | Lifecycle rule enforcement                             |
| `src/cluster/*`                | Identity, membership, quorum, internode HTTP           |
| `src/raft/*`                   | openraft TypeConfig + log/fsm storage adapters + AbixioRaft runtime + RaftNetwork |
| `src/metrics/*`                | Prometheus registry + per-subject collectors           |
| `src/admin/*`                  | `/_admin/*` route handlers                             |

### Source layout ([`abix-/abixio-ui`](https://github.com/abix-/abixio-ui))

| Module                              | Subject                                       |
| ----------------------------------- | --------------------------------------------- |
| `src/bench/mod.rs`                  | `run()`, CLI arg routing                      |
| `src/bench/stats.rs`                | BenchResult, Stats, JSON output, baseline compare |
| `src/bench/l1_disk.rs`              | L1 HTTP ingress (renamed. PUT flow order)   |
| `src/bench/l2_compute.rs`           | L2 S3 protocol (in-memory pipe, no TCP, isolated) |
| `src/bench/l3_storage.rs`           | L3 storage pipeline (VolumePool, write-path × cache matrix) |
| `src/bench/l4_http.rs`              | L4 compute (hashing + RS encode)              |
| `src/bench/l5_s3proto.rs`           | L5 raw disk                                   |
| `src/bench/l6_s3storage.rs`         | L6 s3s + real VolumePool                      |
| `src/bench/l7_e2e.rs`               | L7 full SDK + child servers + competitive     |
| `src/bench/tls.rs`                  | TLS cert generation                           |
| `src/bench/servers.rs`              | AbixioServer, ExternalServer (RustFS/MinIO)   |
| `src/bench/clients.rs`              | AwsCliHarness, rclone helpers                 |

## Current architecture (the headline: circa 2026-05)

### Two write tiers (the WAL refactor)

The old three-tier model (log_store + write_slot_pool + file) is
**gone**. `log_store.rs` and `write_slot_pool.rs` were deleted;
all 19 techniques carried over or improved. Today:

- **WAL tier (`--write-tier=wal`, default)**: append-only log
  with background materialize to the file tier. Zero-copy hot
  path: `MaterializeRequest` uses `Arc<str>`; worker reads from
  mmap instead of receiving data copies. WAL append is ~3 µs at
  4 KB in release (was 26 µs before mmap-write + try_send +
  ack-after-append optimizations). Versioned + has heal path
  (kovarex #4 enforced).
- **File tier (`--write-tier=file`)**: direct
  `mkdir + File::create + write + close`. Default switched to WAL
  per perf wins; file tier remains for ablation + large objects.

`WalShardWriter` is **dual-mode**: small PUTs (<= 64 KB) buffer in
RAM, large PUTs (>= 1 MB) promote to streaming so disk write
overlaps network receive. 1 GB L7 PUT unsigned went 317 → 449
MB/s through these refactors.

`encode_and_write` pipelines the large-PUT path via `mpsc(8)` +
spawned writer task, gated on `content_length >= 1MB`. Matches
rustfs's mpsc pattern + minio's ring-buffer-per-writer design.

### Read + write caches (RAM)

- **Write cache**. DashMap, `--write-cache <MB>` flag (default
  256 MB, 0 disables). Peer replication enforced per kovarex #3.
- **Read cache**. DashMap, `--read-cache on/off`. Warmed on
  write so small-object GETs after PUT hit RAM. 4 KB L7 GET p50
  went 1.5 ms → 807 µs at wal+wc+rc canonical stack (1.86x).
  Cold GET populate uses `Bytes::from_owner` of the existing
  mmap view (no copy).

### Canonical stack

`wal + wc + rc` is the official abixio stack. Benchmarks default
to this; the 8-config ablation matrix is opt-in via explicit
multi-value flags. Small-object regime (<= 64 KB) flows through
`wc/rc/WAL`; file tier is the > 64 KB path. Canonical 4 KB L7
GET: 1.1 ms cold / 861 µs hot. Wins small-object PUT vs RustFS
1.9x and MinIO 1.5x.

### 1+0 fast path

Single-disk deployments skip RS encode and shard buffers. EC
bypass is cluster-aware (based on volume count), not a config
flag. Documented as a design gap that was closed.

### openraft control plane

`openraft` integration is shipped behind `--raft-enable`:

- **Storage** (`raft::storage`): on-disk log store + snapshot
  store + vote persistence with atomic writes and index rebuild.
  Implements openraft storage-v2 traits (log storage + fsm
  storage adapters).
- **Network** (`raft::network`). RaftNetwork + RaftNetworkFactory
  over the existing internode HTTP layer.
- **Runtime** (`raft::AbixioRaft`): single-node bootstrap /
  submit / read works (integration test passes). Wired into
  `main.rs` behind `--raft-enable` + `--raft-bootstrap` +
  `--raft-id` + `--raft-dir`. Default raft id is blake3-derived.
- **HTTP**: storage server exposes `/raft/append-entries`,
  `/raft/vote`, `/raft/install-snapshot`. Admin server exposes
  `/raft/peers`, `/raft/primary`, `/raft/bootstrap`, `/raft/join`,
  `/raft/leave`, `/raft/snapshot`.

`docs/status.md` Consensus bumped 0 → 5/10, Clustering 5 → 6/10.

### Observability

`/_admin/metrics` Prometheus endpoint with request latency
histograms + cache / WAL / disk / cluster / heal / lifecycle
counters. `docs/metrics.md` is the surface reference. Plus a
per-layer `server-timing` response header for on-demand request
profiling.

### Graceful shutdown

Drains HTTP → flushes cache → drains pool renames → exits.

### S3 surface

41 of 72 S3 API operations implemented; conditional requests
(If-Match / If-None-Match / If-Modified-Since /
If-Unmodified-Since) wired through s3s DTOs; versioning response
headers wired; multipart, presigned URLs, tagging, bucket policy,
lifecycle live. The `mc` throughput gap was closed (354 → 1476
MB/s, 4.2x) by reporting exact `remaining_length` so hyper uses
`Content-Length` not chunked encoding.

`s3_integration.rs` was split into 9 test files by category.

### kovarex review enforcement

These items from the kovarex review have landed; do not regress:
- **#3** write cache peer replication
- **#4** WAL versioned + heal path
- **#7** lifecycle + bucket policy enforcement
- **#8.1** read cache (warmed on write)
- Volume format version assertion
- Unwrap plague closed: 530 of 533 were test-only; the sole
  production unwrap became `let-else`.

## Build + run

ALWAYS use `k3sc cargo-lock` (the project's cargo wrapper);
the user's global CLAUDE.md forbids bare `cargo`.

```bash
# build (from abixio repo root)
k3sc cargo-lock build --release

# run single node, two disks
abixio --listen 127.0.0.1:10000 --volumes /path/data{1...2}

# TLS
abixio --listen 127.0.0.1:10000 --volumes /path/data1 \
       --tls-cert /path/tls-cert.pem --tls-key /path/tls-key.pem

# auth via env: ABIXIO_ACCESS_KEY / ABIXIO_SECRET_KEY  (or --no-auth)

# write tier + caches
abixio --write-tier wal --write-cache 256 --read-cache on

# raft enable (single-node bootstrap)
abixio --raft-enable --raft-bootstrap --raft-id <id> --raft-dir /path/raft
```

`{N...M}` expands sequential ranges in `--volumes` and `--nodes`
(see `docs/cluster.md`).

### TLS gotcha (do not regress)

abixio's dep tree pulls both `aws-lc-rs` and `ring`. rustls 0.23
refuses to auto-pick. Top of `main()`:

```rust
tokio_rustls::rustls::crypto::ring::default_provider()
    .install_default()
    .expect("install rustls ring crypto provider");
```

### Windows perf caveats

- Always use `127.0.0.1`, never `localhost` (DNS adds ~200 ms).
- `TCP_NODELAY` must be set explicitly.
- hyper needs `writev(true)` + `max_buf_size(4 MB)` for optimal
  throughput.
- Loopback TCP connect is ~0.2 ms on Windows vs ~0.03 ms on Linux.

## Benchmarks

All benchmarks live in [`abix-/abixio-ui`](https://github.com/abix-/abixio-ui) under `src/bench/`. Run
via `abixio-ui bench`. Layer naming follows PUT flow order:
L1=HTTP, L2=S3, L3=storage, L4=compute, L5=disk, L6=S3+storage,
L7=e2e.

```bash
abixio-ui bench                                      # full suite, canonical stack
abixio-ui bench --layers L7 --sizes 4KB,1GB         # specific
abixio-ui bench --output results.json
abixio-ui bench --baseline results.json             # diff vs saved
```

### CLI flags

| Flag             | Values                              | Default                 |
| ---------------- | ----------------------------------- | ----------------------- |
| `--sizes`        | `4KB,64KB,10MB,100MB,1GB`           | all                     |
| `--layers`       | `L1..L7`                            | all                     |
| `--write-paths`  | `file,wal`                          | `wal` (canonical)       |
| `--write-cache`  | `on,off,both`                       | `on` (canonical)        |
| `--read-cache`   | `on,off,both`                       | `on` (canonical)        |
| `--servers`      | `abixio,rustfs,minio`               | all                     |
| `--clients`      | `sdk,aws-cli,rclone`                | all                     |
| `--ops`          | `PUT,GET,HEAD,LIST,DELETE`          | all                     |
| `--iters`        | number                              | auto-scaled by size; cap 1000 |
| `--tls`          | `on,off,both`                       | `on`                    |
| `--tmp-dir`      | path                                | Defender-excluded tmp   |
| `--disks`        | number                              | 1                       |
| `--output`       | path                                | none                    |
| `--baseline`     | path                                | none                    |

Pre-run guards: wipe `--tmp-dir` contents + check free space
(`GetDiskFreeSpaceExW`); abort if < 20× max-size (floor 4 GB).

### Fairness rules (must hold)

1. Same warmup (20 PUT + 3 GET) for every client.
2. Same I/O model (disk-backed PUT source / GET sink).
3. Same connection warming (SDK keep-alive vs CLI respawn noted).
4. Same iteration counts per (server, size).
5. Same payload bytes.
6. Same auth mode (HTTPS + SigV4 + UNSIGNED-PAYLOAD).
7. Same server config (single-node, NTFS tmpdir, release builds).
8. Roundtrip verification (PUT then GET, size check) before timing.

### Harness gotchas

- Panicking benches leave child processes alive. Kill via
  `tasklist | grep -iE "abixio|rustfs|minio"` then `taskkill /F`.
- Each server's TempDir is dropped before the next starts to
  avoid ENOSPC on three-server runs.
- AWS CLI v2 required.
- External binaries default paths plus env overrides:
  `RUSTFS_BIN`, `MINIO_BIN`, `RCLONE`, `AWS`.

## Session etiquette

- Public repo. Generic content: no machine-specific paths, no
  user-identifiable seeds.
- Always `k3sc cargo-lock` (never bare `cargo`).
- Always read the authoritative `docs/<subject>.md` before
  changing that subject; the docs are the spec.
- `docs/todo.md` carries the priority list (kovarex review +
  foundations before features).
- ASCII source/docs/commits; commits lowercase, push immediately.
- Each tier doc owns its perf numbers; don't duplicate perf
  claims into architecture.md or comparison.md.
