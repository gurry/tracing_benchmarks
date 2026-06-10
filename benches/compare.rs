#![feature(codeview_annotation)]
#![allow(unused)]

//! Criterion benchmarks comparing `tracelogging` vs `wpp` crate performance.
//!
//! Both providers are registered but no ETW session is actively collecting,
//! so we measure the cost of the enabled-check fast-path plus any overhead
//! the macro introduces (field serialization, descriptors, etc.).
//!
//! Run with: cargo +stage1 bench
//!
//! To benchmark with an active ETW listener collecting events:
//!   tracelog -start BenchTlg -f tlg.etl -guid *BenchProvider.TraceLogging -level 5 -matchanykw 0xFF
//!   tracelog -start BenchWpp -f wpp.etl -guid 84bdb2e9-829e-41b3-b891-02f454bc2bd7 -level 5 -matchanykw 0xFF
//!   cargo +stage1 bench
//!   tracelog -stop BenchTlg
//!   tracelog -stop BenchWpp

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tracelogging as tlg;

tlg::define_provider!(TLG_PROVIDER, "BenchProvider.TraceLogging");

wpp::wpp_control_guids!(
    WppBench 84bdb2e9-829e-41b3-b891-02f454bc2bd7 {
        GENERAL,
    }
);


fn register_all() {
    unsafe {
        TLG_PROVIDER.register();
        WppBench::init();
    }
}

fn unregister_all() {
    WppBench::clean_up();
    TLG_PROVIDER.unregister();
}


fn bench_enabled_check(c: &mut Criterion) {
    register_all();
    let mut group = c.benchmark_group("enabled_check");

    group.bench_function("tracelogging", |b| {
        b.iter(|| {
            black_box(TLG_PROVIDER.enabled(tlg::Level::Verbose, 0x1));
        });
    });

    group.bench_function("wpp", |b| {
        b.iter(|| {
            black_box(WppBench::STATE.is_enabled(5, WppBench::GENERAL));
        });
    });

    group.finish();
    unregister_all();
}

fn bench_fmt_with_single_arg(c: &mut Criterion) {
    register_all();
    let mut group = c.benchmark_group("fmt_with_single_arg");
    let status = -1;

    group.bench_function("tracelogging", |b| {
        b.iter(|| {
            let _ = black_box(tlg::write_event!(
                TLG_PROVIDER,
                "StrEvent",
                str8("Format", "WdfDriverCreate failed with status {}"),
                i32("Status", &status),
            ));
        });
    });

    group.bench_function("wpp", |b| {
        b.iter(|| {
            black_box(trace!(INFO, GENERAL, "WdfDriverCreate failed with status {}", status));
        });
    });

    group.finish();
    unregister_all();
}

criterion_group!(
    benches,
    // bench_enabled_check,
    bench_fmt_with_single_arg,
);
criterion_main!(benches);
