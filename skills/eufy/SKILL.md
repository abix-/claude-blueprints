---
name: eufy
description: "eufy SoloCam S220 local capture, motion detection, and event records. Rust capture/watcher/thick-client + eufy-security-ws Node server, no HomeBase, no cloud in the video path. Lives in the private eufy repo. Use when working on the capture tool, the detector, zones, the records/index system, the operator UI, or debugging the camera's P2P behavior."
user-invocable: false
version: "1.7"
---
# eufy

Local pipeline for a eufy SoloCam S220 (model T8134, standalone
battery WiFi camera): hold the livestream open during watch windows,
record everything, run our own pixel-diff motion detection, cut
per-event clips. Built because the camera's own PIR+AI pipeline
drops most small-animal events (camera self-reports ~275 detected vs
~56 recorded over 3 days). Device serials, LAN addresses, and session
specifics live in the repo's docs/eufy-solocam-s220.md, not here.

## Architecture

- **eufy-security-ws** (Node, npm-installed at repo root): the
  community reverse-engineered protocol library. Logs into a eufy
  account (cloud auth), opens local P2P to the camera, exposes a
  websocket API on 127.0.0.1:3000. Everything on our side is Rust.
- **capture/** (binary `eufy-capture`): websocket client. One-shot
  capture, `--watch` (segments back to back), `--replay` (re-run
  detection over a recording), `--ui` (thick client). Modules:
  main (session + watch loop), detect (pixel-diff detector),
  records (clips/thumbnails/index/retention), status (shared
  LiveStatus + log ring), ui (Iced app).

## Run

    npx eufy-security-server -c config.json -H 127.0.0.1
    ./target/debug/eufy-capture.exe --ui --zone "<label>:x,y,w,h" ...

- Bind 127.0.0.1 EXPLICITLY: default "localhost" binds IPv6-only.
- ffmpeg on PATH is required (remux, clips, thumbnails, detection
  decode).
- GStreamer MSVC (winget gstreamerproject.gstreamer, per-user
  install; the unified installer INCLUDES devel files) must be on
  PATH for EVERY run of the binary once the UI/player is linked in,
  headless modes included. Missing DLLs = instant silent exit, zero
  output. Builds need PKG_CONFIG_PATH=<gst-root>/lib/pkgconfig.
- Build via `k3sc cargo-lock`; the repo pins its own target-dir in
  .cargo/config.toml (without it the binary lands in the shared
  cache and gets lost).

## Camera P2P doctrine (hard-won, do not relearn)

- **The sleep race rules everything.** The battery camera answers
  P2P commands reliably only while a livestream holds the connection
  open. Idle P2P closes after ~30-45s ("saving battery") and async
  replies die with it. Any database/query command: start a stream
  first, then send.
- **Saved-clip retrieval is a firmware dead end.** The local
  database query answers returnCode 0 with data:[] on modern
  firmware (upstream eufy-security-ws issue 545, client issue 715).
  Clips DO download by exact path, but arrive encrypted per-clip;
  the cipher id is only delivered in the push notification at record
  time, and pushes do not reach the library reliably (Mega/v6
  migration). Do not rabbit-hole here; the pipeline does not need it.
- **station.get_commands is a lie.** It reads a static per-device
  table inside eufy-security-client, not the camera.
- **The bundled test client prints NOTHING without -v** in single
  command mode. Every silent run measured nothing.
- **Pin local P2P**: config.json p2pConnectionSetup: 1 (ONLY_LOCAL;
  2 = QUICKEST races the cloud relay and the relay wins) plus
  stationIPAddresses mapping serial to LAN IP.
- **Timezones**: server logs are UTC; camera OSD and on-camera
  filenames are local time.
- Shared (second) accounts see NO cloud event history; owner account
  sees this camera's cloud ledger as empty anyway (standalone cams
  do not populate the v2 event endpoints).
- eufy is sunsetting the legacy API this stack rides on; keep the
  detection/records side decoupled so only the stream source swaps.

## Detection doctrine

- Defaults were tuned on a LIVE squirrel recording and the shipped
  originals missed it entirely; do not "round up" thresholds.
  recordings/baseline-squirrel.h264 is the regression baseline:
  --replay it after any detector change; expect exactly 1 event
  starting ~80s, quiet first 80s.
- **Zones, not thresholds, separate animals from vegetation.** Tree
  sway peaks ~10x a squirrel's signal; grass sways too. Zones are
  labeled rects (frame fractions); the area threshold divides by
  watched pixels so its meaning is stable. Events report their
  dominant zone. AUTO zones (--auto-zones): the vision model maps
  the scene from a 4x3 tile grid of the latest frame (patio/feeder
  tiles wanted; fence/tree/grass excluded) into zones.json; a big
  frame change vs zones-reference.jpg (camera moved) remaps loudly,
  night IR never triggers. Every recording is stamped with its
  capture zones (capture-<epoch>.zones.json) and every event row
  carries them, so replay/rescan use the zones the video was
  RECORDED under; camera moves cannot corrupt history.
- Color aware diff (--color-aware-diff): RGB decode, per-channel
  mean cancellation, pixel changed on its LARGEST channel delta.
  Chroma swings harder than luma under sunlight shifts: calibrated
  --pixel-delta 18 (15 fired a 9s phantom on a passing cloud, 20+
  ate the squirrel's tail). Night IR has no chroma; degrades to
  luma behavior.
- Solar CANNOT outpace streaming: ~4.5%/h net drain while watching
  even with the charging flag on (measured 2026-07-12, 86%->1%).
  The panel only gains while the camera sleeps; watch windows
  (HH:MM-HH:MM local, windows.json) are the battery lever.
- **Duration, not peak size, separates animals from moths and light
  flicker.** Ledger-verified 2026-07-11: every real creature held
  motion 11s+ (humans 52-72s, squirrels 32s, bird 11s); every moth /
  flicker event died within 2.5s. Peak does NOT separate them: a
  moth near the lens peaks HIGHER than a distant squirrel (1.14% vs
  0.29%). Events shorter than --min-duration (default 2.0s) are
  dropped at event close, loudly ("short motion dropped" log line),
  never silently.
- Uniform lighting shifts (exposure steps, IR flicker; one flash lit
  63% of the frame for 0.27s) cancel out by subtracting the mean
  frame-to-frame brightness change over watched pixels before the
  per-pixel compare; local motion survives the subtraction.
- Raw elementary streams CANNOT be stream-copy cut (parameter sets
  live at the file head; slices have no stream). Live event cuts
  re-encode; segment-end cuts from the mp4 stream-copy.
- Cut thumbnails FROM the cut clip, not the raw: timestamp seeking
  a still-growing raw stream is bitrate-estimated and can overshoot
  the end, exiting 0 with zero frames written.
- The camera stamps no fps; everything assumes 15.

## Records

- Full recordings (capture-<epoch>.h264/.mp4/.motion.txt) prune
  after --keep-hours (default 24). events/ clips + thumbnails are
  kept forever. Files not named capture-*/download-* are never
  pruned (that protects the baseline).
