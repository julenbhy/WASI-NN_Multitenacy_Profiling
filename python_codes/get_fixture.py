import os
import torch
from transformers import AutoModelForCausalLM
import gc

import warnings
warnings.filterwarnings("ignore", category=torch.jit.TracerWarning)


# Models to test
MODEL_NAMES = ["14m", "70m"]  # ["14m", "70m", "160m", "410m", "1b", "1.4b", "2.8b", "6.9b", "12b"]  # puedes añadir "6.9b", "12b"
MODELS = [f"EleutherAI/pythia-{label}" for label in MODEL_NAMES]

# Tipos de precisión
DTYPES = {
    #"float32": torch.float32,
    "float16": torch.float16,
}

# Dispositivos a probar
DEVICES = ["cpu", "gpu"]  # añade "mps" si tienes Mac M1/M2

OUTPUT_DIR = "fixture"
os.makedirs(OUTPUT_DIR, exist_ok=True)


class ScriptModel(torch.nn.Module):
    def __init__(self, model):
        super().__init__()
        self.model = model

    def forward(self, input_ids):
        return self.model(input_ids).logits



def get_model(model_name: str, dtype_name: str, dtype, device: str):
    print(f"\n=== Processing: {model_name} | {dtype_name} | {device} ===")

    device_map = "cuda:0" if device == "gpu" else "cpu"

    model = AutoModelForCausalLM.from_pretrained(
        model_name,
        torch_dtype=dtype,
        device_map=device_map
    )

    model.eval()
    wrapped = ScriptModel(model)

    # Example input (batch_size=1, seq_len=10)
    example_input = torch.randint(0, model.config.vocab_size, (1, 10), device=device_map)
    traced_model = torch.jit.trace(wrapped, example_input)

    safe_name = model_name.replace("/", "_")
    model_dir = os.path.join(OUTPUT_DIR, "models", dtype_name, device.replace(":", "_"))
    os.makedirs(model_dir, exist_ok=True)

    model_path = os.path.join(model_dir, f"{safe_name}.pt")
    traced_model.save(model_path)

    print(f"Saved at: {model_path}")

    # Clean memory
    del model
    del wrapped
    del traced_model
    del example_input
    gc.collect()

    if device == "gpu" and torch.cuda.is_available():
        torch.cuda.empty_cache()


def main():
    # Download the tokenizer: https://huggingface.co/EleutherAI/pythia-14m/resolve/main/tokenizer.json

    tokenizer_url = "https://huggingface.co/EleutherAI/pythia-14m/resolve/main/tokenizer.json"
    tokenizer_path = os.path.join(OUTPUT_DIR, "tokenizer.json")
    if not os.path.exists(tokenizer_path):
        os.system(f"wget {tokenizer_url} -O {tokenizer_path}")
        print(f"Tokenizer saved at: {tokenizer_path}")
    else:
        print(f"Tokenizer already exists at: {tokenizer_path}")

    for model in MODELS:
        for dtype_name, dtype in DTYPES.items():
            for device in DEVICES:
                get_model(model, dtype_name, dtype, device)

if __name__ == "__main__":
    main()