// pub mod storage;

use icn_covm::bytecode::{BytecodeCompiler, BytecodeExecution};
use icn_covm::cli::federation::{federation_command, handle_federation_command};
use icn_covm::cli::proposal::{handle_proposal_command, proposal_command};
use icn_covm::cli::proposal_demo::run_proposal_demo;
use icn_covm::compiler::{parse_dsl, parse_dsl_with_stdlib, CompilerError};
use icn_covm::events::LogFormat;
use icn_covm::federation::messages::{ProposalScope, ProposalStatus, VotingModel};
use icn_covm::federation::{NetworkNode, NodeConfig};
use icn_covm::identity::Identity;
use icn_covm::storage::auth::AuthContext;
use icn_covm::storage::implementations::file_storage::FileStorage;
use icn_covm::storage::implementations::in_memory::InMemoryStorage;
use icn_covm::storage::traits::StorageBackend;
use icn_covm::storage::utils::now;
use icn_covm::vm::{VMError, VM};
use icn_covm::api;

use clap::{Arg, ArgAction, Command};
use log::{debug, error, info, warn};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::Path;
use std::process;
use std::time::Instant;
use thiserror::Error;
use std::sync::{Arc, Mutex};

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

    #[error("Federation error: {0}")]
    Federation(String),

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

