mod vm;
mod compiler;
use std::fs;
use std::path::Path;
use vm::{Op, VM};
use compiler::parse_dsl;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the program file
    let program_path = Path::new("program.dsl");
    let program_source = fs::read_to_string(program_path)?;

    // Parse the DSL into Vec<Op>
    let ops = parse_dsl(&program_source)?;

    // Create and execute the VM
    let mut vm = VM::new();
    vm.execute(&ops)?;

    // Print the final stack
    println!("Final stack:");
    for (i, &value) in vm.stack.iter().enumerate() {
        println!("  {}: {}", i, value);
    }

    Ok(())
}
