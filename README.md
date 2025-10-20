# WASI-NN_Multitenacy_experiments

## Test

Launch the executor on a terminal

    export LIBTORCH=/opt/pytorch-v2.4.0/libtorch-2.4.0-arm/libtorch
    export LD_LIBRARY_PATH=$LIBTORCH/lib:$LD_LIBRARY_PATH
    RUST_LOG=info cargo run -p server

Add the wasm target to Cargo

    rustup target add wasm32-wasip1

Compile a wasm code example

    cargo build-wasm -p hello

Create and run some runtimes

    python python_codes/call.py


## Monitorize multitenancy inference

Download fixtures

    python python_codes/get_fixture.py

Launch the executor on a terminal

    export LIBTORCH=/opt/pytorch-v2.4.0/libtorch-2.4.0-arm/libtorch
    export LD_LIBRARY_PATH=$LIBTORCH/lib:$LD_LIBRARY_PATH
    RUST_LOG=info cargo run -p server --release

Compile a wasm code example

    cargo build-wasm -p llm

Create and run some runtimes

    python python_codes/monitor.py



## Launch multiple servers

Launch the executor on a terminal

    export LIBTORCH=/opt/pytorch-v2.4.0/libtorch-2.4.0-arm/libtorch
    export LD_LIBRARY_PATH=$LIBTORCH/lib:$LD_LIBRARY_PATH
    cargo run -p server --release -- -p 3030

    on other terminal:

    export LIBTORCH=/opt/pytorch-v2.4.0/libtorch-2.4.0-arm/libtorch
    export LD_LIBRARY_PATH=$LIBTORCH/lib:$LD_LIBRARY_PATH
    cargo run -p server --release -- -p 3031


Compile a wasm code example

    cargo build-wasm -p llm

Create and run some runtimes

    python python_codes/monitor_multy_server.py



## TODO:

- Capture the stdout and stderr and send it to the client