impl From<Box<dyn Error>> for AppError {
    fn from(e: Box<dyn Error>) -> Self {
        AppError::Other(e.to_string())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::init();

    // Default storage settings
    let default_storage_backend = "memory";
    let default_storage_path = "./storage";

    // Parse command line arguments
    let api_cmd = Command::new("api")
        .about("Start the API server for web/mobile access")
        .arg(
            Arg::new("port")
                .long("port")
                .short('p')
                .value_name("PORT")
                .help("Port to listen on (default: 3030)")
                .value_parser(clap::value_parser!(u16))
                .default_value("3030"),
        );

    let matches = Command::new("icn-covm")
        .version("0.7.0")
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
                // Federation-related options
                .arg(
                    Arg::new("enable-federation")
                        .long("enable-federation")
                        .help("Enable federation support")
                        .action(ArgAction::SetTrue),
                )
                .arg(
                    Arg::new("federation-port")
                        .long("federation-port")
                        .value_name("PORT")
                        .help("Port number for federation listening")
                        .default_value("0"),
                )
                .arg(
                    Arg::new("bootstrap-nodes")
                        .long("bootstrap-nodes")
                        .value_name("MULTIADDR")
                        .help("Multiaddresses of bootstrap nodes (can be used multiple times)")
                        .action(ArgAction::Append),
                )
                .arg(
                    Arg::new("node-name")
                        .long("node-name")
                        .value_name("NAME")
                        .help("Human-readable name for this node")
                        .default_value("icn-covm-node"),
                )
                .arg(
                    Arg::new("capabilities")
                        .long("capabilities")
                        .value_name("CAPABILITY")
                        .help("Capabilities this node offers to the network (can be used multiple times)")
                        .action(ArgAction::Append),
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
                        ),
                )
        )
        .subcommand(proposal_command())
        .subcommand(federation_command())
        .subcommand(
            Command::new("proposal-demo")
                .about("Run a demo of the proposal lifecycle")
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
                                .help("Namespace to get value from")
                                .required(true)
                                .index(1),
                        )
                        .arg(
                            Arg::new("key")
                                .help("Key to get value for")
                                .required(true)
                                .index(2),
                        )
                )
        )
        .subcommand(api_cmd)
        .get_matches();

    // Handle subcommands
    let result: Result<(), AppError> = match matches.subcommand() {
        Some(("run", run_matches)) => {
            // Extract parameters
            let params = run_matches
                .get_many::<String>("param")
                .unwrap_or_default()
                .map(|s| {
                    let parts: Vec<&str> = s.split('=').collect();
                    if parts.len() != 2 {
                        eprintln!("Invalid parameter format: {}", s);
                        process::exit(1);
                    }
                    (parts[0].to_string(), parts[1].to_string())
                })
                .collect();

            // Get basic configuration
            let verbose = run_matches.get_flag("verbose");
            let program_path = run_matches.get_one::<String>("program").unwrap();
            let use_stdlib = run_matches.get_flag("stdlib");
            let use_bytecode = run_matches.get_flag("bytecode");

            // Use let bindings for default values to ensure they live long enough
            let default_storage_backend = "memory".to_string();
            let default_storage_path = "./storage".to_string();

            let storage_backend = run_matches
                .get_one::<String>("storage-backend")
                .unwrap_or(&default_storage_backend);
            let storage_path = run_matches
                .get_one::<String>("storage-path")
                .unwrap_or(&default_storage_path);

            // Get federation configuration
            let enable_federation = run_matches.get_flag("enable-federation");
            let federation_port = run_matches
                .get_one::<String>("federation-port")
                .unwrap()
                .parse::<u16>()
                .unwrap_or(0);
            let bootstrap_nodes = run_matches
                .get_many::<String>("bootstrap-nodes")
                .unwrap_or_default()
                .map(|s| s.parse().expect("Invalid multiaddress format"))
                .collect::<Vec<_>>();
            let node_name = run_matches
                .get_one::<String>("node-name")
                .unwrap()
                .to_string();
            let capabilities = run_matches
                .get_many::<String>("capabilities")
                .unwrap_or_default()
                .cloned()
                .collect::<Vec<String>>();

            if run_matches.get_flag("benchmark") {
                run_benchmark(
                    program_path,
                    verbose,
                    use_stdlib,
                    params,
                    storage_backend,
                    storage_path,
                )
            } else if run_matches.get_flag("interactive") {
                run_interactive(verbose, params, use_bytecode, storage_backend, storage_path)
            } else if enable_federation {
                // Run with federation enabled
                run_with_federation(
                    program_path,
                    verbose,
                    use_stdlib,
                    params,
                    use_bytecode,
                    storage_backend,
                    storage_path,
                    federation_port,
                    bootstrap_nodes,
                    node_name,
                    capabilities,
                )
                .await
            } else {
                // Standard run
                run_program(
                    program_path,
                    verbose,
                    use_stdlib,
                    params,
                    use_bytecode,
                    storage_backend,
                    storage_path,
                )
            }
        }
        Some(("identity", identity_matches)) => match identity_matches.subcommand() {
            Some(("register", register_matches)) => {
                let id_file = register_matches.get_one::<String>("file").unwrap();
                let id_type = register_matches.get_one::<String>("type").unwrap();
                let output_file = register_matches.get_one::<String>("output");
                register_identity(id_file, id_type, output_file)
            }
            _ => Err("Unknown identity subcommand".into()),
        },
        Some(("proposal", sub_matches)) => {
            let auth_context =
                get_or_create_auth_context(default_storage_backend, default_storage_path)?;
            let storage = setup_storage(default_storage_backend, default_storage_path)?;
            let mut vm = VM::with_storage_backend(storage);
            handle_proposal_command(&mut vm, sub_matches, &auth_context).map_err(|e| e.into())
        }
        Some(("proposal-demo", _)) => run_proposal_demo().map_err(|e| e.to_string().into()),
        Some(("storage", storage_matches)) => {
            let storage_backend = storage_matches
                .get_one::<String>("storage-backend")
                .unwrap();
            let storage_path = storage_matches.get_one::<String>("storage-path").unwrap();

            match storage_matches.subcommand() {
                Some(("list-keys", list_keys_matches)) => {
                    let namespace = list_keys_matches.get_one::<String>("namespace").unwrap();
                    let prefix = list_keys_matches.get_one::<String>("prefix");
                    list_keys_command(namespace, prefix, storage_backend, storage_path)
                }
                Some(("get-value", get_value_matches)) => {
                    let namespace = get_value_matches.get_one::<String>("namespace").unwrap();
                    let key = get_value_matches.get_one::<String>("key").unwrap();
                    get_value_command(namespace, key, storage_backend, storage_path)
                }
                _ => Err("Unknown storage subcommand".into()),
            }
        }
        Some(("federation", sub_matches)) => {
            let auth_context =
                get_or_create_auth_context(default_storage_backend, default_storage_path)?;
            let storage = setup_storage(default_storage_backend, default_storage_path)?;
            let mut vm = VM::with_storage_backend(storage);
            handle_federation_command(&mut vm, sub_matches, &auth_context)
                .await
                .map_err(|e| e.into())
        }
        Some(("api", api_matches)) => {
            let port = *api_matches.get_one::<u16>("port").unwrap_or(&3030);
            println!("Starting API server on port {}...", port);

            // Initialize VM with storage
            let storage = setup_storage(default_storage_backend, default_storage_path)?;
            let vm = VM::with_storage_backend(storage);

            // Start the API server
            api::start_api_server(vm, port)
                .await
                .map_err(|e| AppError::Other(format!("API server error: {}", e)))
        }
        _ => Err("Unknown command".into()),
    };

    // Handle errors
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    Ok(())
}

