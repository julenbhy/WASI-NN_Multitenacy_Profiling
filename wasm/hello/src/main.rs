use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    // Print the arguments
    for arg in &args {
        println!("Arg: {}", arg);
    }

    // Get the first argument (after the program name)
    let id = if args.len() == 1 {
        args[0].parse::<usize>().unwrap_or(0)
    } else {
        0
    };

    println!("hello from runtime: {}", id);
}