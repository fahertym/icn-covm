# TypedValue Migration Checklist

## Issues Addressed
- [x] Implemented `neg` method for TypedValue
- [x] Updated VM execution logic to use TypedValue for Op::Negate
- [x] Removed deprecated TypeError variant
- [x] Fixed AssertTop to use TypedValue equals instead of f64 subtraction
- [x] Fixed AssertMemory to use TypedValue equals instead of f64 subtraction
- [x] Fixed Match operation to use TypedValue equals
- [x] Fixed Return operation to use TypedValue instead of f64
- [x] Fixed economic operations (Mint, Transfer, Burn) to use TypedValue properly

## Remaining Issues

### Execution.rs
- [ ] Fix execute_increment_reputation to use proper error handling for TypedValue

### VM.rs
- [ ] Update any remaining uses of f64 to TypedValue
- [ ] Ensure all stack operations use TypedValue
- [ ] Fix any remaining arithmetic operations using direct f64 operations

### Tests
- [ ] Update test assertions comparing f64 values to use TypedValue
- [ ] Ensure tests account for all TypedValue variants

### VMError and TypedValueError
- [ ] Fix VMError variants related to TypedValue operations
- [ ] Ensure consistent error handling between TypedValueError and VMError

### Other Components
- [ ] Update governance modules to use TypedValue operations
- [ ] Update compiler modules to handle TypedValue
- [ ] Update bytecode interpreter to handle TypedValue

## Validation Steps
- [ ] Run `cargo check --all-targets`
- [ ] Run `cargo test --all-features`
- [ ] Run `cargo fmt -- --check`
- [ ] Run `cargo clippy --all-features -- -D warnings`

## Common Conversions
When migrating from f64 to TypedValue, use these patterns:

1. For literals:
   ```rust
   // Old
   let value = 1.0;
   
   // New
   let value = TypedValue::Number(1.0);
   ```

2. For arithmetic:
   ```rust
   // Old
   let result = a + b;
   
   // New
   let result = a.add(&b).map_err(|e| VMError::TypeMismatch {
       expected: "Number".to_string(),
       found: format!("{} and {}", a.type_name(), b.type_name()),
       operation: "addition".to_string(),
   })?;
   ```

3. For comparisons:
   ```rust
   // Old
   if a == b {
   
   // New
   let equals_result = a.equals(&b)?;
   if equals_result == TypedValue::Boolean(true) {
   ```

4. For stack operations:
   ```rust
   // Old
   stack.push(1.0);
   
   // New
   stack.push(TypedValue::Number(1.0));
   ```

5. For error handling:
   ```rust
   // Old
   return Err(VMError::TypeError { ... });
   
   // New
   return Err(VMError::TypeMismatch { ... });
   ``` 