/// Run the virtual machine with federation enabled
async fn run_with_federation(
    program_path: &str,
    verbose: bool,
    use_stdlib: bool,
    parameters: HashMap<String, String>,
    use_bytecode: bool,
    storage_backend: &str,
    storage_path: &str,
    federation_port: u16,
    bootstrap_nodes: Vec<libp2p::Multiaddr>,
    node_name: String,
    capabilities: Vec<String>,
) -> Result<(), AppError> {
    info!("Starting ICN-COVM with federation enabled");
    debug!("Federation port: {}", federation_port);
    debug!("Bootstrap nodes: {:?}", bootstrap_nodes);
    debug!("Node name: {}", node_name);
    debug!("Capabilities: {:?}", capabilities);

    // Configure federation
    let node_config = NodeConfig {
        port: Some(federation_port),
        bootstrap_nodes,
        name: Some(node_name),
        capabilities,
        protocol_version: "1.0.0".to_string(),
    };

    // Create and start network node
    let mut network_node = match NetworkNode::new(node_config).await {
        Ok(node) => node,
        Err(e) => {
            return Err(AppError::Federation(format!(
                "Failed to create network node: {}",
                e
            )))
        }
    };

    info!("Local peer ID: {}", network_node.local_peer_id());

    // Start the network node
    if let Err(e) = network_node.start().await {
        return Err(AppError::Federation(format!(
            "Failed to start network node: {}",
            e
        )));
    }

    // Now run the program if specified
    if program_path != "program.dsl" || Path::new(program_path).exists() {
        run_program(
            program_path,
            verbose,
            use_stdlib,
            parameters,
            use_bytecode,
            storage_backend,
            storage_path,
        )?;
    } else {
        info!("No program specified, running in network-only mode");

        // Keep the node running until interrupted
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
    }

    Ok(())
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
    let storage = create_storage_backend(storage_backend, storage_path)?;

    if use_bytecode {
        // Bytecode execution with FileStorage
        let mut compiler = BytecodeCompiler::new();
        let program = compiler.compile(&ops);

        if verbose {
            println!("Compiled bytecode program:\n{}", program.dump());
        }

        // Create bytecode interpreter with proper auth context and storage
        let mut vm: VM<InMemoryStorage> = VM::new();
        vm.set_auth_context(auth_context);
        vm.set_namespace("demo");
        vm.set_storage_backend(storage);

        let mut interpreter =
            BytecodeExecution::new(VM::<InMemoryStorage>::new(), program.instructions);

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
        let mut vm: VM<InMemoryStorage> = VM::new();
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

    Ok(())
}

/// Helper to create the appropriate storage backend
fn create_storage_backend(backend_type: &str, path: &str) -> Result<InMemoryStorage, AppError> {
    match backend_type {
        "memory" | _ => {
            // For simplicity, we're only supporting InMemoryStorage for now
            // since there are type issues with FileStorage
            Ok(InMemoryStorage::new())
        }
    }
}

// Helper function to initialize any storage backend
fn initialize_storage<T: StorageBackend>(
    auth_context: &AuthContext,
    storage: &mut T,
    verbose: bool,
) -> Result<(), AppError> {
    // Create user account
    if let Err(e) = storage.create_account(Some(auth_context), &auth_context.user_id(), 1024 * 1024)
    {
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

    // Register the identity
    auth.register_identity(
        Identity::new(user_id.to_string(), None, "user".to_string(), None)
            .expect("Failed to create identity"),
    );

    // Add user roles directly to the auth context
    auth.add_role("global", "user");
    auth.add_role("demo", "reader");

    auth
}

// Helper function to create a demo auth context and initialize storage
fn setup_storage_for_demo() -> (AuthContext, InMemoryStorage) {
    let auth = create_demo_auth_context();

    // Create storage backend
    let mut storage = InMemoryStorage::new();

    // Create user account
    if let Err(e) = storage.create_account(Some(&auth), &auth.user_id(), 1024 * 1024) {
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

    let mut vm: VM<InMemoryStorage> = VM::new();

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

    let mut vm: VM<InMemoryStorage> = VM::new();
    vm.set_auth_context(auth_context);
    vm.set_namespace("demo");

    let mut interpreter =
        BytecodeExecution::new(VM::<InMemoryStorage>::new(), program.instructions);
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

    let mut vm: VM<InMemoryStorage> = VM::new();

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
                vm = VM::<InMemoryStorage>::new();
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

                            let mut interpreter = BytecodeExecution::new(
                                VM::<InMemoryStorage>::new(),
                                program.instructions,
                            );

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
    let id = identity_data
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'id' field")?;

    // Create the identity
    let identity = Identity::new(
        id.to_string(),
        None,
        id_type.to_string(),
        Some(
            identity_data
                .get("metadata")
                .and_then(|v| v.as_object())
                .map(|map| {
                    let mut hashmap = HashMap::new();
                    for (k, v) in map {
                        hashmap.insert(k.clone(), v.clone());
                    }
                    hashmap
                })
                .unwrap_or_default(),
        ),
    )
    .expect("Failed to create identity");

    // Create a basic auth context to simulate registration
    let mut auth = AuthContext::new("system");
    auth.add_role("global", "admin");

    // Register the identity
    auth.register_identity(identity.clone());

    // Output the identity
    println!(
        "Identity registered successfully: {} (type: {})",
        id, id_type
    );

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
            fs::create_dir_all(storage_dir).map_err(|e| {
                AppError::Other(format!("Failed to create storage directory: {}", e))
            })?;
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
                println!(
                    "No keys found in namespace '{}'{}",
                    namespace,
                    prefix.map_or(String::new(), |p| format!(" with prefix '{}'", p))
                );
            } else {
                println!(
                    "Keys in namespace '{}'{}",
                    namespace,
                    prefix.map_or(String::new(), |p| format!(" with prefix '{}'", p))
                );
                let keys_count = keys.len();
                for key in keys {
                    println!("  - {}", key);
                }
                println!("Total: {} keys", keys_count);
            }
            Ok(())
        }
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
            fs::create_dir_all(storage_dir).map_err(|e| {
                AppError::Other(format!("Failed to create storage directory: {}", e))
            })?;
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
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&json)
                                    .unwrap_or_else(|_| text.to_string())
                            );
                        }
                    }
                }
                Err(_) => {
                    println!(
                        "Value for {}:{} (binary data, {} bytes)",
                        namespace,
                        key,
                        data.len()
                    );
                    println!("{:?}", data);
                }
            }
            Ok(())
        }
        Err(e) => Err(AppError::Other(format!("Failed to get value: {}", e))),
    }
}

