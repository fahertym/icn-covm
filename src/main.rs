mod vm;
mod test_runner;
use std::fs;
use std::path::Path;
use vm::{Op, VM};
use test_runner::TestRunner;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() > 1 && args[1] == "test" {
        // Run tests
        let runner = TestRunner::new("tests");
        runner.run_all_tests()?;
    } else {
        // Run normal program
        let program_path = Path::new("program.json");
        let program_json = fs::read_to_string(program_path)?;
        let ops: Vec<Op> = serde_json::from_str(&program_json)?;

        let mut vm = VM::new();
        vm.execute(&ops)?;

        println!("Final stack:");
        for (i, &value) in vm.stack.iter().enumerate() {
            println!("  {}: {}", i, value);
        }
    }

    Ok(())
}
