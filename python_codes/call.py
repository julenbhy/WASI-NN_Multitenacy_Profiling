import requests

BASE_URL = "http://127.0.0.1:3030"
TARGET = "llm" # llm or hello

def main():
    # Create 3 runtimes
    create_resp = requests.post(BASE_URL + "/create", json={"count": 1, "wasm_path": f"./target/wasm32-wasip1/release/{TARGET}.wasm"})
    print("Create response:", create_resp.json())
    # Example output: {"status": "created", "total_runtimes": 3}

    # Run the 3 runtimes simultaneously
    run_resp = requests.post(BASE_URL + "/run", json={"runtime_ids": [0]})
    print("Run response:", run_resp.json())
    # Example output: {"results": ["Runtime 0 executed!", "Runtime 1 executed!", "Runtime 2 executed!"]}

    #create_resp = requests.post(BASE_URL + "/create", json={"count": 2})
    #print("Create response:", create_resp.json())

    #run_resp = requests.post(BASE_URL + "/run", json={"runtime_ids": [2, 3, 4, 5]})
    #print("Run response:", run_resp.json())


    run_resp = requests.post(BASE_URL + "/list")
    print("Run response:", run_resp.json())

    run_resp = requests.post(BASE_URL + "/delete_all")
    print("Run response:", run_resp.json())




if __name__ == "__main__":
    main()