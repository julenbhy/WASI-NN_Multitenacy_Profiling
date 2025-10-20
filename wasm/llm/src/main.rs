#![allow(warnings)]

use anyhow::{Error, Result, bail, anyhow};
use std::{fs, env};
use std::time::Instant;
use wasi_nn::{self, ExecutionTarget, GraphBuilder, GraphEncoding};
use tokenizers::Tokenizer;

/// Run one inference step
fn run_inference(
    context: &mut wasi_nn::GraphExecutionContext,
    input_ids: &[i32],
    vocab_size: usize,
) -> Result<u32, Error> {
    let seq_len = input_ids.len();
    if seq_len == 0 {
        bail!("input_ids is empty");
    }

    // Set input tensor
    let shape: Vec<usize> = vec![1, seq_len];
    //println!("→ Setting input tensor with shape {:?}", shape);
    context.set_input(0, wasi_nn::TensorType::I32, &shape, input_ids)?;

    //println!("→ Running inference... ");
    context.compute()?;

    // Prepare output buffer and check size
    //println!("→ Getting output tensor... ");
    let mut output_buffer = vec![0f32; input_ids.len() * vocab_size];
    context.get_output(0, &mut output_buffer)?;
    //println!("→ Got output buffer, len = {}", output_buffer.len());

    // Take last timestep logits
    let start = (seq_len - 1) * vocab_size;
    let last_logits = &output_buffer[start..start + vocab_size];


    // Greedy argmax
    let (best_id, _) = last_logits
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
        .unwrap();

    Ok(best_id as u32)
}

/// Generate autoregressively N tokens
fn generate_text(
    context: &mut wasi_nn::GraphExecutionContext,
    mut input_ids: Vec<i32>,
    tokenizer: &Tokenizer,
    vocab_size: usize,
    max_new_tokens: usize,
) -> Result<String, Error> {
    for _ in 0..max_new_tokens {
        let next_token_id = run_inference(context, &input_ids, vocab_size)?;
        input_ids.push(next_token_id as i32);
    }

    // Decodificar la secuencia completa (mejor que decodificar token a token y concatenar)
    let ids_u32: Vec<u32> = input_ids.iter().map(|&x| x as u32).collect();
    let output_text = tokenizer
        .decode(&ids_u32, true)
        .map_err(|e| anyhow!("error al decodificar: {}", e))?;
    Ok(output_text)
}


pub fn main() -> Result<(), Error> {

    let target_str = "gpu"; // hardcoded argument
    let model_size = "14m";

    // Configuration
    let prompt = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Phasellus ac ipsum vel erat ornare blandit eu at nunc. Vivamus fermentum efficitur nisi. Nulla placerat lorem sit amet aliquam interdum. Mauris mi lectus, ullamcorper sit amet malesuada eget, lacinia eu nunc. Vivamus lacinia eget massa nec sagittis. Nam iaculis, eros eu tempor tempor, odio nulla auctor libero, eu ornare erat est ut lorem. Donec malesuada porta porttitor. Aenean vehicula purus non justo egestas tristique. Donec sit amet leo id ante sagittis ultrices. Proin a lacus at velit elementum interdum. Donec vehicula felis purus, ac tempor augue mollis eu. Maecenas accumsan ullamcorper leo nec suscipit.";
    let model_path = format!("./fixture/models/float32/{}/EleutherAI_pythia-{}.pt", target_str, model_size);
    let vocab_size = 50304;
    let max_tokens = 20;

    // Select CPU or GPU
    let args: Vec<String> = env::args().collect();
    let target = if target_str.to_lowercase() == "gpu" {
        println!("→ Using GPU for inference");
        ExecutionTarget::GPU
    } else {
        println!("→ Using CPU for inference");
        ExecutionTarget::CPU
    };

    let start_total = Instant::now();

    // Load model
    //println!("→ Loading model from {}", model_path);
    let model = fs::read(model_path)?;
    //println!("→ Initializing graph... ");
    let graph = GraphBuilder::new(GraphEncoding::Pytorch, target).build_from_bytes(&[model])?;
    let mut context = graph.init_execution_context()?;

    // Load tokenizer
    //println!("→ Loading tokenizer... ");
    let tokenizer = Tokenizer::from_file("./fixture/tokenizer.json").unwrap();

    // Tokenize prompt
    //println!("→ Tokenizing prompt... ");
    let encoding = tokenizer
        .encode(prompt, false)
        .map_err(|e| anyhow!("error tokenizando prompt: {}", e))?;
    let input_ids: Vec<i32> = encoding.get_ids().iter().map(|&x| x as i32).collect();



    // Generate text
    let generated_text = generate_text(&mut context, input_ids, &tokenizer, vocab_size, max_tokens)?;
    
    println!("→ Input prompt: {}", prompt);
    println!("\n→ Generated text:\n{}", generated_text);

    let duration_total = start_total.elapsed();
    //println!("→ Total time: {:?}", duration_total);

    Ok(())
}
