//! Criterion bench for `hush::compute_suggestions`.
//!
//! Pairs with `bench/compute_suggestions.mjs` - both run the exact
//! same synthetic input, so the reported Rust and JS numbers are
//! directly comparable.
//!
//! ## Realistic scales
//!
//! A heavy Chrome user typically has 20-50 tabs open. Each tab's
//! state is independently capped in `background.js` at:
//!
//! - `MAX_SEEN_RESOURCES = 500`
//! - `MAX_JS_CALLS = 500`
//!
//! so a fully-saturated "busy tab" is 500 + 500. `latestIframes` and
//! `latestStickies` are per-scan snapshots, typically under 20 each.
//! The benches cover:
//!
//! - **light_tab** (100 resources, 50 js-calls, 5 iframes, 5 stickies)
//!   - just-loaded tab, early in session
//! - **heavy_tab** (500 + 500 + 20 + 20, i.e. at the cap ceiling)
//!   - saturated Reddit/Twitter/Gmail-shape tab after sustained use
//! - **50_tabs_of_heavy** - sequentially runs the heavy_tab fixture
//!   50 times to model the aggregate cost if the popup ever got
//!   opened once per tab in rapid succession
//!
//! What's NOT measured: the wasm-bindgen boundary. Criterion runs the
//! native release build; the in-browser WASM runtime typically adds
//! 1.5-2x on top. See `docs/benchmarks.md` for the breakdown.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use hush::types::{
    Allowlist, BehaviorState, Config, IframeHit, JsCall, ReplayVendor, Resource, SiteConfig,
    StickyHit, StickyRect,
};
use hush::{compute_suggestions, SuggestionLayer};

/// Parameters describing a synthetic tab snapshot. Chosen scales
/// below match the production caps; the detector mix roughly mirrors
/// what a Reddit / Twitter / Gmail tab accumulates over a session.
struct TabShape {
    resources: usize,
    js_calls: usize,
    iframes: usize,
    stickies: usize,
}

const LIGHT_TAB: TabShape = TabShape {
    resources: 100,
    js_calls: 50,
    iframes: 5,
    stickies: 5,
};

const HEAVY_TAB: TabShape = TabShape {
    resources: 500,
    js_calls: 500,
    iframes: 20,
    stickies: 20,
};

