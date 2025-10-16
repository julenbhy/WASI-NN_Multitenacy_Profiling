import requests
from resource_monitor import ResourceMonitor

BASE_URL = "http://127.0.0.1:3030"

def main():

    # Start the resource monitor
    monitor = ResourceMonitor(interval=0.05)
    monitor.start()

    create_resp = requests.post(BASE_URL + "/create", json={"count": 3, "wasm_path": "./target/wasm32-wasip1/release/llm.wasm"})
    print("Create response:", create_resp.json())

    run_resp = requests.post(BASE_URL + "/run", json={"runtime_ids": [0]})
    print("Run response:", run_resp.json())

    run_resp = requests.post(BASE_URL + "/delete_all")
    print("Run response:", run_resp.json())

    # Stop the resource monitor
    monitor.stop()

    # Retrieve and save the collected metrics
    df = monitor.get_data()
    print(df.head())
    df.to_csv("jetson_metrics.csv", index=False)
    print("Metrics saved to jetson_metrics.csv")

if __name__ == "__main__":
    main()
