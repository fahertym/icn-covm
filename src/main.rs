mod compiler;
mod events;
mod vm;
mod bytecode;

#[cfg(feature = "typed-values")]
mod typed;

use bytecode::{BytecodeCompiler, BytecodeInterpreter};
use clap::{Arg, Command};
use compiler::{parse_dsl, parse_dsl_with_stdlib, CompilerError};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process;
use std::time::Instant;
use thiserror::Error;
use vm::{VMError, VM};

#[cfg(feature = "typed-values")]
use typed::TypedValue;

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
        .arg(
            Arg::new("bytecode")
                .short('b')
                .long("bytecode")
                .help("Run in bytecode mode (compile and execute bytecode)")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("benchmark")
                .long("benchmark")
                .help("Run both AST and bytecode execution and compare performance")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // Get program file and verbosity setting
    let program_path = matches.get_one::<String>("program").unwrap();
    let verbose = matches.get_flag("verbose");
    let interactive = matches.get_flag("interactive");
    let use_bytecode = matches.get_flag("bytecode");
    let benchmark = matches.get_flag("benchmark");

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
        if let Err(err) = run_interactive(verbose, parameters, use_bytecode) {
            eprintln!("Error: {}", err);
            process::exit(1);
        }
    } else {
        let use_stdlib = matches.get_flag("stdlib");
        if benchmark {
            if let Err(err) = run_benchmark(program_path, verbose, use_stdlib, parameters) {
                eprintln!("Error: {}", err);
                process::exit(1);
            }
        } else {
            if let Err(err) = run_program(program_path, verbose, use_stdlib, parameters, use_bytecode) {
                eprintln!("Error: {}", err);
                process::exit(1);
            }
        }
    }
}

fn run_program(
    program_path: &str,
    verbose: bool,
    use_stdlib: bool,
    parameters: HashMap<String, String>,
    use_bytecode: bool,
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

    if use_bytecode {
        // Compile operations to bytecode
        let mut compiler = BytecodeCompiler::new();
        let program = compiler.compile(&ops);
        
        if verbose {
            println!("Program compiled to {} bytecode instructions", program.instructions.len());
            println!("{}", program.dump());
        }

        // Create bytecode interpreter
        let mut interpreter = BytecodeInterpreter::new(program);
        
        // Set parameters
        interpreter.set_parameters(parameters)?;

        if verbose {
            println!("Executing bytecode program...");
            println!("-----------------------------------");
        }

        // Execute the bytecode program
        interpreter.execute()?;

        if verbose {
            println!("-----------------------------------");
            println!("Bytecode program execution completed successfully");
            
            // Print final stack state
            if let Some(top) = interpreter.vm().top() {
                println!("Final top of stack: {}", top);
            } else {
                println!("Stack is empty");
            }
        }
    } else {
        // Create and execute the VM
        let mut vm = VM::new();

        // Set parameters
        vm.set_parameters(parameters)?;

        if verbose {
            println!("Executing program in AST interpreter mode...");
            println!("-----------------------------------");
        }

        vm.execute(&ops)?;

        if verbose {
            println!("-----------------------------------");
            println!("Program execution completed successfully");
            
            // Print final stack state
            if let Some(top) = vm.top() {
                println!("Final top of stack: {}", top);
            } else {
                println!("Stack is empty");
            }
        }
    }

    Ok(())
}

