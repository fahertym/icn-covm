use serde::Deserialize;
use std::fs;
use std::path::Path;
use crate::vm::{Op, VM};

#[derive(Debug, Deserialize)]
pub struct TestCase {
    name: String,
    description: String,
    program: Vec<Op>,
    assertions: TestAssertions,
}

#[derive(Debug, Deserialize)]
pub struct TestAssertions {
    final_stack: Option<Vec<f64>>,
}

pub struct TestRunner {
    test_dir: String,
}

impl TestRunner {
    pub fn new(test_dir: &str) -> Self {
        TestRunner {
            test_dir: test_dir.to_string(),
        }
    }

    pub fn run_all_tests(&self) -> Result<(), Box<dyn std::error::Error>> {
        let test_files = fs::read_dir(&self.test_dir)?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().map_or(false, |ext| ext == "json"));

        let mut passed = 0;
        let mut failed = 0;

        for entry in test_files {
            let path = entry.path();
            println!("\nRunning test: {}", path.display());
            
            match self.run_test(&path) {
                Ok(_) => {
                    println!("✅ Test passed");
                    passed += 1;
                }
                Err(e) => {
                    println!("❌ Test failed: {}", e);
                    failed += 1;
                }
            }
        }

        println!("\nTest Summary:");
        println!("  Passed: {}", passed);
        println!("  Failed: {}", failed);
        println!("  Total:  {}", passed + failed);

        if failed > 0 {
            Err("Some tests failed".into())
        } else {
            Ok(())
        }
    }

    fn run_test(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let test_case: TestCase = serde_json::from_str(&contents)?;

        println!("  Name: {}", test_case.name);
        println!("  Description: {}", test_case.description);

        let mut vm = VM::new();
        vm.execute(&test_case.program)?;

        // Check final stack assertions
        if let Some(expected_stack) = test_case.assertions.final_stack {
            if vm.stack != expected_stack {
                return Err(format!(
                    "Stack mismatch:\n  Expected: {:?}\n  Got:      {:?}",
                    expected_stack, vm.stack
                ).into());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_loads_test_file() {
        let runner = TestRunner::new("tests");
        let result = runner.run_all_tests();
        assert!(result.is_ok());
    }
} 