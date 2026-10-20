test:
    cargo test --manifest-path winterland_shovel/Cargo.toml
alias t := test
build:
    cargo check --manifest-path winterland_app/rust/Cargo.toml

check-with-frb:
    cargo check --manifest-path winterland_shovel/Cargo.toml --features flutter_rust_bridge
alias cwf := check-with-frb

frb-generate:
    cd winterland_app && flutter_rust_bridge_codegen generate
alias fg := frb-generate
