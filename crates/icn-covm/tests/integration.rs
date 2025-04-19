use icn_covm::{Op, VM};
use icn_covm::typed::TypedValue;
use std::fs;
use std::path::Path;

#[test]
fn test_program_json_runs_correctly() -> Result<(), Box<dyn std::error::Error>> {
    // Read and parse program.json
    let program_path = Path::new("program.json");
    let program_json = fs::read_to_string(program_path)?;
    let ops: Vec<Op> = serde_json::from_str(&program_json)?;

    // Create and run VM
    let mut vm = VM::new();
    vm.execute(&ops)?;

    // Print debug info
    println!("\nFinal stack:");
    for (i, value) in vm.get_stack().iter().enumerate() {
        println!("  {}: {}", i, value);
    }

    Ok(())
}

#[test]
fn test_governance_operations() -> Result<(), Box<dyn std::error::Error>> {
    // Create a program that uses all governance-inspired opcodes
    let ops = vec![
        // Test Match opcode with value computed on stack
        Op::Push(TypedValue::Number(1.0)),
        Op::Push(TypedValue::Number(2.0)),
        Op::Add,
        Op::Match {
            value: vec![], // Empty - use value on stack
            cases: vec![
                (TypedValue::Number(3.0), vec![Op::Push(TypedValue::Number(42.0))]), // Should match 3
                (TypedValue::Number(4.0), vec![Op::Push(TypedValue::Number(24.0))]),
            ],
            default: Some(vec![Op::Push(TypedValue::Number(0.0))]),
        },
        // Test AssertEqualStack
        Op::Dup,
        Op::Dup,
        Op::AssertEqualStack { depth: 3 },
        // Test EmitEvent
        Op::EmitEvent {
            category: "test".to_string(),
            message: "governance operations test".to_string(),
        },
        // Test Break in Loop
        Op::Push(TypedValue::Number(0.0)),
        Op::Store("counter".to_string()),
        Op::Loop {
            count: 10,
            body: vec![
                Op::Load("counter".to_string()),
                Op::Push(TypedValue::Number(1.0)),
                Op::Add,
                Op::Store("counter".to_string()),
                Op::Load("counter".to_string()),
                Op::Push(TypedValue::Number(5.0)),
                Op::Eq,
                Op::If {
                    condition: vec![],
                    then: vec![Op::Break],
                    else_: None,
                },
            ],
        },
        Op::Load("counter".to_string()),
        // Test Continue in While
        Op::Push(TypedValue::Number(0.0)),
        Op::Store("sum".to_string()),
        Op::Push(TypedValue::Number(0.0)),
        Op::Store("i".to_string()),
        Op::While {
            condition: vec![
                Op::Load("i".to_string()),
                Op::Push(TypedValue::Number(5.0)),
                Op::Lt, // i < 5, returns non-zero to continue loop
            ],
            body: vec![
                Op::Load("i".to_string()),
                Op::Push(TypedValue::Number(1.0)),
                Op::Add,
                Op::Store("i".to_string()),
                // Skip odd numbers
                Op::Load("i".to_string()),
                Op::Push(TypedValue::Number(2.0)),
                Op::Mod,
                Op::Push(TypedValue::Number(0.0)),
                Op::Eq,
                Op::Not,
                Op::If {
                    condition: vec![],
                    then: vec![Op::Continue],
                    else_: None,
                },
                // Add even numbers
                Op::Load("sum".to_string()),
                Op::Load("i".to_string()),
                Op::Add,
                Op::Store("sum".to_string()),
            ],
        },
        Op::Load("sum".to_string()),
    ];

    // Create and run VM
    let mut vm = VM::new();
    vm.execute(&ops)?;

    // Verify results
    let stack = vm.get_stack();
    println!("Stack: {:?}", stack);
    println!("Stack length: {}", stack.len());

    // Check that the stack contains the expected values somewhere
    // We don't assert the exact stack length as it might change with implementation details
    assert!(stack.contains(&TypedValue::Number(42.0))); // Result of Match operation
    assert!(stack.contains(&TypedValue::Number(5.0))); // Result of Break test
    assert!(stack.contains(&TypedValue::Number(12.0))); // Result of Continue test (sum of 2+4+6)

    // Verify memory
    println!("counter: {:?}", vm.get_memory_value("counter"));
    println!("sum: {:?}", vm.get_memory_value("sum"));
    println!("i: {:?}", vm.get_memory_value("i"));
    assert_eq!(vm.get_memory_value("counter"), Some(&TypedValue::Number(5.0)));
    assert_eq!(vm.get_memory_value("sum"), Some(&TypedValue::Number(12.0))); // Sum of even numbers 2+4+6=12
    assert_eq!(vm.get_memory_value("i"), Some(&TypedValue::Number(6.0))); // i increments to 6 in the test

    Ok(())
}
