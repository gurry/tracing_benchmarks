# Tracing Benchmarks

A benchmark project for comparing performance across different tracing technologies.

## Running Benchmarks

Run from an elevated PowerShell prompt:

```powershell
.\run_benchmarks.ps1
```

Run without starting ETW listeners:

```powershell
.\run_benchmarks.ps1 -NoListeners
```

The script loads the MSVC environment if needed, starts ETW trace sessions unless `-NoListeners` is specified, runs `cargo bench`, and prints results.

### Prerequisites

Make sure you have the custom Rust compiler that implements the `codeview_annotation` intrinsic installed on your machine as a toolchain named `stage1`.