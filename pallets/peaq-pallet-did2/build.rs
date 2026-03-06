//! Todo: Description

fn main() {
    // Re-run the build (and thus re-expand generate_proto_file!()) whenever the DID spec
    // type definitions change, so that src/did_spec/did.proto stays in sync automatically.
    println!("cargo:rerun-if-changed=src/did_spec/v0.rs");
}
