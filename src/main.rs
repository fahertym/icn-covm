mod compiler;
mod events;
mod vm;
use clap::{Arg, Command};
use compiler::{parse_dsl, parse_dsl_with_stdlib, CompilerError};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process;
use thiserror::Error;
use vm::{VMError, VM};

#[derive(Debug, Error)]
enum AppError {
    #[error("VM error: {0}")]
    VM(#[from] VMError),

    #[error("Compiler error: {0}")]
    Compiler(#[from] CompilerError),

    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("{0}")]
    Other(String),
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Other(s.to_string())
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Other(s)
    }
}

fn main() {
    // Parse command line arguments
    let matches = Command::new("nano-cvm")
        .version("0.2.0")
        .author("Intercooperative Network")
        .about("Secure stack-based virtual machine with governance-inspired opcodes")
        .arg(
            Arg::new("program")
                .short('p')
                .long("program")
                .value_name("FILE")
                .help("Program file to execute (.dsl or .json)")
                .default_value("program.dsl"),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Display detailed execution information")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("param")
                .short('P')
                .long("param")
                .value_name("KEY=VALUE")
                .help("Set a key-value parameter for the program (can be used multiple times)")
                .action(clap::ArgAction::Append),
        )
        .arg(
            Arg::new("interactive")
                .short('i')
                .long("interactive")
                .help("Start in interactive REPL mode")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .help("Output logs in JSON format")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("stdlib")
                .long("stdlib")
                .help("Include standard library functions")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Get program file and verbosity setting
    let program_path = matches.get_one::<String>("program").unwrap();
    let verbose = matches.get_flag("verbose");
    let interactive = matches.get_flag("interactive");

    // Collect parameters
    let mut parameters = HashMap::new();
    if let Some(params) = matches.get_many::<String>("param") {
        for param_str in params {
            if let Some(equals_pos) = param_str.find('=') {
                let key = param_str[0..equals_pos].to_string();
                let value = param_str[equals_pos + 1..].to_string();
                if verbose {
                    println!("Parameter: {} = {}", key, value);
                }
                parameters.insert(key, value);
            } else {
                eprintln!(
                    "Warning: Invalid parameter format '{}', expected KEY=VALUE",
                    param_str
                );
            }
        }
    }

    // Execute the program
    if interactive {
        if let Err(err) = run_interactive(verbose, parameters) {
            eprintln!("Error: {}", err);
            process::exit(1);
        }
    } else {
        let use_stdlib = matches.get_flag("stdlib");
        if let Err(err) = run_program(program_path, verbose, use_stdlib, parameters) {
            eprintln!("Error: {}", err);
            process::exit(1);
        }
    }
}

fn run_program(
    program_path: &str,
    verbose: bool,
    use_stdlib: bool,
    parameters: HashMap<String, String>,
) -> Result<(), AppError> {
    let path = Path::new(program_path);

    // Check if file exists
    if !path.exists() {
        return Err(format!("Program file not found: {}", program_path).into());
    }

    // Parse operations based on file extension
    let ops = if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
        match extension.to_lowercase().as_str() {
            "dsl" => {
                if verbose {
                    println!("Parsing DSL program from {}", program_path);
                }
                let program_source = fs::read_to_string(path)?;

                // Check if we should include the standard library
                if verbose && use_stdlib {
                    println!("Including standard library functions");
                }

                if use_stdlib {
                    parse_dsl_with_stdlib(&program_source)?
                } else {
                    parse_dsl(&program_source)?
                }
            }
            "json" => {
                if verbose {
                    println!("Parsing JSON program from {}", program_path);
                }
                let program_json = fs::read_to_string(path)?;
                serde_json::from_str(&program_json)?
            }
            _ => return Err(format!("Unsupported file extension: {}", extension).into()),
        }
    } else {
        return Err("File has no extension".into());
    };

    if verbose {
        println!("Program loaded with {} operations", ops.len());
    }

    // Create and execute the VM
    let mut vm = VM::new();

    // Set parameters
    vm.set_parameters(parameters)?;

    if verbose {
        println!("Executing program...");
        println!("-----------------------------------");
    }

    vm.execute(&ops)?;

    if verbose {
        println!("-----------------------------------");
        println!("Program execution completed successfully");
    }

    // Print the final stack
    println!("Final stack:");
    for (i, &value) in vm.get_stack().iter().enumerate() {
        println!("  {}: {}", i, value);
    }

    Ok(())
}

fn run_interactive(verbose: bool, parameters: HashMap<String, String>) -> Result<(), AppError> {
    use std::io::{self, BufRead, Write};

    println!("nano-cvm interactive mode");
    println!("Enter commands in DSL format, 'help' for available commands, 'exit' to quit");

    let mut vm = VM::new();
    vm.set_parameters(parameters)?;

    // Show initial parameters if any
    if !vm.get_memory_map().is_empty() {
        println!("Initial parameters:");
        for (key, &value) in vm.get_memory_map().iter() {
            println!("  {}: {}", key, value);
        }
    }

    let stdin = io::stdin();
    let mut handle = stdin.lock();
    let mut buffer = String::new();

    loop {
        print!("> ");
        io::stdout().flush()?;

        buffer.clear();
        handle.read_line(&mut buffer)?;

        let input = buffer.trim();

        match input {
            "exit" | "quit" => {
                println!("Exiting interactive mode");
                break;
            }
            "help" => {
                println!("Available commands:");
                println!("  push <number>     - Push a number onto the stack");
                println!("  add               - Add the top two values on the stack");
                println!("  sub               - Subtract the top value from the second value");
                println!("  mul               - Multiply the top two values on the stack");
                println!("  div               - Divide the second value by the top value");
                println!("  dup               - Duplicate the top value on the stack");
                println!("  swap              - Swap the top two values on the stack");
                println!("  emit \"message\"    - Output a message");
                println!(
                    "  store <key>       - Store the top stack value in memory with the given key"
                );
                println!("  load <key>        - Load a value from memory with the given key");
                println!("  dump_stack        - Display the current stack");
                println!("  dump_memory       - Display the current memory");
                println!("  exit, quit        - Exit the interactive mode");
            }
            "dump_stack" => {
                println!("Stack:");
                for (i, &value) in vm.get_stack().iter().enumerate() {
                    println!("  {}: {}", i, value);
                }
            }
            "dump_memory" => {
                println!("Memory:");
                for (key, &value) in vm.get_memory_map().iter() {
                    println!("  {}: {}", key, value);
                }
            }
            _ => {
                // Try to parse and execute the input as DSL
                if !input.is_empty() {
                    match parse_dsl(input) {
                        Ok(ops) => {
                            if verbose {
                                println!("Executing {} operation(s)", ops.len());
                            }

                            if let Err(err) = vm.execute(&ops) {
                                println!("Error executing operations: {}", err);
                            } else if verbose {
                                println!("Operation(s) executed successfully");

                                println!("Stack:");
                                for (i, &value) in vm.get_stack().iter().enumerate() {
                                    println!("  {}: {}", i, value);
                                }
                            }
                        }
                        Err(err) => {
                            println!("Error parsing input: {}", err);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
