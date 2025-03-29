#[cfg(feature = "typed-values")]
mod tests {
    use crate::vm::{Op, VM};
    use crate::typed::TypedValue;
    
    #[test]
    fn test_typed_stack_operations() {
        let mut vm = VM::new();
        
        let ops = vec![
            Op::Push(TypedValue::Number(10.0)),
            Op::Push(TypedValue::Number(20.0)),
            Op::Add,
        ];
        
        vm.execute(&ops).unwrap();
        
        assert_eq!(vm.stack.len(), 1);
        if let Some(val) = vm.top() {
            if let TypedValue::Number(num) = val {
                assert_eq!(*num, 30.0);
            } else {
                panic!("Expected Number(30.0)");
            }
        } else {
            panic!("Stack is empty");
        }
    }
    
    #[test]
    fn test_string_operations() {
        let mut vm = VM::new();
        
        let ops = vec![
            Op::Push(TypedValue::String("Hello, ".to_string())),
            Op::Push(TypedValue::String("World!".to_string())),
            Op::Add,
        ];
        
        vm.execute(&ops).unwrap();
        
        assert_eq!(vm.stack.len(), 1);
        if let Some(val) = vm.top() {
            if let TypedValue::String(s) = val {
                assert_eq!(s, "Hello, World!");
            } else {
                panic!("Expected String(\"Hello, World!\")");
            }
        } else {
            panic!("Stack is empty");
        }
    }
    
    #[test]
    fn test_string_number_coercion() {
        let mut vm = VM::new();
        
        let ops = vec![
            Op::Push(TypedValue::String("Count: ".to_string())),
            Op::Push(TypedValue::Number(42.0)),
            Op::Add,
        ];
        
        vm.execute(&ops).unwrap();
        
        assert_eq!(vm.stack.len(), 1);
        if let Some(val) = vm.top() {
            if let TypedValue::String(s) = val {
                assert_eq!(s, "Count: 42");
            } else {
                panic!("Expected String(\"Count: 42\")");
            }
        } else {
            panic!("Stack is empty");
        }
    }
    
    #[test]
    fn test_boolean_operations() {
        let mut vm = VM::new();
        
        let ops = vec![
            Op::Push(TypedValue::Boolean(true)),
            Op::Push(TypedValue::Boolean(false)),
            Op::And,
        ];
        
        vm.execute(&ops).unwrap();
        
        assert_eq!(vm.stack.len(), 1);
        if let Some(val) = vm.top() {
            if let TypedValue::Boolean(b) = val {
                assert_eq!(*b, false);
            } else {
                panic!("Expected Boolean(false)");
            }
        } else {
            panic!("Stack is empty");
        }
    }
    
    #[test]
    fn test_string_repetition() {
        let mut vm = VM::new();
        
        let ops = vec![
            Op::Push(TypedValue::String("Na".to_string())),
            Op::Push(TypedValue::Number(4.0)),
            Op::Mul,
        ];
        
        vm.execute(&ops).unwrap();
        
        assert_eq!(vm.stack.len(), 1);
        if let Some(val) = vm.top() {
            if let TypedValue::String(s) = val {
                assert_eq!(s, "NaNaNaNa");
            } else {
                panic!("Expected String(\"NaNaNaNa\")");
            }
        } else {
            panic!("Stack is empty");
        }
    }
    
    #[test]
    fn test_null_equality() {
        let mut vm = VM::new();
        
        let ops = vec![
            Op::Push(TypedValue::Null),
            Op::Push(TypedValue::Null),
            Op::Eq,
        ];
        
        vm.execute(&ops).unwrap();
        
        assert_eq!(vm.stack.len(), 1);
        if let Some(val) = vm.top() {
            if let TypedValue::Boolean(b) = val {
                assert_eq!(*b, true);
            } else {
                panic!("Expected Boolean(true)");
            }
        } else {
            panic!("Stack is empty");
        }
    }
    
    #[test]
    fn test_if_with_boolean() {
        let mut vm = VM::new();
        
        let ops = vec![
            Op::If {
                condition: vec![
                    Op::Push(TypedValue::Boolean(true)),
                ],
                then: vec![
                    Op::Push(TypedValue::Number(42.0)),
                ],
                else_: Some(vec![
                    Op::Push(TypedValue::Number(24.0)),
                ]),
            },
        ];
        
        vm.execute(&ops).unwrap();
        
        assert_eq!(vm.stack.len(), 1);
        if let Some(val) = vm.top() {
            if let TypedValue::Number(num) = val {
                assert_eq!(*num, 42.0);
            } else {
                panic!("Expected Number(42.0)");
            }
        } else {
            panic!("Stack is empty");
        }
    }
} 