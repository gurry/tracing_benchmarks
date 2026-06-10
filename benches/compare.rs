#![feature(codeview_annotation)]
#![allow(unused)]

//! Criterion benchmarks comparing `tracelogging` vs `wpp` vs `tracing-etw` crate performance.
//!
//! All providers are registered but no ETW session is actively collecting,
//! so we measure the cost of the enabled-check fast-path plus any overhead
//! the macro introduces (field serialization, descriptors, etc.).
//!
//! Run with: cargo +stage1 bench
//!
//! To benchmark with an active ETW listener collecting events, use:
//!   .\bench-with-tracing.ps1

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tracelogging as tlg;
use tracing_subscriber::prelude::*;

tlg::define_provider!(TLG_PROVIDER, "BenchProvider.TraceLogging");

wpp::wpp_control_guids!(
    WppBench 84bdb2e9-829e-41b3-b891-02f454bc2bd7 {
        GENERAL,
    }
);

static TRACING_INIT: std::sync::Once = std::sync::Once::new();

fn register_all() {
    unsafe {
        TLG_PROVIDER.register();
        WppBench::init();
    }

    // tracing-etw uses the tracing subscriber infrastructure, which can only
    // be set once per process. Use Once to guard initialization.
    TRACING_INIT.call_once(|| {
        tracing_subscriber::registry()
            .with(
                tracing_etw::LayerBuilder::new("BenchProvider.TracingEtw")
                    .with_default_keyword(0x1)
                    .build()
                    .unwrap(),
            )
            .init();
    });
}

fn unregister_all() {
    WppBench::clean_up();
    TLG_PROVIDER.unregister();
}


fn bench_fmt_with_single_arg(c: &mut Criterion) {
    register_all();
    let mut group = c.benchmark_group("fmt_with_single_arg");
    let status = -1;

    group.bench_function("tracing", |b| {
        b.iter(|| {
            tracing::event!(
                tracing::Level::INFO,
                status = black_box(status),
                "WdfDriverCreate failed with status {}",
                black_box(status)
            );
        });
    });

    group.bench_function("tracing_etw", |b| {
        b.iter(|| {
            tracing_etw::etw_event!(
                name: "BenchEvent",
                tracing::Level::INFO,
                0x1,
                { status = black_box(status) },
                "WdfDriverCreate failed with status {}",
                black_box(status)
            );
        });
    });

    group.bench_function("tracelogging", |b| {
        b.iter(|| {
            tlg::write_event!(
                TLG_PROVIDER,
                "StrEvent",
                str8("Format", black_box("WdfDriverCreate failed with status {}")),
                i32("Status", &black_box(status)),
            );
        });
    });

    group.bench_function("wpp", |b| {
        b.iter(|| {
            trace!(INFO, GENERAL, "WdfDriverCreate failed with status {}", black_box(status));
        });
    });

    group.finish();
    unregister_all();
}

criterion_group!(
    benches,
    bench_fmt_with_single_arg,
);
criterion_main!(benches);