fn run_benchmark(
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
                println!("Parsing DSL program from {}", program_path);
                let program_source = fs::read_to_string(path)?;

                if use_stdlib {
                    parse_dsl_with_stdlib(&program_source)?
                } else {
                    parse_dsl(&program_source)?
                }
            }
            "json" => {
                println!("Parsing JSON program from {}", program_path);
                let program_json = fs::read_to_string(path)?;
                serde_json::from_str(&program_json)?
            }
            _ => return Err(format!("Unsupported file extension: {}", extension).into()),
        }
    } else {
        return Err("File has no extension".into());
    };

    println!("Program loaded with {} operations", ops.len());
    println!("\nBenchmarking execution modes...");

    // Run AST interpreter
    println!("\n1. Running AST interpreter...");
    
    let mut vm = VM::new();
    vm.set_parameters(parameters.clone())?;
    
    let ast_start = Instant::now();
    vm.execute(&ops)?;
    let ast_duration = ast_start.elapsed();
    
    println!("AST execution time: {:?}", ast_duration);
    
    // Run bytecode compilation and execution
    println!("\n2. Running bytecode compiler and interpreter...");
    
    let compiler_start = Instant::now();
    let mut compiler = BytecodeCompiler::new();
    let program = compiler.compile(&ops);
    let compiler_duration = compiler_start.elapsed();
    
    println!("Bytecode compilation time: {:?}", compiler_duration);
    println!("Bytecode size: {} instructions", program.instructions.len());
    
    let mut interpreter = BytecodeInterpreter::new(program);
    interpreter.set_parameters(parameters)?;
    
    let bytecode_start = Instant::now();
    interpreter.execute()?;
    let bytecode_duration = bytecode_start.elapsed();
    
    println!("Bytecode execution time: {:?}", bytecode_duration);
    
    // Calculate speedup
    let total_bytecode_time = compiler_duration + bytecode_duration;
    println!("\nTotal bytecode time (compilation + execution): {:?}", total_bytecode_time);
    
    if ast_duration > bytecode_duration {
        let speedup = ast_duration.as_secs_f64() / bytecode_duration.as_secs_f64();
        println!("Bytecode execution is {:.2}x faster than AST interpretation", speedup);
    } else {
        let slowdown = bytecode_duration.as_secs_f64() / ast_duration.as_secs_f64();
        println!("Bytecode execution is {:.2}x slower than AST interpretation", slowdown);
    }
    
    if ast_duration > total_bytecode_time {
        let speedup = ast_duration.as_secs_f64() / total_bytecode_time.as_secs_f64();
        println!("Bytecode (including compilation) is {:.2}x faster than AST interpretation", speedup);
    } else {
        let slowdown = total_bytecode_time.as_secs_f64() / ast_duration.as_secs_f64();
        println!("Bytecode (including compilation) is {:.2}x slower than AST interpretation", slowdown);
    }
    
    Ok(())
}