- events/events.jsonl is the ledger; the UI gallery renders every
  row from it (index.html was removed 2026-07-12: the UI does it
  all). Rows carry duration/peak/zone plus bbox (motion bounding
  box), zones (capture-time zone set), detected/confidence/votes,
  and the model's verbatim per-frame descriptions (the audit trail
  that proves keyword gaps). The records layer is idempotent
  (existing clip = skip), so re-detection only ever adds.
- Audit pattern: replay recordings on scratch COPIES without mp4
  siblings for detection-only reports (no clips cut into the
  ledger).

## Thick client

Iced 0.14 + iced_video_player (GStreamer) in ONE application with
the watcher on a background thread (jbot shape). Tabs: dashboard
(health strip, disk counters, battery drain + ETA, full-ledger
gallery), video (ONE surface for live + event clips + recordings;
a per-event panel shows timestamp/duration/peak/verdict with
confidence, vote counts, and the model's verbatim per-frame
descriptions; a "show zones" toggle overlays the zones THAT video
was captured under, 16:9 letterbox-corrected), config (zone
preview, full settings editor, watch-windows field, rescan button),
log (datetime ring, newest first). The rescan button drops
rescan.request; the process owning the ledger (the service in
viewer mode) picks it up within 2s: remaps auto zones
UNCONDITIONALLY (a bad map must not survive a rescan; night keeps
zones with a log line), re-detects kept recordings each with its
stamped zones, and clears all verdicts so everything re-identifies.
Startup recovery: any raw recording without a motion.txt sibling
heals on launch.

