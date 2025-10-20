use clap::Parser;
use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use std::time::Instant;
use tokio::sync::Mutex as TokioMutex;
use warp::Filter;
use serde::Deserialize;
use tokio::task;
use serde_json::json;
use env_logger;
use log;

mod runtime;
use runtime::WasmRuntime;

/// Command-line arguments
#[derive(Parser, Debug)]
#[command(name = "executor")]
#[command(about = "WASM Runtime Executor Server", long_about = None)]
struct Args {
    /// Port to expose the server (default: 3030)
    #[arg(short, long, default_value_t = 3030)]
    port: u16,
}

/// Shared state type alias
type RuntimeMap = Arc<TokioMutex<HashMap<usize, Arc<StdMutex<WasmRuntime>>>>>;

/// Request payloads
#[derive(Deserialize)]
struct CreateRequest {
    count: usize,
    wasm_path: String,
}

#[derive(Deserialize)]
struct RunRequest {
    runtime_ids: Vec<usize>,
}

#[derive(Deserialize)]
struct DeleteRequest {
    runtime_ids: Vec<usize>,
}

#[tokio::main]
async fn main() {
    // Parse command-line arguments
    let args = Args::parse();

    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("server=info")).init();
    log::info!("Executor starting on port {}", args.port);

    // Shared global state
    let runtimes: RuntimeMap = Arc::new(TokioMutex::new(HashMap::new()));

    // Build all routes
    let routes = warp::any().and(
        create_route(runtimes.clone())
            .or(run_route(runtimes.clone()))
            .or(list_route(runtimes.clone()))
            .or(delete_route(runtimes.clone()))
            .or(delete_all_route(runtimes.clone())),
    );

    println!("Server running at http://127.0.0.1:{}", args.port);
    warp::serve(routes).run(([127, 0, 0, 1], args.port)).await;
}

//
// -----------------------------
// ROUTE HANDLERS
// -----------------------------
//

/// POST /create
/// Creates N new WasmRuntime instances and measures total creation time
fn create_route(runtimes: RuntimeMap) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("create"))
        .and(warp::body::json())
        .and_then(move |req: CreateRequest| {
            let runtimes = runtimes.clone();
            async move {
                let start_time = Instant::now(); // start timer

                let mut map = runtimes.lock().await;
                let start_id = map.len();
                log::info!("Creating {} runtimes starting from ID {}", req.count, start_id);

                let mut created = 0usize;
                for i in 0..req.count {
                    let id = start_id + i;
                    match WasmRuntime::new(id, &req.wasm_path) {
                        Ok(runtime) => {
                            map.insert(id, Arc::new(StdMutex::new(runtime)));
                            created += 1;
                        }
                        Err(e) => {
                            log::error!("Error creating runtime {}: {}", id, e);
                        }
                    }
                }

                let elapsed = start_time.elapsed();
                log::info!(
                    "Created {} runtimes in {:.2?} (total: {})",
                    created,
                    elapsed,
                    map.len()
                );

                Ok::<_, warp::Rejection>(warp::reply::json(&json!({
                    "status": "created",
                    "created": created,
                    "total_runtimes": map.len(),
                    "elapsed_ms": elapsed.as_millis()
                })))
            }
        })
}

/// POST /run
/// Executes the specified runtimes and measures execution time per runtime
fn run_route(runtimes: RuntimeMap) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("run"))
        .and(warp::body::json())
        .and_then(move |req: RunRequest| {
            let runtimes = runtimes.clone();
            async move {
                let global_start = Instant::now();

                // Collect runtime references
                let mut missing = Vec::new();
                let mut to_run = Vec::new();

                {
                    let map = runtimes.lock().await;
                    for &id in &req.runtime_ids {
                        if let Some(rt_arc) = map.get(&id) {
                            to_run.push((id, rt_arc.clone()));
                        } else {
                            missing.push(format!("Runtime {} not found", id));
                        }
                    }
                }

                // Run all runtimes concurrently
                let mut handles = Vec::new();
                for (id, arc) in to_run {
                    handles.push(task::spawn_blocking(move || {
                        let start = Instant::now();
                        let mut guard = arc.lock().unwrap();
                        let result = match guard.run() {
                            Ok(msg) => msg,
                            Err(e) => format!("Runtime {} error: {}", guard.id, e),
                        };
                        let duration = start.elapsed();
                        log::info!("Runtime {} executed in {:.2?}", id, duration);
                        json!({
                            "id": id,
                            "result": result,
                            "elapsed_ms": duration.as_millis()
                        })
                    }));
                }

                // Collect results
                let mut results = Vec::new();
                for h in handles {
                    match h.await {
                        Ok(res) => results.push(res),
                        Err(e) => results.push(json!({"error": format!("Task join error: {}", e)})),
                    }
                }

                let total_time = global_start.elapsed();
                log::info!("All runtimes executed in {:.2?}", total_time);

                Ok::<_, warp::Rejection>(warp::reply::json(&json!({
                    "results": results,
                    "missing": missing,
                    "total_elapsed_ms": total_time.as_millis()
                })))
            }
        })
}

/// POST /list
/// Returns the list of available runtimes
fn list_route(runtimes: RuntimeMap) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("list"))
        .and_then(move || {
            let runtimes = runtimes.clone();
            async move {
                let start = Instant::now();
                let map = runtimes.lock().await;
                let ids: Vec<usize> = map.keys().cloned().collect();
                let elapsed = start.elapsed();

                log::info!("Listed {} runtimes in {:.2?}", ids.len(), elapsed);

                Ok::<_, warp::Rejection>(warp::reply::json(&json!({
                    "total": ids.len(),
                    "ids": ids,
                    "elapsed_ms": elapsed.as_millis()
                })))
            }
        })
}

/// POST /delete
/// Deletes the specified runtimes by ID
fn delete_route(runtimes: RuntimeMap) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("delete"))
        .and(warp::body::json())
        .and_then(move |req: DeleteRequest| {
            let runtimes = runtimes.clone();
            async move {
                let start = Instant::now();

                let mut removed = Vec::new();
                let mut missing = Vec::new();
                let mut map = runtimes.lock().await;

                for id in req.runtime_ids {
                    if map.remove(&id).is_some() {
                        removed.push(id);
                    } else {
                        missing.push(id);
                    }
                }

                let elapsed = start.elapsed();
                log::info!(
                    "Deleted {} runtimes in {:.2?} (missing: {})",
                    removed.len(),
                    elapsed,
                    missing.len()
                );

                Ok::<_, warp::Rejection>(warp::reply::json(&json!({
                    "removed": removed,
                    "missing": missing,
                    "total_remaining": map.len(),
                    "elapsed_ms": elapsed.as_millis()
                })))
            }
        })
}

/// POST /delete_all
/// Deletes all runtimes and clears the global state
fn delete_all_route(runtimes: RuntimeMap) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path("delete_all"))
        .and_then(move || {
            let runtimes = runtimes.clone();
            async move {
                let start = Instant::now();
                let mut map = runtimes.lock().await;
                let removed_ids: Vec<usize> = map.keys().cloned().collect();
                map.clear();
                let elapsed = start.elapsed();

                log::info!("Deleted all runtimes ({} total) in {:.2?}", removed_ids.len(), elapsed);

                Ok::<_, warp::Rejection>(warp::reply::json(&json!({
                    "removed": removed_ids,
                    "total_remaining": 0,
                    "elapsed_ms": elapsed.as_millis()
                })))
            }
        })
}
