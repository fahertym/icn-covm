// pub mod storage;

use icn_covm::bytecode::{BytecodeCompiler, BytecodeExecutor};
use icn_covm::compiler::{parse_dsl, parse_dsl_with_stdlib, CompilerError};
use icn_covm::storage::auth::AuthContext;
use icn_covm::vm::{VM, VMError};
use icn_covm::identity::{Identity, MemberProfile};
use icn_covm::storage::traits::StorageBackend;
use icn_covm::storage::implementations::in_memory::InMemoryStorage;
use icn_covm::storage::implementations::file_storage::FileStorage;
use icn_covm::storage::utils::now;

use clap::{Arg, Command, ArgAction};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process;
use std::time::Instant;
use thiserror::Error;
use std::io::{Write};

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
    let matches = Command::new("icn-covm")
        .version("0.5.0")
        .author("Intercooperative Network")
        .about("Secure stack-based virtual machine with governance-inspired opcodes")
        .subcommand(
            Command::new("run")
                .about("Run a program")
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
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("param")
                        .short('P')
                        .long("param")
                        .value_name("KEY=VALUE")
                        .help("Set a key-value parameter for the program (can be used multiple times)")
                        .action(ArgAction::Append),
                )
                .arg(
                    Arg::new("interactive")
                        .short('i')
                        .long("interactive")
                        .help("Start in interactive REPL mode")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("json")
                        .long("json")
                        .help("Output logs in JSON format")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("stdlib")
                        .long("stdlib")
                        .help("Include standard library functions")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("bytecode")
                        .short('b')
                        .long("bytecode")
                        .help("Run in bytecode mode (compile and execute bytecode)")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("benchmark")
                        .long("benchmark")
                        .help("Run both AST and bytecode execution and compare performance")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("storage-backend")
                        .long("storage-backend")
                        .value_name("TYPE")
                        .help("Storage backend type (memory or file)")
                        .default_value("memory"),
                )
                .arg(
                    Arg::new("storage-path")
                        .long("storage-path")
                        .value_name("PATH")
                        .help("Path for file storage backend")
                        .default_value("./storage"),
                )
        )
        .subcommand(
            Command::new("identity")
                .about("Identity management commands")
                .subcommand(
                    Command::new("register")
                        .about("Register a new identity")
                        .arg(
                            Arg::new("file")
                                .short('f')
                                .long("file")
                                .value_name("FILE")
                                .help("JSON file containing identity information")
                                .required(true),
                        )
                        .arg(
                            Arg::new("type")
                                .short('t')
                                .long("type")
                                .value_name("TYPE")
                                .help("Type of identity (member, cooperative, service)")
                                .default_value("member"),
                        )
                        .arg(
                            Arg::new("output")
                                .short('o')
                                .long("output")
                                .value_name("FILE")
                                .help("Output file to save the registered identity to"),
                        )
                )
        )
        .subcommand(
            Command::new("storage")
                .about("Storage inspection commands")
                .arg(
                    Arg::new("storage-backend")
                        .long("storage-backend")
                        .value_name("TYPE")
                        .help("Storage backend type (memory or file)")
                        .default_value("file"),
                )
                .arg(
                    Arg::new("storage-path")
                        .long("storage-path")
                        .value_name("PATH")
                        .help("Path for file storage backend")
                        .default_value("./storage"),
                )
                .subcommand(
                    Command::new("list-keys")
                        .about("List all keys in a namespace")
                        .arg(
                            Arg::new("namespace")
                                .help("Namespace to list keys from")
                                .required(true)
                                .index(1),
                        )
                        .arg(
                            Arg::new("prefix")
                                .short('p')
                                .long("prefix")
                                .help("Only list keys with this prefix")
                                .value_name("PREFIX"),
                        )
                )
                .subcommand(
                    Command::new("get-value")
                        .about("Get a value from storage")
                        .arg(
                            Arg::new("namespace")
                                .help("Namespace containing the key")
                                .required(true)
                                .index(1),
                        )
                        .arg(
                            Arg::new("key")
                                .help("Key to retrieve")
                                .required(true)
                                .index(2),
                        )
                )
        )
        .get_matches();

    match matches.subcommand() {
        Some(("run", run_matches)) => {
            // Get program file and verbosity setting
            let program_path = run_matches.get_one::<String>("program").unwrap();
            let verbose = run_matches.get_flag("verbose");
            let interactive = run_matches.get_flag("interactive");
            let use_bytecode = run_matches.get_flag("bytecode");
            let benchmark = run_matches.get_flag("benchmark");
            let use_stdlib = run_matches.get_flag("stdlib");
            let storage_backend = run_matches.get_one::<String>("storage-backend").unwrap();
            let storage_path = run_matches.get_one::<String>("storage-path").unwrap();

            // Collect parameters
            let mut parameters = HashMap::new();
            if let Some(params) = run_matches.get_many::<String>("param") {
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
                if let Err(err) = run_interactive(verbose, parameters, use_bytecode, storage_backend, storage_path) {
                    eprintln!("Error: {}", err);
                    process::exit(1);
                }
            } else {
                if benchmark {
                    if let Err(err) = run_benchmark(program_path, verbose, use_stdlib, parameters, storage_backend, storage_path) {
                        eprintln!("Error: {}", err);
                        process::exit(1);
                    }
                } else if let Err(err) =
                    run_program(program_path, verbose, use_stdlib, parameters, use_bytecode, storage_backend, storage_path)
                {
                    eprintln!("Error: {}", err);
                    process::exit(1);
                }
            }
        },
        Some(("identity", identity_matches)) => {
            match identity_matches.subcommand() {
                Some(("register", register_matches)) => {
                    let id_file = register_matches.get_one::<String>("file").unwrap();
                    let id_type = register_matches.get_one::<String>("type").unwrap();
                    let output_file = register_matches.get_one::<String>("output");
                    
                    if let Err(err) = register_identity(id_file, id_type, output_file) {
                        eprintln!("Error registering identity: {}", err);
                        process::exit(1);
                    }
                },
                _ => {
                    eprintln!("Unknown identity subcommand");
                    process::exit(1);
                }
            }
        },
        Some(("storage", storage_matches)) => {
            let storage_backend = storage_matches.get_one::<String>("storage-backend").unwrap();
            let storage_path = storage_matches.get_one::<String>("storage-path").unwrap();
            
            match storage_matches.subcommand() {
                Some(("list-keys", list_keys_matches)) => {
                    let namespace = list_keys_matches.get_one::<String>("namespace").unwrap();
                    let prefix = list_keys_matches.get_one::<String>("prefix");
                    
                    if let Err(err) = list_keys_command(namespace, prefix, storage_backend, storage_path) {
                        eprintln!("Error listing keys: {}", err);
                        process::exit(1);
                    }
                },
                Some(("get-value", get_value_matches)) => {
                    let namespace = get_value_matches.get_one::<String>("namespace").unwrap();
                    let key = get_value_matches.get_one::<String>("key").unwrap();
                    
                    if let Err(err) = get_value_command(namespace, key, storage_backend, storage_path) {
                        eprintln!("Error getting value: {}", err);
                        process::exit(1);
                    }
                },
                _ => {
                    eprintln!("Unknown storage subcommand. Use one of: list-keys, get-value");
                    process::exit(1);
                }
            }
        },
        _ => {
            // No subcommand or unknown subcommand
            // For backward compatibility, assume 'run' with default arguments
            let program_path = "program.dsl";
            let verbose = false;
            let use_stdlib = false;
            let parameters = HashMap::new();
            let use_bytecode = false;
            let storage_backend = "memory";
            let storage_path = "./storage";
            
            if let Err(err) = run_program(program_path, verbose, use_stdlib, parameters, use_bytecode, storage_backend, storage_path) {
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
    storage_backend: &str,
    storage_path: &str,
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

    // Setup auth context and storage based on selected backend
    let auth_context = create_demo_auth_context();
    
    // Select the appropriate storage backend
    if storage_backend == "file" {
        if verbose {
            println!("Using FileStorage backend at {}", storage_path);
        }
        
        // Create the storage directory if it doesn't exist
        let storage_dir = Path::new(storage_path);
        if !storage_dir.exists() {
            if verbose {
                println!("Creating storage directory: {}", storage_path);
            }
            fs::create_dir_all(storage_dir)
                .map_err(|e| AppError::Other(format!("Failed to create storage directory: {}", e)))?;
        }
        
        // Initialize the FileStorage backend
        match FileStorage::new(storage_path) {
            Ok(mut storage) => {
                initialize_storage(&auth_context, &mut storage, verbose)?;
                
                if use_bytecode {
                    // Bytecode execution with FileStorage
                    let mut compiler = BytecodeCompiler::new();
                    let program = compiler.compile(&ops);
                    
                    if verbose {
                        println!("Compiled bytecode program:\n{}", program.dump());
                    }
                    
                    // Create bytecode interpreter with proper auth context and storage
                    let mut vm = VM::new();
                    vm.set_auth_context(auth_context);
                    vm.set_namespace("demo");
                    vm.set_storage_backend(storage);
                    
                    let mut interpreter = BytecodeExecutor::new(vm, program.instructions);
                    
                    // Set parameters
                    interpreter.vm.set_parameters(parameters)?;
                    
                    // Execute
                    let start = Instant::now();
                    let result = interpreter.execute();
                    let duration = start.elapsed();
                    
                    if verbose {
                        println!("Execution completed in {:?}", duration);
                    }
                    
                    if let Err(err) = result {
                        return Err(err.into());
                    }
                    
                    if verbose {
                        println!("Final stack: {:?}", interpreter.vm.stack);
                        
                        if let Some(top) = interpreter.vm.top() {
                            println!("Top of stack: {}", top);
                        } else {
                            println!("Stack is empty");
                        }
                        
                        println!("Final memory: {:?}", interpreter.vm.memory);
                    }
                } else {
                    // AST execution with FileStorage
                    let mut vm = VM::new();
                    vm.set_auth_context(auth_context);
                    vm.set_namespace("demo");
                    vm.set_storage_backend(storage);

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
            },
            Err(e) => {
                return Err(AppError::Other(format!("Failed to initialize file storage: {}", e)));
            }
        }
    } else {
        // Use InMemoryStorage (default)
        if verbose {
            println!("Using InMemoryStorage backend");
        }
        
        // Initialize InMemoryStorage
        let mut storage = InMemoryStorage::new();
        initialize_storage(&auth_context, &mut storage, verbose)?;
        
        if use_bytecode {
            // Bytecode execution with InMemoryStorage
            let mut compiler = BytecodeCompiler::new();
            let program = compiler.compile(&ops);
            
            if verbose {
                println!("Compiled bytecode program:\n{}", program.dump());
            }
            
            // Create bytecode interpreter with proper auth context and storage
            let mut vm = VM::new();
            vm.set_auth_context(auth_context);
            vm.set_namespace("demo");
            vm.set_storage_backend(storage);
            
            let mut interpreter = BytecodeExecutor::new(vm, program.instructions);
            
            // Set parameters
            interpreter.vm.set_parameters(parameters)?;
            
            // Execute
            let start = Instant::now();
            let result = interpreter.execute();
            let duration = start.elapsed();
            
            if verbose {
                println!("Execution completed in {:?}", duration);
            }
            
            if let Err(err) = result {
                return Err(err.into());
            }
            
            if verbose {
                println!("Final stack: {:?}", interpreter.vm.stack);
                
                if let Some(top) = interpreter.vm.top() {
                    println!("Top of stack: {}", top);
                } else {
                    println!("Stack is empty");
                }
                
                println!("Final memory: {:?}", interpreter.vm.memory);
            }
        } else {
            // AST execution with InMemoryStorage
            let mut vm = VM::new();
            vm.set_auth_context(auth_context);
            vm.set_namespace("demo");
            vm.set_storage_backend(storage);

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
    }

    Ok(())
}

// Helper function to initialize any storage backend
fn initialize_storage<T: StorageBackend>(
    auth_context: &AuthContext,
    storage: &mut T,
    verbose: bool,
) -> Result<(), AppError> {
    // Create user account
    if let Err(e) = storage.create_account(Some(auth_context), &auth_context.user_id, 1024 * 1024) {
        if verbose {
            println!("Warning: Failed to create account: {:?}", e);
        }
    }
    
    // Create namespace
    if let Err(e) = storage.create_namespace(Some(auth_context), "demo", 1024 * 1024, None) {
        if verbose {
            println!("Warning: Failed to create namespace: {:?}", e);
        }
    }
    
    Ok(())
}

// Create a demo authentication context for storage operations
fn create_demo_auth_context() -> AuthContext {
    // Create a basic auth context for demo purposes
    let user_id = "demo_user";
    let mut auth = AuthContext::new(user_id);
    
    // Add roles with storage permissions - match the required roles in StorageBackend impl
    auth.add_role("global", "admin");   // Permission to create accounts and namespaces
    auth.add_role("demo", "reader");    // Permission to read from demo namespace
    auth.add_role("demo", "writer");    // Permission to write to demo namespace
    auth.add_role("demo", "admin");     // Permission to administrate demo namespace
    
    // Set up identity
    let mut identity = Identity::new(user_id, "user");
    identity.add_metadata("description", "Demo User");
    
    // Register the identity
    auth.register_identity(identity);
    
    // Set up member profile
    let mut profile = MemberProfile::new(Identity::new(user_id, "user"), now());
    profile.add_role("user");
    auth.register_member(profile);
    
    auth
}

// Helper function to create a demo auth context and initialize storage
fn setup_storage_for_demo() -> (AuthContext, InMemoryStorage) {
    let auth = create_demo_auth_context();
    
    // Create storage backend
    let mut storage = InMemoryStorage::new();
    
    // Create user account
    if let Err(e) = storage.create_account(Some(&auth), &auth.user_id, 1024 * 1024) {
        println!("Warning: Failed to create account: {:?}", e);
    }
    
    // Create namespace
    if let Err(e) = storage.create_namespace(Some(&auth), "demo", 1024 * 1024, None) {
        println!("Warning: Failed to create namespace: {:?}", e);
    }
    
    (auth, storage)
}

fn run_benchmark(
    program_path: &str,
    _verbose: bool,
    use_stdlib: bool,
    parameters: HashMap<String, String>,
    _storage_backend: &str,
    _storage_path: &str,
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
    
    // Set up auth context and namespace
    let auth_context = setup_storage_for_demo().0;
    vm.set_auth_context(auth_context.clone());
    vm.set_namespace("demo");
    
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

    let mut vm = VM::new();
    vm.set_auth_context(auth_context);
    vm.set_namespace("demo");
    
    let mut interpreter = BytecodeExecutor::new(vm, program.instructions);
    interpreter.vm.set_parameters(parameters)?;

    let bytecode_start = Instant::now();
    interpreter.execute()?;
    let bytecode_duration = bytecode_start.elapsed();

    println!("Bytecode execution time: {:?}", bytecode_duration);

    // Calculate speedup
    let total_bytecode_time = compiler_duration + bytecode_duration;
    println!(
        "\nTotal bytecode time (compilation + execution): {:?}",
        total_bytecode_time
    );

    if ast_duration > bytecode_duration {
        let speedup = ast_duration.as_secs_f64() / bytecode_duration.as_secs_f64();
        println!(
            "Bytecode execution is {:.2}x faster than AST interpretation",
            speedup
        );
    } else {
        let slowdown = bytecode_duration.as_secs_f64() / ast_duration.as_secs_f64();
        println!(
            "Bytecode execution is {:.2}x slower than AST interpretation",
            slowdown
        );
    }

    if ast_duration > total_bytecode_time {
        let speedup = ast_duration.as_secs_f64() / total_bytecode_time.as_secs_f64();
        println!(
            "Bytecode (including compilation) is {:.2}x faster than AST interpretation",
            speedup
        );
    } else {
        let slowdown = total_bytecode_time.as_secs_f64() / ast_duration.as_secs_f64();
        println!(
            "Bytecode (including compilation) is {:.2}x slower than AST interpretation",
            slowdown
        );
    }

    Ok(())
}

fn run_interactive(
    verbose: bool,
    parameters: HashMap<String, String>,
    use_bytecode: bool,
    storage_backend: &str,
    storage_path: &str,
) -> Result<(), AppError> {
    use std::io::{self, Write};

    println!("ICN Cooperative VM Interactive Shell (type 'exit' to quit, 'help' for commands)");
    
    let mut vm = VM::new();
    
    // Set up auth context and namespace
    let (auth_context, _storage) = setup_storage_for_demo();
    vm.set_auth_context(auth_context);
    vm.set_namespace("demo");
    
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
                let stack = vm.get_stack();
                for (i, &value) in stack.iter().enumerate() {
                    println!("  {}: {}", i, value);
                }
                if stack.is_empty() {
                    println!("  (empty)");
                }
            }
            "memory" => {
                println!("Memory:");
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
            "reset" => {
                vm = VM::new();
                println!("VM reset");
            }
            "mode ast" => {
                return run_interactive(
                    verbose,
                    vm.get_memory_map()
                        .iter()
                        .map(|(k, v)| (k.clone(), v.to_string()))
                        .collect(),
                    false,
                    storage_backend,
                    storage_path,
                );
            }
            "mode bytecode" => {
                return run_interactive(
                    verbose,
                    vm.get_memory_map()
                        .iter()
                        .map(|(k, v)| (k.clone(), v.to_string()))
                        .collect(),
                    true,
                    storage_backend,
                    storage_path,
                );
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
                match parse_dsl(trimmed) {
                    Ok(ops) => {
                        if use_bytecode {
                            // Compile to bytecode and execute
                            let mut compiler = BytecodeCompiler::new();
                            let program = compiler.compile(&ops);

                            if verbose {
                                println!("Compiled to bytecode:");
                                println!("{}", program.dump());
                            }

                            let mut interpreter = BytecodeExecutor::new(VM::new(), program.instructions);

                            // Copy VM state to interpreter
                            for (key, value) in vm.memory.iter() {
                                interpreter.vm.memory.insert(key.clone(), *value);
                            }

                            // Execute with bytecode
                            let bytecode_start = Instant::now();
                            interpreter.execute()?;
                            let bytecode_duration = bytecode_start.elapsed();

                            println!("Bytecode: {:?}", bytecode_duration);

                            // Copy results back to REPL VM
                            vm.stack = interpreter.vm.stack.clone();
                            vm.memory = interpreter.vm.memory.clone();

                            // Print result (if any)
                            if let Some(result) = interpreter.vm.top() {
                                println!("Result: {}", result);
                            }
                        } else {
                            // Execute directly with AST interpreter
                            match vm.execute(&ops) {
                                Ok(()) => {
                                    if let Some(result) = vm.top() {
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

/// Register a new identity using the information in the provided JSON file
fn register_identity(
    id_file: &str,
    id_type: &str,
    output_file: Option<&String>,
) -> Result<(), AppError> {
    // Load the identity data from file
    let id_data = fs::read_to_string(id_file)?;
    
    // Parse as JSON
    let identity_data: serde_json::Value = serde_json::from_str(&id_data)?;
    
    // Extract required fields
    let id = identity_data.get("id").and_then(|v| v.as_str()).ok_or("Missing 'id' field")?;
    
    // Create the identity
    let mut identity = Identity::new(id, id_type);
    
    // Add metadata
    if let Some(metadata) = identity_data.get("metadata").and_then(|v| v.as_object()) {
        for (key, value) in metadata {
            if let Some(val_str) = value.as_str() {
                identity.add_metadata(key, val_str);
            }
        }
    }
    
    // Create a basic auth context to simulate registration
    let mut auth = AuthContext::new("system");
    auth.add_role("global", "admin");
    
    // Register the identity
    auth.register_identity(identity.clone());
    
    // Output the identity
    println!("Identity registered successfully: {} (type: {})", id, id_type);
    
    // Save to output file if specified
    if let Some(out_file) = output_file {
        let json = serde_json::to_string_pretty(&identity)?;
        fs::write(out_file, json)?;
        println!("Identity saved to: {}", out_file);
    }
    
    Ok(())
}

/// Command to list keys in a namespace
fn list_keys_command(
    namespace: &str,
    prefix: Option<&String>,
    storage_backend: &str,
    storage_path: &str,
) -> Result<(), AppError> {
    // Create an admin auth context for inspection purposes
    let auth_context = create_admin_auth_context();

    // Initialize the appropriate storage backend
    let storage: Box<dyn StorageBackend> = if storage_backend == "file" {
        // Create the storage directory if it doesn't exist
        let storage_dir = Path::new(storage_path);
        if !storage_dir.exists() {
            println!("Creating storage directory: {}", storage_path);
            fs::create_dir_all(storage_dir)
                .map_err(|e| AppError::Other(format!("Failed to create storage directory: {}", e)))?;
        }
        
        // Initialize FileStorage backend
        let storage = FileStorage::new(storage_path)
            .map_err(|e| AppError::Other(format!("Failed to initialize file storage: {}", e)))?;
        Box::new(storage)
    } else {
        // Initialize InMemoryStorage backend
        Box::new(InMemoryStorage::new())
    };
    
    // Convert the optional prefix String to an optional &str
    let prefix_str = prefix.map(|s| s.as_str());
    
    // List keys from the storage backend
    match storage.list_keys(Some(&auth_context), namespace, prefix_str) {
        Ok(keys) => {
            if keys.is_empty() {
                println!("No keys found in namespace '{}'{}", 
                    namespace, 
                    prefix.map_or(String::new(), |p| format!(" with prefix '{}'", p))
                );
            } else {
                println!("Keys in namespace '{}'{}", 
                    namespace, 
                    prefix.map_or(String::new(), |p| format!(" with prefix '{}'", p))
                );
                for key in keys {
                    println!("  - {}", key);
                }
                println!("Total: {} keys", keys.len());
            }
            Ok(())
        },
        Err(e) => Err(AppError::Other(format!("Failed to list keys: {}", e))),
    }
}

/// Command to get a value from storage
fn get_value_command(
    namespace: &str,
    key: &str,
    storage_backend: &str,
    storage_path: &str,
) -> Result<(), AppError> {
    // Create an admin auth context for inspection purposes
    let auth_context = create_admin_auth_context();

    // Initialize the appropriate storage backend
    let storage: Box<dyn StorageBackend> = if storage_backend == "file" {
        // Create the storage directory if it doesn't exist
        let storage_dir = Path::new(storage_path);
        if !storage_dir.exists() {
            println!("Creating storage directory: {}", storage_path);
            fs::create_dir_all(storage_dir)
                .map_err(|e| AppError::Other(format!("Failed to create storage directory: {}", e)))?;
        }
        
        // Initialize FileStorage backend
        let storage = FileStorage::new(storage_path)
            .map_err(|e| AppError::Other(format!("Failed to initialize file storage: {}", e)))?;
        Box::new(storage)
    } else {
        // Initialize InMemoryStorage backend
        Box::new(InMemoryStorage::new())
    };
    
    // Get the value from storage
    match storage.get(Some(&auth_context), namespace, key) {
        Ok(data) => {
            // Try to decode as UTF-8 string
            match std::str::from_utf8(&data) {
                Ok(text) => {
                    println!("Value for {}:{}", namespace, key);
                    println!("{}", text);
                    
                    // If it looks like JSON, try to pretty-print it
                    if text.trim().starts_with('{') || text.trim().starts_with('[') {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
                            println!("\nFormatted JSON:");
                            println!("{}", serde_json::to_string_pretty(&json).unwrap_or_else(|_| text.to_string()));
                        }
                    }
                },
                Err(_) => {
                    println!("Value for {}:{} (binary data, {} bytes)", namespace, key, data.len());
                    println!("{:?}", data);
                }
            }
            Ok(())
        },
        Err(e) => Err(AppError::Other(format!("Failed to get value: {}", e))),
    }
}

/// Creates an admin auth context for inspection purposes
fn create_admin_auth_context() -> AuthContext {
    let mut auth = AuthContext::new("admin");
    
    // Add admin roles for all operations
    auth.add_role("global", "admin");
    
    // Set up identity
    let mut identity = Identity::new("admin", "admin");
    identity.add_metadata("description", "Storage CLI Admin");
    
    // Register the identity
    auth.register_identity(identity);
    
    auth
}
