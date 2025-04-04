use icn_covm::vm::Op;
use icn_covm::vm::VM;

fn main() {
    println!("Testing basic VM operations");

    let mut vm = VM::new();

    // Basic stack operations
    let ops = vec![Op::Push(1.0), Op::Push(2.0), Op::Add];
    vm.execute(&ops).unwrap();
    println!("1 + 2 = {:?}", vm.stack.last());
    assert_eq!(vm.stack.last().copied(), Some(3.0));

    // More complex operations
    let ops = vec![
        Op::Push(5.0),
        Op::Push(3.0),
        Op::Sub,
        Op::Push(4.0),
        Op::Mul,
    ];
    vm.execute(&ops).unwrap();
    println!("(5 - 3) * 4 = {:?}", vm.stack.last());
    assert_eq!(vm.stack.last().copied(), Some(8.0));

    // Test memory operations
    let ops = vec![
        Op::Push(42.0),
        Op::Store("x".to_string()),
        Op::Load("x".to_string()),
    ];
    vm.execute(&ops).unwrap();
    println!("Store and load 42 = {:?}", vm.stack.last());
    assert_eq!(vm.stack.last().copied(), Some(42.0));

    // Test conditional operations
    let ops = vec![
        Op::Push(10.0),
        Op::Push(5.0),
        Op::Gt,
        Op::If {
            condition: vec![], // Empty condition means use the value on the stack
            then: vec![Op::Push(100.0)],
            else_: Some(vec![Op::Push(200.0)]),
        },
    ];
    vm.execute(&ops).unwrap();
    println!("If 10 > 5 then 100 else 200 = {:?}", vm.stack.last());
    assert_eq!(vm.stack.last().copied(), Some(100.0));

    println!("All tests passed!");
}
