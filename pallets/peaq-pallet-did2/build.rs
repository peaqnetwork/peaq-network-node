//! Build script: keeps proto files in sync with DID spec types and compiles them to Rust.
//!
//! The prost-generated Rust code is written to `OUT_DIR` (inside `target/`, gitignored)
//! and pulled in at compile time via `include!` in `src/proto_gen.rs`.

fn main() {
    // Re-run whenever the DID spec type definitions or proto files change.
    println!("cargo:rerun-if-changed=src/did_spec/v0.rs");
    println!("cargo:rerun-if-changed=src/did_spec/mod.rs");
    println!("cargo:rerun-if-changed=proto/did_spec_v0.proto");
    println!("cargo:rerun-if-changed=proto/did_spec.proto");

    // Compile proto → Rust. Output lands in OUT_DIR (target/…), never in the repo.
    prost_build::Config::new()
        .compile_protos(&["proto/did_spec_v0.proto", "proto/did_spec.proto"], &["."])
        .expect("prost_build failed to compile proto files");
}
