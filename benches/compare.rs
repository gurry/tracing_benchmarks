#![feature(codeview_annotation)]

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

// ---------------------------------------------------------------------------
// TraceLogging provider
// ---------------------------------------------------------------------------

tlg::define_provider!(TLG_PROVIDER, "BenchProvider.TraceLogging");

// ---------------------------------------------------------------------------
// WPP provider
// ---------------------------------------------------------------------------

wpp::wpp_control_guids!(
    WppBench 84bdb2e9-829e-41b3-b891-02f454bc2bd7 {
        GENERAL,
    }
);

// ---------------------------------------------------------------------------
// Helpers: register/unregister both providers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Benchmarks: no fields
// ---------------------------------------------------------------------------

fn bench_no_fields(c: &mut Criterion) {
    register_all();
    let mut group = c.benchmark_group("no_fields");

    group.bench_function("tracelogging", |b| {
        b.iter(|| {
            let _ = black_box(tlg::write_event!(TLG_PROVIDER, "NoFieldEvent"));
        });
    });

    group.bench_function("wpp", |b| {
        b.iter(|| {
            black_box(trace!(INFO, GENERAL, "NoFieldEvent"));
        });
    });

    group.finish();
    unregister_all();
}

// ---------------------------------------------------------------------------
// Benchmarks: single u32 field
// ---------------------------------------------------------------------------

fn bench_u32_field(c: &mut Criterion) {
    register_all();
    let mut group = c.benchmark_group("u32_field");
    let val: u32 = 42;

    group.bench_function("tracelogging", |b| {
        b.iter(|| {
            let _ = black_box(tlg::write_event!(
                TLG_PROVIDER,
                "U32Event",
                u32("Counter", &val),
            ));
        });
    });

    group.bench_function("wpp", |b| {
        b.iter(|| {
            black_box(trace!(INFO, GENERAL, "{}", val));
        });
    });

    group.finish();
    unregister_all();
}

// ---------------------------------------------------------------------------
// Benchmarks: single string field
// ---------------------------------------------------------------------------

fn bench_str_field(c: &mut Criterion) {
    register_all();
    let mut group = c.benchmark_group("str_field");
    let msg = "hello world benchmark string payload";

    group.bench_function("tracelogging", |b| {
        b.iter(|| {
            let _ = black_box(tlg::write_event!(
                TLG_PROVIDER,
                "StrEvent",
                str8("Message", msg),
            ));
        });
    });

    group.bench_function("wpp", |b| {
        b.iter(|| {
            black_box(trace!(INFO, GENERAL, "{}", msg));
        });
    });

    group.finish();
    unregister_all();
}

// ---------------------------------------------------------------------------
// Benchmarks: multiple mixed-type fields
// ---------------------------------------------------------------------------

fn bench_multi_field(c: &mut Criterion) {
    register_all();
    let mut group = c.benchmark_group("multi_field");
    let name = "benchmark-operation";
    let count: u32 = 100;
    let elapsed: f64 = 3.14159;
    let ok: bool = true;

    group.bench_function("tracelogging", |b| {
        b.iter(|| {
            let _ = black_box(tlg::write_event!(
                TLG_PROVIDER,
                "MultiFieldEvent",
                level(Informational),
                keyword(0x1),
                str8("Name", name),
                u32("Count", &count),
                f64("Elapsed", &elapsed),
                bool8("Success", &ok),
            ));
        });
    });

    group.bench_function("wpp", |b| {
        b.iter(|| {
            black_box(trace!(INFO, GENERAL, "{} {} {} {}", name, count, elapsed, ok));
        });
    });

    group.finish();
    unregister_all();
}

// ---------------------------------------------------------------------------
// Benchmarks: enabled check only
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_enabled_check,
    bench_no_fields,
    bench_u32_field,
    bench_str_field,
    bench_multi_field,
);
criterion_main!(benches);
