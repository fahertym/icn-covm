// Tests for TypedValue integration in the VM

use icn_covm::compiler::parse_dsl_with_stdlib;
use icn_covm::typed::TypedValue;
use icn_covm::vm::VM;

#[test]
fn test_push_and_emit_mixed_types() {
    let dsl = r#"
        # Push different types
        push 42.0
        push "hello"
        push true
        push null
        
        # Emit the values
        emit "Number: "
        emit
        emit "String: "
        emit
        emit "Boolean: "
        emit
        emit "Null: "
        emit
    "#;

    let program = parse_dsl_with_stdlib(dsl).unwrap();
    let mut vm = VM::new();

    vm.execute(&program).unwrap();

    let output = vm.get_output();

    // Check that all values were emitted correctly
    assert!(output.contains("Number: 42"));
    assert!(output.contains("String: hello"));
    assert!(output.contains("Boolean: true"));
    assert!(output.contains("Null: null"));
}

#[test]
fn test_if_with_boolean_control() {
    let dsl = r#"
        # Test with true condition
        push true
        if:
            push "Condition was true"
            emit
        else:
            push "Condition was false"
            emit
            
        # Test with false condition
        push false
        if:
            push "Condition was true"
            emit
        else:
            push "Condition was false"
            emit
            
        # Test with truthy number
        push 1.0
        if:
            push "Number 1.0 is truthy"
            emit
        else:
            push "Number 1.0 is not truthy"
            emit
            
        # Test with falsy number
        push 0.0
        if:
            push "Number 0.0 is truthy"
            emit
        else:
            push "Number 0.0 is not truthy"
            emit
    "#;

    let program = parse_dsl_with_stdlib(dsl).unwrap();
    let mut vm = VM::new();

    vm.execute(&program).unwrap();

    let output = vm.get_output();

    // Check that conditions were evaluated correctly
    assert!(output.contains("Condition was true"));
    assert!(output.contains("Condition was false"));
    assert!(output.contains("Number 1.0 is truthy"));
    assert!(output.contains("Number 0.0 is not truthy"));
}

#[test]
fn test_add_type_error_string_plus_number() {
    let dsl = r#"
        # Test string + number (should work as concatenation)
        push "Count: "
        push 42.0
        add
        emit
        
        # Test number + string (should work as concatenation)
        push 42.0
        push " is the answer"
        add
        emit
        
        # Test string + string
        push "Hello, "
        push "World!"
        add
        emit
    "#;

    let program = parse_dsl_with_stdlib(dsl).unwrap();
    let mut vm = VM::new();

    vm.execute(&program).unwrap();

    let output = vm.get_output();

    // Check that the operations worked correctly
    assert!(output.contains("Count: 42"));
    assert!(output.contains("42 is the answer"));
    assert!(output.contains("Hello, World!"));
}

#[test]
fn test_stack_order_mixed_types() {
    let dsl = r#"
        # Push values of different types
        push 1.0
        push "two"
        push true
        
        # Manipulate the stack
        dup  # Stack: 1.0, "two", true, true
        swap # Stack: 1.0, "two", true, true -> 1.0, "two", true, true
        over # Stack: 1.0, "two", true, true -> 1.0, "two", true, true, "two"
        
        # Emit values to check order
        emit
        emit
        emit
        emit
        emit
    "#;

    let program = parse_dsl_with_stdlib(dsl).unwrap();
    let mut vm = VM::new();

    vm.execute(&program).unwrap();

    let output = vm.get_output();
    let lines: Vec<&str> = output.lines().collect();

    // Check the emission order matches stack operations
    assert_eq!(lines.len(), 5);
    assert_eq!(lines[0], "two");
    assert_eq!(lines[1], "true");
    assert_eq!(lines[2], "true");
    assert_eq!(lines[3], "two");
    assert_eq!(lines[4], "1");
}

#[test]
fn test_memory_store_and_retrieve_typed() {
    let dsl = r#"
        # Store different types in memory
        push 42.0
        store number_val
        
        push "hello"
        store string_val
        
        push true
        store bool_val
        
        push null
        store null_val
        
        # Retrieve and emit values
        load number_val
        emit
        
        load string_val
        emit
        
        load bool_val
        emit
        
        load null_val
        emit
    "#;

    let program = parse_dsl_with_stdlib(dsl).unwrap();
    let mut vm = VM::new();

    vm.execute(&program).unwrap();

    let output = vm.get_output();
    let lines: Vec<&str> = output.lines().collect();

    // Check that values were stored and retrieved correctly
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], "42");
    assert_eq!(lines[1], "hello");
    assert_eq!(lines[2], "true");
    assert_eq!(lines[3], "null");

    // Check the memory map
    let memory = vm.get_memory_map();
    assert_eq!(memory.get("number_val"), Some(&TypedValue::Number(42.0)));
    assert_eq!(
        memory.get("string_val"),
        Some(&TypedValue::String("hello".to_string()))
    );
    assert_eq!(memory.get("bool_val"), Some(&TypedValue::Boolean(true)));
    assert_eq!(memory.get("null_val"), Some(&TypedValue::Null));
}

#[test]
fn test_comparison_operations() {
    let dsl = r#"
        # Test number comparisons
        push 10.0
        push 5.0
        gt  # 10 > 5 = true
        emit
        
        # Test string comparisons
        push "apple"
        push "banana"
        lt  # "apple" < "banana" = true
        emit
        
        # Test boolean comparisons
        push false
        push true
        lt  # false < true = true
        emit
        
        # Test equality
        push 42.0
        push 42.0
        eq  # 42 == 42 = true
        emit
        
        push "hello"
        push "hello"
        eq  # "hello" == "hello" = true
        emit
        
        push true
        push 1.0
        eq  # true == 1.0 = true (type coercion)
        emit
    "#;

    let program = parse_dsl_with_stdlib(dsl).unwrap();
    let mut vm = VM::new();

    vm.execute(&program).unwrap();

    let output = vm.get_output();
    let lines: Vec<&str> = output.lines().collect();

    // Check that comparisons work correctly
    assert_eq!(lines.len(), 6);
    assert_eq!(lines[0], "true"); // 10 > 5
    assert_eq!(lines[1], "true"); // "apple" < "banana"
    assert_eq!(lines[2], "true"); // false < true
    assert_eq!(lines[3], "true"); // 42 == 42
    assert_eq!(lines[4], "true"); // "hello" == "hello"
    assert_eq!(lines[5], "true"); // true == 1.0
}
