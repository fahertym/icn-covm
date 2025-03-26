mod vm;
use vm::{VM, Op};
use std::fs;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the program file
    let program_path = Path::new("program.json");
    let program_json = fs::read_to_string(program_path)?;
    
    // Parse the JSON into Vec<Op>
    let ops: Vec<Op> = serde_json::from_str(&program_json)?;
    
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