Live config files (the pattern: the UI writes, the watcher re-reads
EVERY segment, zero restarts): config.json (all detection/classify
knobs; overrides CLI flags), windows.json (watch windows),
zones.json (activity zones, auto or hand). CLI flags in service.json
are seeds/fallbacks only. settings.json stays watcher-WRITTEN
display state that seeds the viewer.

Vision identification: moondream (1.8B) via local ollama. Per event,
N frames (config dropdown, default 11; odd so a two-way vote cannot
tie) spread evenly across the motion span; each slot sends its
SHARPEST frame (variance-of-Laplacian inside the motion bbox, one
gray decode pass; blur loses to a crisp frame nearby), cropped to
the event's motion bounding box (pad 25%, floor 20% of frame; crop
took a squirrel from 40% full-frame confidence to 10/10 votes).
Every frame gets a verdict by word-boundary keyword match (plurals
auto-match; wide vocabulary: bird species, rat/mouse/rodent,
raccoon, possum, rabbit, deer, fox, insect words, and "shadow"
ordered after the animals so a bird's shadow counts as bird); the
most common non-"other" label wins, confidence = winner's share of
all votes. Rows without a bbox get one derived from the clip
itself, so nothing classifies full-frame. Operator-locked doctrine:
the model STARTS, does the thing, STOPS (keep_alive "0s", the
STRING form; integer 0 does not unload on old ollama) and inference
is NEVER enabled without measuring GPU placement first. The 7B
default-config attempt silently ran 0/29 layers on GPU (pure CPU, 8
threads) and degraded the whole machine. moondream ignores one-word
instructions; it DESCRIBES and the label is extracted by
word-boundary matching (bare substring turned "scattered" into a
cat). Hard opt-in: --classify.

UI language: config-tab dropdown (English/Spanish), persisted in
recordings/ui.json; strings are (en, es) pairs at call sites,
accented Spanish via ASCII \u escapes.

## Session monitoring pattern (proven, reusable)

When babysitting the watcher from a Claude session: launch the
supervisor loop as a background shell (its stdout goes to a task
output file), then arm a persistent Monitor tailing that file:

    tail -f <task-output-file> | grep --line-buffered -E \
      "motion |event clip|battery|recover|classif|app crashed|segment failed|no video data|panicked"

- --line-buffered is MANDATORY: without it grep holds ~4KB and events
  arrive late or die in the buffer when the pipe is killed.
- The alternation must cover FAILURE signals, not just wins: silence
  looks identical to "still running".
- Re-arm after every app relaunch: the output file is per-task.
- Supervisor loop shape (crash-restart, clean-exit break):
  `while true; do app; code=$?; [ $code -eq 0 ] && break; sleep 3; done`
  A hard kill reads as a crash and restarts the OLD binary: stop the
  supervisor task BEFORE taskkill when rebuilding, or the relaunched
  app holds the exe lock and the build fails with os error 5.

## Deployment (built + live-verified 2026-07-11)

The "eufy-watcher" Windows service is the production shape: ONE exe
(native windows-service crate), auto-start, LocalSystem, supervising
the node protocol server as a restartable child and running the
watch stack headless. It writes recordings/status.json every 2s and
finalizes the in-progress segment on graceful stop. Install from an
elevated shell at the repo root, FLAGS BEFORE THE SUBCOMMAND:

    eufy-capture.exe --classify --zone "..." service install
    sc start eufy-watcher

Config = service.json (captured flag vector; edit + sc stop/start).
The service's registry Environment value carries GStreamer + ffmpeg
+ node paths (LocalSystem does not inherit user PATH). --ui detects
a fresh status.json and becomes a VIEWER attached to the service
(no second capture; rescan disabled: the ledger lock is
per-process). Rebuilds: sc stop -> build -> sc start (stop also
frees the exe lock). Tray-icon minimize for the viewer is
operator-requested and still pending.

Gotchas paid for once: Iced 0.14 .theme() with a closure fails
"implementation of Fn is not general enough" (pass a fn item).
latest-frame.jpg must load via image::Handle::from_bytes (renderer
caches per handle; the file changes under a fixed name). playbin has
NO tcp:// URI handler: the live view hand-builds tcpclientsrc !
h264parse ! avdec_h264 into the crate's NV12 appsink via
from_gst_pipeline, and a hand-built pipeline MUST call
gstreamer::init() first (Video::new does it internally; skipping it
panics). Re-encoded clips need -g 15 (keyframe per second): the
x264 default keyframe spacing makes scrubbing decode garbage and
once crashed wgpu texture upload.