fn run_interactive(verbose: bool, parameters: HashMap<String, String>, use_bytecode: bool) -> Result<(), AppError> {
    use std::io::{self, Write};

    println!("nano-cvm interactive REPL");
    println!("Type DSL code to execute, 'help' for commands, or 'exit' to quit");
    if use_bytecode {
        println!("Running in bytecode mode");
    } else {
        println!("Running in AST interpreter mode");
    }
    
    // Show verbose setting if enabled
    if verbose {
        println!("Verbose mode: enabled");
    }
    
    let mut vm = VM::new();
    vm.set_parameters(parameters)?;

    // Create an editor for interactive input
    let mut rl = rustyline::DefaultEditor::new().map_err(|e| AppError::Other(e.to_string()))?;

    loop {
        print!("> ");
        io::stdout().flush()?;

        // Read a line of input
        let line = match rl.readline("> ") {
            Ok(line) => line,
            Err(rustyline::error::ReadlineError::Interrupted) => {
                println!("Interrupted (Ctrl+C)");
                break;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("EOF (Ctrl+D)");
                break;
            }
            Err(e) => {
                return Err(AppError::Other(format!("Error reading input: {}", e)));
            }
        };

        // Add the line to the editor history
        if let Err(e) = rl.add_history_entry(&line) {
            return Err(AppError::Other(format!("Error adding to history: {}", e)));
        }

        // Process the line
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        match trimmed {
            "exit" | "quit" => {
                println!("Exiting REPL");
                break;
            }
            "help" => {
                println!("Available commands:");
                println!("  help         - Show this help message");
                println!("  exit, quit   - Exit the REPL");
                println!("  stack        - Display the current stack");
                println!("  memory       - Display memory contents");
                println!("  reset        - Reset the VM");
                println!("  mode ast     - Switch to AST interpreter mode");
                println!("  mode bytecode - Switch to bytecode execution mode");
                println!("  save <file>  - Save current program to a file");
                println!("  load <file>  - Load program from a file");
                println!();
                println!("Any other input will be interpreted as DSL code and executed.");
            }
            "stack" => {
                println!("Stack:");
                if use_bytecode {
                    let stack = vm.get_stack();
                    for (i, value) in stack.iter().enumerate() {
                        println!("  {}: {}", i, value);
                    }
                    if stack.is_empty() {
                        println!("  (empty)");
                    }
                } else {
                    let stack = vm.get_stack();
                    for (i, value) in stack.iter().enumerate() {
                        println!("  {}: {}", i, value);
                    }
                    if stack.is_empty() {
                        println!("  (empty)");
                    }
                }
            }
            "memory" => {
                println!("Memory:");
                if use_bytecode {
                    let memory = vm.get_memory_map();
                    let mut keys: Vec<_> = memory.keys().collect();
                    keys.sort();
                    for key in keys {
                        println!("  {}: {}", key, memory.get(key).unwrap());
                    }
                    if memory.is_empty() {
                        println!("  (empty)");
                    }
                } else {
                    let memory = vm.get_memory_map();
                    let mut keys: Vec<_> = memory.keys().collect();
                    keys.sort();
                    for key in keys {
                        println!("  {}: {}", key, memory.get(key).unwrap());
                    }
                    if memory.is_empty() {
                        println!("  (empty)");
                    }
                }
            }
            "reset" => {
                vm = VM::new();
                println!("VM reset");
            }
            "mode ast" => {
                return run_interactive(verbose, vm.get_memory_map().iter().map(|(k, v)| (k.clone(), v.to_string())).collect(), false);
            }
            "mode bytecode" => {
                return run_interactive(verbose, vm.get_memory_map().iter().map(|(k, v)| (k.clone(), v.to_string())).collect(), true);
            }
            _ if trimmed.starts_with("save ") => {
                let file_name = trimmed[5..].trim();
                if file_name.is_empty() {
                    println!("Usage: save <file>");
                    continue;
                }
                // Not implemented yet
                println!("Save functionality not yet implemented");
            }
            _ if trimmed.starts_with("load ") => {
                let file_name = trimmed[5..].trim();
                if file_name.is_empty() {
                    println!("Usage: load <file>");
                    continue;
                }
                // Not implemented yet
                println!("Load functionality not yet implemented");
            }
            _ => {
                // Parse and execute the input as DSL code
                match parse_dsl_with_stdlib(trimmed) {
                    Ok(ops) => {
                        if use_bytecode {
                            // Compile to bytecode and execute
                            let mut compiler = BytecodeCompiler::new();
                            let program = compiler.compile(&ops);
                            
                            if verbose {
                                println!("Compiled to bytecode:");
                                println!("{}", program.dump());
                            }
                            
                            let mut interpreter = BytecodeInterpreter::new(program);
                            
                            // Copy VM state to interpreter
                            for (key, value) in vm.get_memory_map() {
                                #[cfg(not(feature = "typed-values"))]
                                interpreter.vm_mut().memory.insert(key.clone(), *value);
                                
                                #[cfg(feature = "typed-values")]
                                interpreter.vm_mut().memory.insert(key.clone(), value.clone());
                            }
                            
                            // Execute
                            match interpreter.execute() {
                                Ok(()) => {
                                    // Copy the stack state back to the main VM for the next loop
                                    vm.stack = interpreter.vm().stack.clone();
                                    vm.memory = interpreter.vm().memory.clone();
                                    
                                    // Show result
                                    if let Some(result) = interpreter.vm().top() {
                                        #[cfg(not(feature = "typed-values"))]
                                        println!("Result: {}", result);
                                        
                                        #[cfg(feature = "typed-values")]
                                        println!("Result: {}", result);
                                    }
                                }
                                Err(e) => println!("Error: {}", e),
                            }
                        } else {
                            // Execute directly with AST interpreter
                            match vm.execute(&ops) {
                                Ok(()) => {
                                    if let Some(result) = vm.top() {
                                        #[cfg(not(feature = "typed-values"))]
                                        println!("Result: {}", result);
                                        
                                        #[cfg(feature = "typed-values")]
                                        println!("Result: {}", result);
                                    }
                                }
                                Err(e) => println!("Error: {}", e),
                            }
                        }
                    }
                    Err(e) => println!("Parse error: {}", e),
                }
            }
        }
    }

    Ok(())
}
