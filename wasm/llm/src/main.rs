#![allow(warnings)]

use anyhow::{Error, Result};
use std::{fs, env};
use std::time::Instant;
use wasi_nn::{self, ExecutionTarget, GraphBuilder, GraphEncoding};
use tokenizers::Tokenizer;

/// Run one inference step
fn run_inference(
    context: &mut wasi_nn::GraphExecutionContext,
    input_ids: &[i64],
    vocab_size: usize,
) -> Result<u32, Error> {
    let shape: Vec<usize> = vec![1, input_ids.len()];
    println!("→ Setting input tensor with shape {:?}", shape);
    context.set_input(0, wasi_nn::TensorType::I64, &shape, input_ids)?;

    println!("→ Running inference... ");
    context.compute()?;

    println!("→ Getting output tensor... ");
    let mut output_buffer = vec![0f32; input_ids.len() * vocab_size];
    context.get_output(0, &mut output_buffer)?;
    println!("→ Got output buffer, len = {}", output_buffer.len());

    // Take last timestep logits
    let seq_len = input_ids.len();
    let start = (seq_len - 1) * vocab_size;
    let end = seq_len * vocab_size;
    let last_logits = &output_buffer[start..end];

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
    mut context: wasi_nn::GraphExecutionContext,
    mut input_ids: Vec<i64>,
    tokenizer: &Tokenizer,
    vocab_size: usize,
    max_new_tokens: usize,
) -> Result<String, Error> {
    let mut output_text = tokenizer.decode(&input_ids.iter().map(|&x| x as u32).collect::<Vec<_>>(), true).unwrap();

    for _ in 0..max_new_tokens {
        let next_token_id = run_inference(&mut context, &input_ids, vocab_size)?;
        input_ids.push(next_token_id as i64);

        let decoded = tokenizer.decode(&[next_token_id], true).unwrap();
        print!("{}", decoded); // streaming output
        output_text.push_str(&decoded);
    }

    Ok(output_text)
}

pub fn main() -> Result<(), Error> {

    // Configuration
    let prompt = "A photo of a cat sitting on a mat with";
    let target = "cpu";
    let model_path = "./fixture/models/float16/cpu/EleutherAI_pythia-14m.pt";
    let vocab_size = 50304;
    let max_tokens = 1;

    // Select CPU or GPU
    let args: Vec<String> = env::args().collect();
    let target = if args.len() > 1 && args[1].to_lowercase() == "gpu" {
        println!("→ Using GPU for inference");
        ExecutionTarget::GPU
    } else {
        println!("→ Using CPU for inference");
        ExecutionTarget::CPU
    };

    let start_total = Instant::now();

    // Load model
    println!("→ Loading model from {}", model_path);
    let model = fs::read(model_path)?;
    println!("→ Initializing graph... ");
    let graph = GraphBuilder::new(GraphEncoding::Pytorch, target).build_from_bytes(&[model])?;
    let context = graph.init_execution_context()?;

    // Load tokenizer
    println!("→ Loading tokenizer... ");
    let tokenizer = Tokenizer::from_file("./fixture/tokenizer.json").unwrap();

    // Tokenize prompt
    println!("→ Tokenizing prompt... ");
    let input_ids: Vec<_> = tokenizer.encode(prompt, false).unwrap()
        .get_ids()
        .iter()
        .map(|&x| x as i64)
        .collect();

    println!("→ Input prompt: {}", prompt);
    println!("→ Tokenized input_ids: {:?}", input_ids);

    // Generate text
    let generated_text = generate_text(context, input_ids, &tokenizer, vocab_size, max_tokens)?;
    
    println!("\n\n→ Final generated text:\n{}", generated_text);

    let duration_total = start_total.elapsed();
    println!("→ Total time: {:?}", duration_total);

    Ok(())
}