import time
import threading
import pandas as pd
from jtop import jtop

class ResourceMonitor:
    """
    Monitorea los recursos de Jetson en un hilo separado.
    Uso:
        monitor = ResourceMonitor()
        monitor.start()
        ... ejecutar tus cosas ...
        monitor.stop()
        df = monitor.get_data()
    """
    def __init__(self, interval=0.05):
        self.interval = interval
        self._stop_event = threading.Event()
        self._thread = None
        self._records = []
        self._start_time = None
        self._end_time = None

    def _collect(self):
        with jtop() as jetson:
            while not self._stop_event.is_set():
                jtop_stats = jetson.stats.copy()
                jtop_stats["ram_used(total)"] = jetson.memory["RAM"]["used"] / 1024 / 1024
                jtop_stats["ram_shared(gpu)"] = jetson.memory["RAM"]["shared"] / 1024 / 1024
                jtop_stats["time"] = time.time()
                self._records.append(jtop_stats)
                time.sleep(self.interval)

    def start(self):
        if self._thread and self._thread.is_alive():
            print("Monitor already running.")
            return
        self._records.clear()
        self._stop_event.clear()
        self._start_time = time.time()
        self._thread = threading.Thread(target=self._collect, daemon=True)
        self._thread.start()
        print("Resource monitor started.")

    def stop(self):
        if not self._thread:
            print("Monitor was never started.")
            return
        self._stop_event.set()
        self._thread.join()
        self._end_time = time.time()
        print("Resource monitor stopped. Duration:", round(self._end_time - self._start_time, 2), "s")

    def get_data(self):
        return pd.DataFrame(self._records)
