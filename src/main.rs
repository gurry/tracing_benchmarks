#![feature(codeview_annotation)]

use tracelogging as tlg;

// -- TraceLogging provider --
tlg::define_provider!(TLG_PROVIDER, "BenchProvider.TraceLogging");

// -- WPP provider --
wpp::wpp_control_guids!(
    WppBench 84bdb2e9-829e-41b3-b891-02f454bc2bd7 {
        GENERAL,
    }
);

fn main() {
    // Register both providers.
    // SAFETY: This is an EXE — no DLL-unload concern.
    unsafe {
        TLG_PROVIDER.register();
        WppBench::init();
    }

    let iterations = 1_000_000u64;
    let msg = "hello benchmark payload";
    let counter: u32 = 42;

    // -- TraceLogging --
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let _ = std::hint::black_box(tlg::write_event!(
            TLG_PROVIDER,
            "BenchEvent",
            level(Informational),
            keyword(0x1),
            str8("Message", msg),
            u32("Counter", &counter),
        ));
    }
    let tlg_elapsed = start.elapsed();

    // -- WPP --
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        std::hint::black_box(trace!(INFO, GENERAL, "{} {}", msg, counter));
    }
    let wpp_elapsed = start.elapsed();

    WppBench::clean_up();
    TLG_PROVIDER.unregister();

    println!("TraceLogging: {iterations} calls in {tlg_elapsed:?} ({:.1} ns/call)",
        tlg_elapsed.as_nanos() as f64 / iterations as f64);
    println!("WPP:          {iterations} calls in {wpp_elapsed:?} ({:.1} ns/call)",
        wpp_elapsed.as_nanos() as f64 / iterations as f64);
}
