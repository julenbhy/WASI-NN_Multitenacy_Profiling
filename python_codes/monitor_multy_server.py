import requests
from resource_monitor import ResourceMonitor
import pandas as pd
import matplotlib.pyplot as plt
import numpy as np
import time

BASE_URL = "http://127.0.0.1"
PORTS = ["3030", "3031"]
EXCLUDED_METRICS = [
    "Temp", "Power", "clocks", "model", "EMC", "APE", "NVDEC",
    "NVJPG", "NVJPG1", "OFA", "SE", "VIC", "Fan", "pwmfan0", "uptime"
]






# =====================================================
# ============== DATA CLEANING UTILS ==================
# =====================================================

def preprocess_monitor_data(df: pd.DataFrame) -> pd.DataFrame:
    """
    Cleans and normalizes the jtop monitoring DataFrame for visualization.

    - Removes unnecessary or irrelevant metrics.
    - Aggregates CPU cores into a single average 'CPU' column.
    - Converts RAM and SWAP from [0,1] fraction to percentage [0,100].
    - Ensures consistent datetime formatting.
    """

    # Drop metrics not relevant for visualization
    filtered = df.drop(
        labels=[col for col in df.columns if any(x in col for x in EXCLUDED_METRICS)],
        axis=1
    )

    # Aggregate all CPU cores into a single mean CPU value
    cpu_cols = [col for col in filtered.columns if col.startswith("CPU")]
    if cpu_cols:
        filtered["CPU"] = filtered[cpu_cols].mean(axis=1)
        filtered.drop(columns=cpu_cols, inplace=True)

    # Convert RAM and SWAP to percentages
    for col in ["RAM", "SWAP"]:
        if col in filtered.columns:
            filtered[col] *= 100.0

    # Ensure GPU is numeric
    if "GPU" in filtered.columns:
        filtered["GPU"] = pd.to_numeric(filtered["GPU"], errors="coerce")

    # Round numeric columns for readability
    return filtered.round(3)


# =====================================================
# ================ PLOTTING UTILS =====================
# =====================================================

def plot_resource_usage_with_ram_details(title: str, df: pd.DataFrame):
    """
    Plots resource usage in two subplots:
    1. CPU, GPU, RAM (%)
    2. RAM used and RAM shared (GB)
    Vertical lines indicate events.
    """

    plt.style.use("seaborn-v0_8-whitegrid")
    fig, (ax1, ax2) = plt.subplots(2, 1, figsize=(14, 10), sharex=True)

    # --- Top subplot: CPU, GPU, RAM (%) ---
    metrics_top = ["CPU", "GPU", "RAM"]
    colors_top = plt.cm.viridis_r(np.linspace(0.1, 0.9, len(metrics_top)))
    for metric, color in zip(metrics_top, colors_top):
        if metric in df.columns:
            ax1.plot(df["time"], df[metric], label=metric, linewidth=2, color=color, alpha=0.9)

    # --- Bottom subplot: RAM used and RAM shared (GB) ---
    metrics_bottom = ["ram_used(total)", "ram_shared(gpu)"]
    colors_bottom = plt.cm.plasma(np.linspace(0.1, 0.9, len(metrics_bottom)))
    for metric, color in zip(metrics_bottom, colors_bottom):
        if metric in df.columns:
            ax2.plot(df["time"], df[metric], label=metric, linewidth=2, color=color, alpha=0.9)

    # --- Vertical lines for events ---

    # --- Titles, labels, grids ---
    ax1.set_title(f"{title} - % Metrics", fontsize=14, fontweight="bold")
    ax1.set_ylabel("Percentage (%)", fontsize=12)
    ax1.grid(True, linestyle="--", alpha=0.5)
    ax1.legend(title="Metrics", fontsize=10, title_fontsize=11, loc="upper left", bbox_to_anchor=(1.02, 1))

    ax2.set_title(f"{title} - RAM Usage (GB)", fontsize=14, fontweight="bold")
    ax2.set_xlabel("Time (seconds since epoch)", fontsize=12)
    ax2.set_ylabel("GB", fontsize=12)
    ax2.grid(True, linestyle="--", alpha=0.5)
    ax2.legend(title="Metrics", fontsize=10, title_fontsize=11, loc="upper left", bbox_to_anchor=(1.02, 1))

    # --- Disable datetime formatting (critical!) ---
    from matplotlib.ticker import FuncFormatter
    ax2.xaxis.set_major_formatter(FuncFormatter(lambda x, _: f"{x:.1f}"))

    ax2.tick_params(axis="x", rotation=30)

    plt.subplots_adjust(bottom=0.15, right=0.8, hspace=0.3)
    plt.tight_layout()
    plt.show()




from requests_futures.sessions import FuturesSession

def main():

    session = FuturesSession()

    # -----------------------------
    # CREATE (async)
    # -----------------------------
    create_payload = {"count": 1, "wasm_path": "./target/wasm32-wasip1/release/llm.wasm"}
    futures = [
        session.post(f"{BASE_URL}:{port}/create", json=create_payload)
        for port in PORTS
    ]
    for fut, port in zip(futures, PORTS):
        resp = fut.result()  # bloquea solo aquí hasta que esté listo
        print(f"Create response from port {port}:", resp.json())

    # -----------------------------
    # RUN AND MONITOR
    # -----------------------------

    # Start the resource monitor
    monitor = ResourceMonitor(interval=0.05)
    monitor.start()

    run_start = time.time()

    run_payload = {"runtime_ids": [0]}

    run_futures = [
        session.post(f"{BASE_URL}:{port}/run", json=run_payload)
        for port in PORTS
    ]

    results = []
    for fut, port in zip(run_futures, PORTS):
        results.append(fut.result().json())

    run_time = time.time() - run_start

    # Stop the resource monitor
    monitor.stop()

    print(f"Run completed in {run_time:.2f} seconds.")
    print("Run responses:", results)


    # -----------------------------
    # DELETE
    # -----------------------------
    for port in PORTS:
        session.post(f"{BASE_URL}:{port}/delete_all")


    # -----------------------------
    # PLOT
    # -----------------------------
    df = monitor.get_data()
    df = preprocess_monitor_data(df)
    #df.to_csv("jetson_metrics.csv", index=False)
    #print("Metrics saved to jetson_metrics.csv")

    title = "Resource Usage"
    plot_resource_usage_with_ram_details(title, df)




if __name__ == "__main__":
    main()