/// Creates an admin auth context for inspection purposes
fn create_admin_auth_context() -> AuthContext {
    let mut auth = AuthContext::new("admin");

    // Add admin roles for all operations
    auth.add_role("global", "admin");

    // Set up admin identity
    let mut identity = Identity::new("admin".to_string(), None, "admin".to_string(), None)
        .expect("Failed to create admin identity");
    identity.profile.other_fields.insert(
        "description".to_string(),
        serde_json::Value::String("Storage CLI Admin".to_string()),
    );

    // Register the identity
    auth.register_identity(identity);

    auth
}

/// Handle the broadcast-proposal federation command
async fn broadcast_proposal(
    proposal_file: &str,
    storage_backend: &str,
    storage_path: &str,
    federation_port: u16,
    bootstrap_nodes: Vec<libp2p::Multiaddr>,
    node_name: String,
    scope: &str,
    model: &str,
    coops: &str,
    expires_in: Option<u64>,
) -> Result<(), AppError> {
    info!("Broadcasting proposal from file: {}", proposal_file);

    // Read and parse the proposal file
    let proposal_content = fs::read_to_string(proposal_file).map_err(|e| AppError::IO(e))?;

    // Parse the proposal content (simple format for now)
    let lines: Vec<&str> = proposal_content.lines().collect();
    if lines.len() < 4 {
        return Err(AppError::Other(
            "Invalid proposal file format. Expected at least 4 lines: ID, namespace, creator, options".to_string(),
        ));
    }

    let proposal_id = lines[0].trim().to_string();
    let namespace = lines[1].trim().to_string();
    let creator = lines[2].trim().to_string();
    let options: Vec<String> = lines[3..]
        .iter()
        .map(|&s| s.trim().to_string())
        .collect::<Vec<String>>();

    // Parse the scope
    let scope = match scope {
        "single" => ProposalScope::SingleCoop(creator.clone()),
        "multi" => {
            let coop_list = coops
                .split(',')
                .map(|s| s.trim().to_string())
                .collect::<Vec<String>>();

            if coop_list.is_empty() {
                ProposalScope::GlobalFederation
            } else {
                ProposalScope::MultiCoop(coop_list)
            }
        }
        _ => ProposalScope::GlobalFederation,
    };

    // Parse the voting model
    let voting_model = match model {
        "coop" => VotingModel::OneCoopOneVote,
        _ => VotingModel::OneMemberOneVote,
    };

    // Create the proposal object
    let proposal = icn_covm::federation::FederatedProposal {
        proposal_id,
        namespace,
        options,
        creator,
        created_at: now() as i64,
        scope,
        voting_model,
        expires_at: expires_in.map(|seconds| (now() as i64) + (seconds as i64)),
        status: ProposalStatus::Open,
    };

    // Configure federation
    let node_config = NodeConfig {
        port: Some(federation_port),
        bootstrap_nodes,
        name: Some(node_name),
        capabilities: vec!["voting".to_string()],
        protocol_version: "1.0.0".to_string(),
    };

    // Create and start network node
    let mut network_node = match NetworkNode::new(node_config).await {
        Ok(node) => node,
        Err(e) => {
            return Err(AppError::Federation(format!(
                "Failed to create network node: {}",
                e
            )))
        }
    };

    info!("Local peer ID: {}", network_node.local_peer_id());

    // Start the network node
    if let Err(e) = network_node.start().await {
        return Err(AppError::Federation(format!(
            "Failed to start network node: {}",
            e
        )));
    }

    // Get a storage backend
    let mut storage = create_storage_backend(storage_backend, storage_path)?;

    // Store the proposal locally
    let federation_storage = network_node.federation_storage();
    if let Err(e) = federation_storage.save_proposal(&mut storage, proposal.clone()) {
        return Err(AppError::Federation(format!(
            "Failed to store proposal: {}",
            e
        )));
    }

    // Broadcast the proposal to the network
    if let Err(e) = network_node.broadcast_proposal(proposal).await {
        return Err(AppError::Federation(format!(
            "Failed to broadcast proposal: {}",
            e
        )));
    }

    info!("Proposal broadcasted successfully");

    // Keep the node running for a short time to ensure propagation
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    Ok(())
}

