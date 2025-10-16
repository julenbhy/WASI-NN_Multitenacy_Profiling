# WASI-NN_Multitenacy_experiments

## Test

Launch the executor on a terminal

    cargo run -p server

Compile a wasm code example

    cargo build-wasm -p hello

Create and run some runtimes

    python python_codes/call.py


# Monitorize multitenancy inference

Launch the executor on a terminal

    cargo run -p server --release

Compile a wasm code example

    cargo build-wasm -p llm

Create and run some runtimes

    python python_codes/monitor.py