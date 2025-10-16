use std::collections::HashMap;
use std::sync::{Arc, Mutex as StdMutex};
use tokio::sync::Mutex as TokioMutex;
use warp::Filter;
use serde::Deserialize;
use tokio::task;
use serde_json::json;
mod runtime;
use runtime::WasmRuntime;



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


    // Map global: async mutex para acceder al HashMap
    let runtimes: Arc<TokioMutex<HashMap<usize, Arc<StdMutex<WasmRuntime>>>>> =
        Arc::new(TokioMutex::new(HashMap::new()));

    //
    // POST /create
    //
    let create_route = {
        let runtimes = runtimes.clone();
        warp::post()
            .and(warp::path("create"))
            .and(warp::body::json())
            .and_then(move |req: CreateRequest| {
                let runtimes = runtimes.clone();
                async move {
                    let mut map = runtimes.lock().await;
                    let start_id = map.len();
                    let wasm_file = req.wasm_path;
                    println!("Creating {} runtimes starting from ID {}", req.count, start_id);
                    for i in 0..req.count {
                        let id = start_id + i;
                        //map.insert(id, WasmRuntime::new(id));
                        match WasmRuntime::new(id, &wasm_file) {
                            Ok(runtime) => { map.insert(id, Arc::new(StdMutex::new(runtime))); },
                            Err(e) => { eprintln!("Error creating runtime {}: {}", id, e); }
                        }
                    }
                    println!("Total runtimes created: {}", map.len());

                    Ok::<_, warp::Rejection>(warp::reply::json(&serde_json::json!({
                        "status": "created",
                        "total_runtimes": map.len()
                    })))
                }
            })
    };

    //
    // POST /run
    //
    let run_route = {
        let runtimes = runtimes.clone();
        warp::post()
            .and(warp::path("run"))
            .and(warp::body::json())
            .and_then(move |req: RunRequest| {
                let runtimes = runtimes.clone();
                async move {
                    // 1) Get locks for all requested runtimes
                    let mut missing = Vec::new();
                    let mut to_run: Vec<Arc<StdMutex<WasmRuntime>>> = Vec::new();
                    {
                        let map = runtimes.lock().await;
                        for &id in &req.runtime_ids {
                            if let Some(rt_arc) = map.get(&id) {
                                to_run.push(rt_arc.clone());
                            } else {
                                missing.push(format!("Runtime {} not found", id));
                            }
                        }
                    } // free the async lock on the map

                    // 2) Launch blocking tasks to run each runtime
                    let mut handles = Vec::new();
                    for arc in to_run {
                        // spawn_blocking because run() is synchronous and potentially CPU-bound
                        handles.push(task::spawn_blocking(move || {
                            let mut guard = arc.lock().unwrap();
                            match guard.run() {
                                Ok(msg) => msg,
                                Err(e) => format!("Runtime {} error: {}", guard.id, e),
                            }
                        }));
                    }

                    // 3) Recollect results
                    let mut results = missing;
                    for h in handles {
                        match h.await {
                            Ok(res) => results.push(res),
                            Err(e) => results.push(format!("Task join error: {}", e)),
                        }
                    }

                    println!("Run results: {:?}", results);

                    Ok::<_, warp::Rejection>(warp::reply::json(&serde_json::json!({
                        "results": results
                    })))
                }
            })
    };

    //
    // POST /list  -> lista runtimes disponibles (ids y total)
    //
    let list_route = {
        let runtimes = runtimes.clone();
        warp::post()
            .and(warp::path("list"))
            .and_then(move || {
                let runtimes = runtimes.clone();
                async move {
                    let map = runtimes.lock().await;
                    let ids: Vec<usize> = map.keys().cloned().collect();
                    Ok::<_, warp::Rejection>(warp::reply::json(&json!({
                        "total": ids.len(),
                        "ids": ids
                    })))
                }
            })
    };


    //
    // POST /delete_all  -> elimina todos los runtimes
    //
    let delete_all_route = {
        let runtimes = runtimes.clone();
        warp::post()
            .and(warp::path("delete_all"))
            .and_then(move || {
                let runtimes = runtimes.clone();
                async move {
                    let mut map = runtimes.lock().await;
                    let removed_ids: Vec<usize> = map.keys().cloned().collect();
                    map.clear();
                    Ok::<_, warp::Rejection>(warp::reply::json(&json!({
                        "removed": removed_ids,
                        "total_remaining": 0
                    })))
                }
            })
    };

    //
    // POST /delete  -> elimina los runtimes listados en {"runtime_ids":[...]}
    //
    let delete_route = {
        let runtimes = runtimes.clone();
        warp::post()
            .and(warp::path("delete"))
            .and(warp::body::json())
            .and_then(move |req: DeleteRequest| {
                let runtimes = runtimes.clone();
                async move {
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
                    Ok::<_, warp::Rejection>(warp::reply::json(&json!({
                        "removed": removed,
                        "missing": missing,
                        "total_remaining": map.len()
                    })))
                }
            })
    };


    let routes = create_route
        .or(run_route)
        .or(list_route)
        .or(delete_all_route)
        .or(delete_route);

    println!("Server running at http://127.0.0.1:3030");
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