/// Handle the submit-vote federation command
async fn submit_vote(
    vote_file: &str,
    storage_backend: &str,
    storage_path: &str,
    federation_port: u16,
    bootstrap_nodes: Vec<libp2p::Multiaddr>,
    node_name: String,
) -> Result<(), AppError> {
    info!("Submitting vote from file: {}", vote_file);

    // Read and parse the vote file
    let vote_content = fs::read_to_string(vote_file).map_err(|e| AppError::IO(e))?;

    // Parse the vote content (simple format for now)
    let lines: Vec<&str> = vote_content.lines().collect();
    if lines.len() < 3 {
        return Err(AppError::Other(
            "Invalid vote file format. Expected at least 3 lines: proposal ID, voter ID, ranked choices".to_string(),
        ));
    }

    let proposal_id = lines[0].trim().to_string();
    let voter = lines[1].trim().to_string();

    // Parse the ranked choices
    let ranked_choices: Vec<f64> = lines[2]
        .split(',')
        .map(|s| {
            s.trim()
                .parse::<f64>()
                .map_err(|_| AppError::Other(format!("Invalid ranked choice: {}", s)))
        })
        .collect::<Result<Vec<f64>, AppError>>()?;

    // Get the message (optional but recommended for real systems)
    let message = if lines.len() > 3 {
        lines[3].trim().to_string()
    } else {
        // Generate a canonical message for signing if none was provided
        format!(
            "Vote from {} on proposal {} with choices {}",
            voter,
            proposal_id,
            lines[2].trim()
        )
    };

    // Get the signature (required for real systems, but we'll accept placeholder for testing)
    let signature = if lines.len() > 4 {
        lines[4].trim().to_string()
    } else {
        info!("No signature provided in vote file, using 'valid' placeholder for testing only");
        "valid".to_string() // For testing only
    };

    info!(
        "Parsed vote for proposal {} by {} with {} ranked choices",
        proposal_id,
        voter,
        ranked_choices.len()
    );

    // Create the vote object
    let vote = icn_covm::federation::FederatedVote {
        proposal_id,
        voter,
        ranked_choices,
        message,
        signature,
    };

    // Configure federation
    let node_config = NodeConfig {
        port: Some(federation_port),
        bootstrap_nodes,
        name: Some(node_name),
        capabilities: vec!["voting".to_string()],
        protocol_version: "1.0.0".to_string(),
    };

    // Create and start network node
    let mut network_node = match NetworkNode::new(node_config).await {
        Ok(node) => node,
        Err(e) => {
            return Err(AppError::Federation(format!(
                "Failed to create network node: {}",
                e
            )))
        }
    };

    info!("Local peer ID: {}", network_node.local_peer_id());

    // Start the network node
    if let Err(e) = network_node.start().await {
        return Err(AppError::Federation(format!(
            "Failed to start network node: {}",
            e
        )));
    }

    // Get a storage backend
    let mut storage = create_storage_backend(storage_backend, storage_path)?;

    // Store the vote locally
    let federation_storage = network_node.federation_storage();
    if let Err(e) = federation_storage.save_vote(&mut storage, vote.clone(), None) {
        return Err(AppError::Federation(format!("Failed to store vote: {}", e)));
    }

    // Submit the vote to the network
    if let Err(e) = network_node.submit_vote(vote).await {
        return Err(AppError::Federation(format!(
            "Failed to submit vote: {}",
            e
        )));
    }

    info!("Vote submitted successfully");

    // Keep the node running for a short time to ensure propagation
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    Ok(())
}