/// Build a synthetic [`BehaviorState`] for the given shape. Exercises
/// every detector path: beacons, pixels, first-party telemetry,
/// polling, hidden iframes, sticky overlays, canvas-fp, webgl-fp (hot
/// and general), audio-fp, font-fp, listener-added density,
/// replay-global, canvas-draw invisibility.
fn sample_state(shape: &TabShape) -> BehaviorState {
    let scale = shape.resources;
    let host = "site.test".to_string();
    let mut state = BehaviorState {
        page_host: Some(host.clone()),
        ..Default::default()
    };

    // Resources: mix of signal-triggering patterns.
    for i in 0..scale {
        // Beacons to a third-party host (~20%).
        if i % 5 == 0 {
            state.seen_resources.push(Resource {
                url: format!("https://tracker.test/beacon?i={i}"),
                host: "tracker.test".into(),
                initiator_type: "beacon".into(),
                transfer_size: 0,
                duration: 5,
                start_time: i as i64,
                reporter_frame: None,
            });
            continue;
        }
        // Tracking pixels (~20%).
        if i % 5 == 1 {
            state.seen_resources.push(Resource {
                url: format!("https://ads.test/p{i}.gif"),
                host: "ads.test".into(),
                initiator_type: "img".into(),
                transfer_size: 43,
                duration: 2,
                start_time: i as i64,
                reporter_frame: None,
            });
            continue;
        }
        // First-party telemetry subdomain (~20%).
        if i % 5 == 2 {
            state.seen_resources.push(Resource {
                url: format!("https://log.site.test/h{i}"),
                host: "log.site.test".into(),
                initiator_type: "fetch".into(),
                transfer_size: 150,
                duration: 10,
                start_time: i as i64,
                reporter_frame: None,
            });
            continue;
        }
        // Polling endpoint (~20%): repeated URL with varying noise.
        if i % 5 == 3 {
            state.seen_resources.push(Resource {
                url: "https://api.test/poll".into(),
                host: "api.test".into(),
                initiator_type: "fetch".into(),
                transfer_size: 50,
                duration: 5,
                start_time: (i as i64) * 1000, // seconds apart
                reporter_frame: None,
            });
            continue;
        }
        // Misc first-party noise to fill the tab (~20%).
        state.seen_resources.push(Resource {
            url: format!("https://site.test/asset{i}.js"),
            host: "site.test".into(),
            initiator_type: "script".into(),
            transfer_size: 8192,
            duration: 15,
            start_time: i as i64,
            reporter_frame: None,
        });
    }

    // Hidden iframes from known-trackery host + one legit.
    for i in 0..shape.iframes {
        state.latest_iframes.push(IframeHit {
            src: format!("https://{}.ads.test/frame{i}", if i % 2 == 0 { "a" } else { "b" }),
            host: format!("{}.ads.test", if i % 2 == 0 { "a" } else { "b" }),
            reasons: vec!["display:none".into(), "1x1 size".into()],
            width: 1,
            height: 1,
            outer_html_preview: "<iframe ...>".into(),
            reporter_frame: None,
        });
    }

    // Sticky overlays.
    for i in 0..shape.stickies {
        state.latest_stickies.push(StickyHit {
            selector: format!("div.popup-{i}"),
            coverage: 45,
            z_index: 9999,
            rect: StickyRect { w: 400, h: 300 },
            reporter_frame: None,
        });
    }

    // main-world js-calls: canvas-fp + webgl-fp hot + font-fp +
    // replay-global + listener density + raf-waste (all at once).
    for i in 0..shape.js_calls {
        let stack = vec![format!("at x (https://fp.test/fp.js:{}:1)", i)];
        let kind = match i % 6 {
            0 => "canvas-fp",
            1 => "webgl-fp",
            2 => "font-fp",
            3 => "audio-fp",
            4 => "listener-added",
            _ => "canvas-draw",
        };
        let mut call = JsCall {
            kind: kind.into(),
            t: "2026-04-19T12:00:00.000Z".into(),
            stack,
            ..Default::default()
        };
        match kind {
            "webgl-fp" => call.hot_param = i % 2 == 0,
            "font-fp" => {
                call.font = Some(format!("12px font-{i}"));
                call.text = Some("probe".into());
            }
            "listener-added" => call.event_type = Some("mousemove".into()),
            "canvas-draw" => {
                call.op = Some("fillRect".into());
                call.visible = Some(i % 3 != 0);
                call.canvas_sel = Some(format!("canvas#c{}", i % 3));
            }
            _ => {}
        }
        state.js_calls.push(call);
    }

    // One replay-global hit to exercise the vendor-dict path.
    state.js_calls.push(JsCall {
        kind: "replay-global".into(),
        t: "2026-04-19T12:00:00.000Z".into(),
        vendors: vec![ReplayVendor {
            key: "_hjSettings".into(),
            vendor: "Hotjar".into(),
        }],
        ..Default::default()
    });

    state
}

fn seed_config() -> Config {
    let mut config = Config::new();
    config.insert(
        "site.test".into(),
        SiteConfig {
            block: vec!["||already-blocked.test".into()],
            ..Default::default()
        },
    );
    config
}

fn bench_compute(c: &mut Criterion) {
    let allowlist = Allowlist::default();
    let config = seed_config();

    let mut group = c.benchmark_group("compute_suggestions");
    for (label, shape) in [("light_tab", &LIGHT_TAB), ("heavy_tab", &HEAVY_TAB)] {
        let state = sample_state(shape);
        group.throughput(Throughput::Elements(
            (shape.resources + shape.js_calls) as u64,
        ));
        group.bench_with_input(BenchmarkId::from_parameter(label), &state, |b, state| {
            b.iter(|| {
                let out = compute_suggestions(
                    black_box(state),
                    black_box(&config),
                    black_box(&allowlist),
                );
                black_box(out);
            });
        });
    }
    group.finish();

    // Aggregate scenario: 50-tab browser session where every tab's
    // popup opened in sequence. This is the worst case for a heavy
    // Chrome user and answers "if I walked through all 50 tabs, how
    // much CPU does the engine burn total?"
    let heavy_state = sample_state(&HEAVY_TAB);
    c.bench_function("compute_suggestions/50_tabs_of_heavy", |b| {
        b.iter(|| {
            for _ in 0..50 {
                let out = compute_suggestions(
                    black_box(&heavy_state),
                    black_box(&config),
                    black_box(&allowlist),
                );
                black_box(out);
            }
        });
    });

    // Sanity: one run at the heavy scale confirms the fixture
    // produces real output. Criterion ignores this block.
    let out = compute_suggestions(&heavy_state, &config, &allowlist);
    assert!(!out.is_empty(), "bench fixture produced no suggestions");
    assert!(
        out.iter().any(|s| matches!(s.layer, SuggestionLayer::Block)),
        "no block-layer suggestions in fixture"
    );
    eprintln!("heavy_tab fixture produces {} suggestions", out.len());
}

criterion_group!(benches, bench_compute);
criterion_main!(benches);
