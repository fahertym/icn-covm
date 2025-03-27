use std::fs;
use std::path::Path;
use serde_json;
use nano_cvm::{Op, VM};

#[test]
fn test_program_json_runs_correctly() -> Result<(), Box<dyn std::error::Error>> {
    // Read and parse program.json
    let program_path = Path::new("program.json");
    let program_json = fs::read_to_string(program_path)?;
    let ops: Vec<Op> = serde_json::from_str(&program_json)?;

    // Create and run VM
    let mut vm = VM::new();
    vm.execute(&ops)?;

    // Verify final stack state - check the tail including countdown's 0.0
    let expected_tail = &[0.0, 1.0, 2.0, 3.0, 3.0, 3.0];
    let actual_tail = &vm.get_stack()[vm.get_stack().len() - expected_tail.len()..];
    assert_eq!(actual_tail, expected_tail);

    // Memory: 'n' should not persist after countdown due to function scope
    assert_eq!(vm.get_memory("n"), None);

    // Optional: Print debug info
    println!("\nFinal stack:");
    for (i, &value) in vm.get_stack().iter().enumerate() {
        println!("  {}: {}", i, value);
    }

    println!("\nFinal memory:");
    for key in ["n"] {
        if let Some(value) = vm.get_memory(key) {
            println!("  {} = {}", key, value);
        } else {
            println!("  {} is not set", key);
        }
    }

    Ok(())
} 