/// Handle the execute-proposal federation command
async fn execute_proposal(
    proposal_id: &str,
    storage_backend: &str,
    storage_path: &str,
    federation_port: u16,
    bootstrap_nodes: Vec<libp2p::Multiaddr>,
    node_name: String,
    force: bool,
) -> Result<(), AppError> {
    info!("Executing proposal: {}", proposal_id);

    // Configure federation
    let node_config = NodeConfig {
        port: Some(federation_port),
        bootstrap_nodes,
        name: Some(node_name),
        capabilities: vec!["voting".to_string()],
        protocol_version: "1.0.0".to_string(),
    };

    // Create and start network node
    let mut network_node = match NetworkNode::new(node_config).await {
        Ok(node) => node,
        Err(e) => {
            return Err(AppError::Federation(format!(
                "Failed to create network node: {}",
                e
            )))
        }
    };

    info!("Local peer ID: {}", network_node.local_peer_id());

    // Start the network node
    if let Err(e) = network_node.start().await {
        return Err(AppError::Federation(format!(
            "Failed to start network node: {}",
            e
        )));
    }

    // Get a storage backend
    let storage = create_storage_backend(storage_backend, storage_path)?;

    // Get the proposal
    let federation_storage = network_node.federation_storage();
    let proposal = match federation_storage.get_proposal(&storage, proposal_id) {
        Ok(proposal) => proposal,
        Err(e) => {
            error!("Failed to retrieve proposal for {}: {}", proposal_id, e);
            return Ok(());
        }
    };

    // Check if the proposal has an expiry time
    if let Some(expires_at) = proposal.expires_at {
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        if current_time < expires_at && !force {
            // If the proposal hasn't expired yet and we're not forcing execution
            let remaining_seconds = expires_at - current_time;
            let remaining_minutes = remaining_seconds / 60;
            let remaining_hours = remaining_minutes / 60;

            return Err(AppError::Federation(
                format!("Proposal has not expired yet. {} hours {} minutes remaining. Use --force to override.", 
                    remaining_hours, remaining_minutes % 60)
            ));
        } else if current_time < expires_at && force {
            info!("Forcing execution of proposal before expiry due to --force flag");
        } else {
            info!("Proposal has expired, proceeding with execution");
        }
    }

    // Get votes
    let votes = match federation_storage.get_votes(&storage, proposal_id) {
        Ok(v) => v,
        Err(e) => {
            error!(
                "Failed to retrieve votes for proposal {}: {}",
                proposal_id, e
            );
            return Ok(());
        }
    };

    if votes.is_empty() {
        println!("No votes found for proposal {}", proposal_id);
        return Ok(());
    }

    println!("Found {} votes for proposal {}", votes.len(), proposal_id);

    // Create mock identities for voters
    let mut voter_identities = HashMap::new();
    for vote in &votes {
        // Create a mock identity with coop information based on the voter name
        // In a real implementation, these would be retrieved from the identity system
        let identity = match icn_covm::identity::Identity::new(
            vote.voter.clone(),
            None,
            "member".to_string(),
            None,
        ) {
            Ok(mut id) => {
                // For our test, we'll use the first part of the voter name as the cooperative ID
                // In a real implementation, this would be properly associated with the voter's identity
                if let Some(idx) = vote.voter.find('_') {
                    let coop_id = vote.voter[0..idx].to_string();
                    // Add metadata to set coop_id
                    let coop_id_value = serde_json::Value::String(coop_id);
                    id.profile
                        .other_fields
                        .insert("coop_id".to_string(), coop_id_value);
                }
                id
            }
            Err(e) => {
                warn!("Error creating identity for {}: {}", vote.voter, e);
                continue;
            }
        };

        voter_identities.insert(vote.voter.clone(), identity);
    }

    // Convert votes to a ranked ballots format
    let ballots = federation_storage.prepare_ranked_ballots(&votes, &proposal, &voter_identities);

    // Print information about the voting model
    match proposal.voting_model {
        VotingModel::OneMemberOneVote => {
            println!(
                "Using 'One Member, One Vote' model with {} votes",
                ballots.len()
            );
        }
        VotingModel::OneCoopOneVote => {
            // Count unique cooperatives
            let unique_coops: HashSet<&str> = voter_identities
                .values()
                .filter_map(|identity| {
                    identity
                        .profile
                        .other_fields
                        .get("coop_id")
                        .and_then(|value| {
                            if let serde_json::Value::String(s) = value {
                                Some(s.as_str())
                            } else {
                                None
                            }
                        })
                })
                .collect();

            println!(
                "Using 'One Cooperative, One Vote' model with {} votes from {} cooperatives",
                ballots.len(),
                unique_coops.len()
            );
        }
    }

    // Create and configure a VM to execute the ranked vote
    let mut vm: VM<InMemoryStorage> = VM::new();

    // Prepare the stack with ballot data
    for ballot in &ballots {
        for preference in ballot {
            vm.stack.push(*preference);
        }
    }

    // Execute ranked vote operation
    let result = vm.execute(&[icn_covm::vm::Op::RankedVote {
        candidates: proposal.options.len(),
        ballots: ballots.len(),
    }]);

    match result {
        Ok(_) => {
            // Get the winning option index
            if let Some(winner_index) = vm.top() {
                let winner_index = winner_index as usize;
                let winner_option = proposal.options.get(winner_index).ok_or_else(|| {
                    AppError::Federation(format!("Invalid winner index: {}", winner_index))
                })?;

                info!("Proposal voting complete!");
                info!(
                    "Winning option ({}/{}): {}",
                    winner_index + 1,
                    proposal.options.len(),
                    winner_option
                );

                // Print out all options and votes for clarity
                for (i, option) in proposal.options.iter().enumerate() {
                    println!("Option {}: {}", i + 1, option);
                }

                println!("\nTotal votes: {}", votes.len());
                println!(
                    "Voting model: {}",
                    match proposal.voting_model {
                        VotingModel::OneMemberOneVote => "One Member, One Vote",
                        VotingModel::OneCoopOneVote => "One Cooperative, One Vote",
                    }
                );
                println!("Eligible votes counted: {}", ballots.len());
                println!("WINNER: Option {} - {}", winner_index + 1, winner_option);
            } else {
                return Err(AppError::Federation(
                    "No result from ranked vote".to_string(),
                ));
            }
        }
        Err(e) => {
            return Err(AppError::VM(e));
        }
    }

    Ok(())
}

fn get_or_create_auth_context(
    storage_backend: &str,
    storage_path: &str,
) -> Result<AuthContext, AppError> {
    // For now, just create a simple auth context for demo purposes
    Ok(AuthContext::new("demo_user"))
}

fn setup_storage(storage_backend: &str, storage_path: &str) -> Result<Arc<tokio::sync::Mutex<InMemoryStorage>>, AppError> {
    // Create an in-memory storage and wrap it in Mutex and Arc for thread safety
    let storage = InMemoryStorage::new();
    Ok(Arc::new(tokio::sync::Mutex::new(storage)))
}
