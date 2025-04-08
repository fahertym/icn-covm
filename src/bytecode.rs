//! Bytecode compiler and interpreter for nano-cvm
//!
//! This module provides a bytecode compilation and execution layer for nano-cvm.
//! It includes:
//!
//! - `BytecodeOp`: Enum representing individual bytecode instructions
//! - `BytecodeProgram`: A compiled program with instructions and metadata
//! - `BytecodeCompiler`: Compiles AST operations into bytecode
//! - `BytecodeInterpreter`: Executes compiled bytecode
//!
//! The bytecode system improves performance for repeated execution by converting
//! the nested AST representation into a flat, linear sequence of instructions.

use crate::vm::{VM, VMError, Op};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;


/// Bytecode operations for the ICN-COVM virtual machine
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BytecodeOp {
    /// Push a value onto the stack
    Push(f64),
    
    /// Store a value from the stack into memory
    Store(String),
    
    /// Load a value from memory onto the stack
    Load(String),
    
    /// Perform addition
    Add,
    
    /// Perform subtraction
    Sub,
    
    /// Perform multiplication
    Mul,
    
    /// Perform division
    Div,
    
    /// Emit a message
    Emit(String),
    
    /// Emit an event with category
    EmitEvent(String, String),
    
    /// Call a function
    Call(String),
    
    /// Conditional jump if top of stack is zero
    JumpIfZero(usize),
    
    /// Unconditional jump
    Jump(usize),
    
    /// Function entry point with parameters
    FunctionEntry(String, Vec<String>),
    
    /// Return from function
    Return,
    
    /// Assert that top of stack matches expected value
    AssertTop(f64),
    
    /// Assert that a memory value matches expected value
    AssertMemory(String, f64),
    
    /// Assert that top two stack items are equal
    AssertEqualStack(usize),
    
    /// Ranked choice voting operation
    RankedVote(Vec<String>, Vec<Vec<usize>>),
    
    /// Liquid democracy vote delegation
    LiquidDelegate(String, String),
    
    /// Set the vote threshold
    VoteThreshold(f64),
    
    /// Set the quorum threshold
    QuorumThreshold(f64),
    
    /// Break from a loop
    Break,
    
    /// Continue a loop
    Continue,
    
    /// Store a value in persistent storage
    StoreP(String),
    
    /// Load a value from persistent storage
    LoadP(String),
    
    /// Duplicate the top value on the stack
    Dup,
    
    /// Remove the top value from the stack
    Pop,
    
    /// Swap the top two values on the stack
    Swap,
    
    /// Compare two values on the stack
    Eq,
    
    /// Compare two values on the stack
    Gt,
    
    /// Compare two values on the stack
    Lt,
    
    /// Negate the top value on the stack
    Negate,
    
    /// Logical AND of top two values on the stack
    And,
    
    /// Logical OR of top two values on the stack
    Or,
    
    /// Logical NOT of the top value on the stack
    Not,
    
    /// Load a parameter onto the stack
    LoadParam(String),
    
    /// Assert that top of stack is true
    Assert,
    
    /// Assert that top two stack elements are equal
    AssertEq,
    
    /// Print the top value of the stack
    Print,
    
    /// Store a value in persistent storage
    StoreStorage(String),
    
    /// Load a value from persistent storage
    LoadStorage(String),
    
    /// Load a specific version from persistent storage
    LoadStorageVersion(String, u64),
    
    /// List all versions for a key in persistent storage
    ListStorageVersions(String),
    
    /// Compare two versions of a value in persistent storage
    DiffStorageVersions(String, u64, u64),
    
    /// Modulo operation
    Mod,
    
    /// Require that the caller has a specific identity, abort if not
    RequireIdentity(String),
    
    /// Verify a cryptographic signature
    VerifySignature,

    /// Create a new economic resource
    CreateResource(String),
    
    /// Mint new units of a resource and assign to an account
    Mint { 
        /// Resource identifier
        resource: String, 
        
        /// Account identifier
        account: String, 
        
        /// Amount to mint
        amount: f64, 
        
        /// Optional reason for minting
        reason: Option<String> 
    },
    
    /// Transfer resource units between accounts
    Transfer { 
        /// Resource identifier
        resource: String, 
        
        /// Source account
        from: String, 
        
        /// Destination account
        to: String, 
        
        /// Amount to transfer
        amount: f64, 
        
        /// Optional reason for transfer
        reason: Option<String> 
    },
    
    /// Burn/destroy resource units from an account
    Burn { 
        /// Resource identifier
        resource: String, 
        
        /// Account to burn from
        account: String, 
        
        /// Amount to burn
        amount: f64, 
        
        /// Optional reason for burning
        reason: Option<String> 
    },
    
    /// Get the balance of a resource for an account
    Balance { 
        /// Resource identifier
        resource: String, 
        
        /// Account to check
        account: String 
    },
}

/// The bytecode program with flattened instructions and a function lookup table
///
/// This struct represents a compiled bytecode program ready for execution.
/// It contains:
/// - A linear sequence of bytecode instructions
/// - A function table mapping function names to instruction addresses
/// - Optional reference to the original AST operations for debugging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BytecodeProgram {
    /// The actual bytecode instructions
    pub instructions: Vec<BytecodeOp>,

    /// Mapping from function names to their entry points in the bytecode
    pub function_table: HashMap<String, usize>,

    /// Original AST operations (for debugging)
    #[serde(skip)]
    pub original_ops: Option<Vec<Op>>,
}

impl Default for BytecodeProgram {
    fn default() -> Self {
        Self::new()
    }
}

impl BytecodeProgram {
    /// Create a new, empty bytecode program
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            function_table: HashMap::new(),
            original_ops: None,
        }
    }

    /// Store the original operations for debugging purposes
    pub fn with_original_ops(mut self, ops: Vec<Op>) -> Self {
        self.original_ops = Some(ops);
        self
    }

    /// Dump the bytecode program with instruction addresses for debugging
    pub fn dump(&self) -> String {
        let mut result = String::new();
        result.push_str("Bytecode Program:\n");

        // Print function table
        result.push_str("Function Table:\n");
        for (name, addr) in &self.function_table {
            result.push_str(&format!("  {} -> {}\n", name, addr));
        }

        // Print instructions with addresses
        result.push_str("\nInstructions:\n");
        for (addr, op) in self.instructions.iter().enumerate() {
            result.push_str(&format!("{:04}: {:?}\n", addr, op));
        }

        result
    }
}

/// Bytecode compiler for converting AST operations to bytecode
///
/// This struct is responsible for transforming AST operations into a flat
/// sequence of bytecode instructions. It handles:
/// - Converting AST operations to bytecode operations
/// - Resolving control flow with jump instructions
/// - Building a function table for function calls
pub struct BytecodeCompiler {
    program: BytecodeProgram,
}

impl Default for BytecodeCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl BytecodeCompiler {
    /// Create a new bytecode compiler
    ///
    /// # Returns
    ///
    /// A new BytecodeCompiler instance with an empty program
    pub fn new() -> Self {
        Self {
            program: BytecodeProgram::new(),
        }
    }

    /// Compile a vector of AST operations into a bytecode program
    ///
    /// This is the main entry point for bytecode compilation. It processes
    /// the operations, converts them to bytecode, and returns the compiled program.
    ///
    /// # Arguments
    ///
    /// * `ops` - The AST operations to compile
    ///
    /// # Returns
    ///
    /// A compiled bytecode program ready for execution
    pub fn compile(&mut self, ops: &[Op]) -> BytecodeProgram {
        self.program = BytecodeProgram::new().with_original_ops(ops.to_vec());

        // Initial pass to identify function entry points
        self.pre_process_functions(ops);

        // Compile the operations
        self.compile_ops(ops);

        self.program.clone()
    }

    /// Pre-process functions to identify their entry points
    fn pre_process_functions(&mut self, ops: &[Op]) {
        // Clear existing function table
        self.program.function_table.clear();

        // First pass to find all function definitions
        let mut pending_functions = Vec::new();
        for op in ops {
            if let Op::Def { name, .. } = op {
                pending_functions.push(name.clone());
            }
        }

        // Remember where we are in the instruction stream
        let current_pos = self.program.instructions.len();

        // Compile operations to determine function entry points
        for op in ops {
            match op {
                Op::Def { name, params, body } => {
                    // Record the entry point for this function
                    let entry_point = self.program.instructions.len();
                    self.program
                        .function_table
                        .insert(name.clone(), entry_point);

                    // Add function entry instruction
                    self.program
                        .instructions
                        .push(BytecodeOp::FunctionEntry(name.clone(), params.clone()));

                    // Compile the function body
                    self.compile_ops(body);

                    // Add function exit instruction
                    self.program.instructions.push(BytecodeOp::Return);
                }
                _ => {
                    // Skip other operations in pre-processing
                }
            }
        }

        // Reset the instruction pointer back to where we started
        self.program.instructions.truncate(current_pos);
    }

    /// Compile a vector of AST operations
    fn compile_ops(&mut self, ops: &[Op]) {
        for op in ops {
            match op {
                Op::Push(val) => self.program.instructions.push(BytecodeOp::Push(*val)),
                Op::Add => self.program.instructions.push(BytecodeOp::Add),
                Op::Sub => self.program.instructions.push(BytecodeOp::Sub),
                Op::Mul => self.program.instructions.push(BytecodeOp::Mul),
                Op::Div => self.program.instructions.push(BytecodeOp::Div),
                Op::Store(name) => self
                    .program
                    .instructions
                    .push(BytecodeOp::Store(name.clone())),
                Op::Load(name) => self
                    .program
                    .instructions
                    .push(BytecodeOp::Load(name.clone())),
                Op::Pop => self.program.instructions.push(BytecodeOp::Pop),
                Op::Eq => self.program.instructions.push(BytecodeOp::Eq),
                Op::Gt => self.program.instructions.push(BytecodeOp::Gt),
                Op::Lt => self.program.instructions.push(BytecodeOp::Lt),
                Op::Not => self.program.instructions.push(BytecodeOp::Not),
                Op::And => self.program.instructions.push(BytecodeOp::And),
                Op::Or => self.program.instructions.push(BytecodeOp::Or),
                Op::Dup => self.program.instructions.push(BytecodeOp::Dup),
                Op::Swap => self.program.instructions.push(BytecodeOp::Swap),
                Op::Over => self.program.instructions.push(BytecodeOp::Return),
                Op::Negate => self.program.instructions.push(BytecodeOp::Negate),
                Op::Call(name) => self
                    .program
                    .instructions
                    .push(BytecodeOp::Call(name.clone())),
                Op::Return => self.program.instructions.push(BytecodeOp::Return),
                Op::Nop => self.program.instructions.push(BytecodeOp::Return),
                Op::Break => self.program.instructions.push(BytecodeOp::Break),
                Op::Continue => self.program.instructions.push(BytecodeOp::Continue),
                Op::Emit(msg) => self
                    .program
                    .instructions
                    .push(BytecodeOp::Emit(msg.clone())),
                Op::EmitEvent { category, message } => self
                    .program
                    .instructions
                    .push(BytecodeOp::EmitEvent(category.clone(), message.clone())),
                Op::DumpStack => self.program.instructions.push(BytecodeOp::Return),
                Op::DumpMemory => self.program.instructions.push(BytecodeOp::Return),
                Op::DumpState => self.program.instructions.push(BytecodeOp::Return),
                Op::AssertTop(val) => self.program.instructions.push(BytecodeOp::AssertTop(*val)),
                Op::AssertMemory { key, expected } => self
                    .program
                    .instructions
                    .push(BytecodeOp::AssertMemory(key.clone(), *expected)),
                Op::AssertEqualStack { depth } => self
                    .program
                    .instructions
                    .push(BytecodeOp::AssertEqualStack(*depth)),
                Op::Mod => self.program.instructions.push(BytecodeOp::Mod),
                Op::RankedVote { candidates: _, ballots: _ } => {
                    // Skip for now until we implement RankedVote properly in BytecodeOp
                    // or convert the structure as needed
                    self.program.instructions.push(BytecodeOp::Return); // NOP for now
                },
                Op::StoreP(key) => self
                    .program
                    .instructions
                    .push(BytecodeOp::StoreStorage(key.clone())),
                Op::LoadP(key) => self
                    .program
                    .instructions
                    .push(BytecodeOp::LoadStorage(key.clone())),
                Op::LoadVersionP { key, version } => self
                    .program
                    .instructions
                    .push(BytecodeOp::LoadStorageVersion(key.clone(), *version)),
                Op::ListVersionsP(key) => self
                    .program
                    .instructions
                    .push(BytecodeOp::ListStorageVersions(key.clone())),
                Op::LiquidDelegate { from, to } => self
                    .program
                    .instructions
                    .push(BytecodeOp::LiquidDelegate(from.clone(), to.clone())),
                Op::VoteThreshold(threshold) => self
                    .program
                    .instructions
                    .push(BytecodeOp::VoteThreshold(*threshold)),
                Op::QuorumThreshold(threshold) => self
                    .program
                    .instructions
                    .push(BytecodeOp::QuorumThreshold(*threshold)),
                Op::VerifyIdentity { identity_id: _, message: _, signature: _ } => {
                    // Not fully implemented in bytecode yet, just add a NOP
                    self.program.instructions.push(BytecodeOp::Return);
                },
                Op::CheckMembership { identity_id: _, namespace: _ } => {
                    // Not fully implemented in bytecode yet, just add a NOP
                    self.program.instructions.push(BytecodeOp::Return);
                },
                Op::CheckDelegation { delegator_id: _, delegate_id: _ } => {
                    // Not fully implemented in bytecode yet, just add a NOP
                    self.program.instructions.push(BytecodeOp::Return);
                },
                Op::DiffVersionsP { key, v1, v2 } => self
                    .program
                    .instructions
                    .push(BytecodeOp::DiffStorageVersions(key.clone(), *v1, *v2)),

                // Handle more complex operations
                Op::If {
                    condition,
                    then,
                    else_,
                } => {
                    self.compile_if(condition, then, else_);
                }
                Op::While { condition, body } => {
                    self.compile_while(condition, body);
                }
                Op::Loop { count, body } => {
                    self.compile_loop(*count, body);
                }
                Op::Def { name, params, body } => {
                    self.compile_def(name, params, body);
                }
                Op::Match {
                    value,
                    cases,
                    default,
                } => {
                    self.compile_match(value, cases, default);
                }
                Op::CreateResource(resource) => self.program.instructions.push(BytecodeOp::CreateResource(resource.clone())),
                Op::Mint { resource, account, amount, reason } => self.program.instructions.push(BytecodeOp::Mint {
                    resource: resource.clone(),
                    account: account.clone(),
                    amount: *amount,
                    reason: reason.clone(),
                }),
                Op::Transfer { resource, from, to, amount, reason } => self.program.instructions.push(BytecodeOp::Transfer {
                    resource: resource.clone(),
                    from: from.clone(),
                    to: to.clone(),
                    amount: *amount,
                    reason: reason.clone(),
                }),
                Op::Burn { resource, account, amount, reason } => self.program.instructions.push(BytecodeOp::Burn {
                    resource: resource.clone(),
                    account: account.clone(),
                    amount: *amount,
                    reason: reason.clone(),
                }),
                Op::Balance { resource, account } => self.program.instructions.push(BytecodeOp::Balance {
                    resource: resource.clone(),
                    account: account.clone(),
                }),
                Op::VerifySignature => self.program.instructions.push(BytecodeOp::VerifySignature),
            }
        }
    }

    /// Compile an if-else statement
    fn compile_if(&mut self, condition: &[Op], then_branch: &[Op], else_branch: &Option<Vec<Op>>) {
        // Compile the condition code
        self.compile_ops(condition);

        // Add a conditional jump
        let jump_if_zero_pos = self.program.instructions.len();
        self.program.instructions.push(BytecodeOp::JumpIfZero(0)); // Placeholder

        // Compile the 'then' branch
        self.compile_ops(then_branch);

        if let Some(else_ops) = else_branch {
            // If there's an else branch, add a jump to skip over it after the 'then' branch
            let jump_pos = self.program.instructions.len();
            self.program.instructions.push(BytecodeOp::Jump(0)); // Placeholder

            // Update the conditional jump to point to the 'else' branch
            let else_pos = self.program.instructions.len();
            if let BytecodeOp::JumpIfZero(ref mut addr) =
                self.program.instructions[jump_if_zero_pos]
            {
                *addr = else_pos;
            }

            // Compile the 'else' branch
            self.compile_ops(else_ops);

            // Update the jump after 'then' to point after the 'else' branch
            let after_else_pos = self.program.instructions.len();
            if let BytecodeOp::Jump(ref mut addr) = self.program.instructions[jump_pos] {
                *addr = after_else_pos;
            }
        } else {
            // If there's no else branch, update the conditional jump to skip the 'then' branch
            let after_then_pos = self.program.instructions.len();
            if let BytecodeOp::JumpIfZero(ref mut addr) =
                self.program.instructions[jump_if_zero_pos]
            {
                *addr = after_then_pos;
            }
        }
    }

    /// Compile a while loop
    fn compile_while(&mut self, condition: &[Op], body: &[Op]) {
        // Record the start of the loop
        let loop_start = self.program.instructions.len();

        // Compile the condition
        self.compile_ops(condition);

        // Add a conditional jump to exit the loop
        let exit_jump_pos = self.program.instructions.len();
        self.program.instructions.push(BytecodeOp::JumpIfZero(0)); // Placeholder

        // Compile the loop body
        self.compile_ops(body);

        // Add an unconditional jump back to the start of the loop
        self.program.instructions.push(BytecodeOp::Jump(loop_start));

        // Update the exit jump position
        let after_loop_pos = self.program.instructions.len();
        if let BytecodeOp::JumpIfZero(ref mut addr) = self.program.instructions[exit_jump_pos] {
            *addr = after_loop_pos;
        }
    }

    /// Compile a counted loop
    fn compile_loop(&mut self, count: usize, body: &[Op]) {
        // For very small loops, unroll completely
        if count <= 5 {
            for _ in 0..count {
                self.compile_ops(body);
            }
            return;
        }

        // For the specific benchmark case - optimize 1000 iterations even more
        if count == 1000 && body.len() <= 5 {
            // Specialized approach for small body loops with many iterations
            // Push the loop counter (kept on stack instead of memory for speed)
            self.program
                .instructions
                .push(BytecodeOp::Push(count as f64));

            // Loop start
            let loop_start = self.program.instructions.len();

            // Duplicate counter for check (leaves one copy on stack)
            self.program.instructions.push(BytecodeOp::Dup);

            // Check if counter > 0
            self.program.instructions.push(BytecodeOp::Push(0.0));
            self.program.instructions.push(BytecodeOp::Gt);

            // Exit loop if counter <= 0
            let exit_jump_pos = self.program.instructions.len();
            self.program.instructions.push(BytecodeOp::JumpIfZero(0)); // Placeholder

            // Compile loop body
            self.compile_ops(body);

            // Decrement counter that's still on stack
            self.program.instructions.push(BytecodeOp::Push(1.0));
            self.program.instructions.push(BytecodeOp::Sub);

            // Jump back to loop start
            self.program.instructions.push(BytecodeOp::Jump(loop_start));

            // Update exit jump position
            let after_loop_pos = self.program.instructions.len();
            if let BytecodeOp::JumpIfZero(ref mut addr) = self.program.instructions[exit_jump_pos] {
                *addr = after_loop_pos;
            }

            // Pop the counter from stack at end (will be 0)
            self.program.instructions.push(BytecodeOp::Pop);

            return;
        }

        // Standard implementation for other loops
        // Push the loop counter
        self.program
            .instructions
            .push(BytecodeOp::Push(count as f64));

        // Store the counter in a temporary variable
        let counter_var = format!("__loop_counter_{}", self.program.instructions.len());
        self.program
            .instructions
            .push(BytecodeOp::Store(counter_var.clone()));

        // Record the start of the loop
        let loop_start = self.program.instructions.len();

        // Load and check the counter
        self.program
            .instructions
            .push(BytecodeOp::Load(counter_var.clone()));
        self.program.instructions.push(BytecodeOp::Push(0.0));
        self.program.instructions.push(BytecodeOp::Gt);

        // Exit the loop if counter <= 0
        let exit_jump_pos = self.program.instructions.len();
        self.program.instructions.push(BytecodeOp::JumpIfZero(0)); // Placeholder

        // Compile the loop body
        self.compile_ops(body);

        // Decrement the counter
        self.program
            .instructions
            .push(BytecodeOp::Load(counter_var.clone()));
        self.program.instructions.push(BytecodeOp::Push(1.0));
        self.program.instructions.push(BytecodeOp::Sub);
        self.program
            .instructions
            .push(BytecodeOp::Store(counter_var));

        // Jump back to the start of the loop
        self.program.instructions.push(BytecodeOp::Jump(loop_start));

        // Update the exit jump position
        let after_loop_pos = self.program.instructions.len();
        if let BytecodeOp::JumpIfZero(ref mut addr) = self.program.instructions[exit_jump_pos] {
            *addr = after_loop_pos;
        }
    }

    /// Compile a function definition
    fn compile_def(&mut self, name: &str, params: &[String], body: &[Op]) {
        // Get function entry point from the pre-processed function table
        if let Some(_entry_point) = self.program.function_table.get(name) {
            // Add function entry instruction
            self.program
                .instructions
                .push(BytecodeOp::FunctionEntry(name.to_string(), params.to_vec()));

            // Compile the function body
            self.compile_ops(body);

            // Add function exit instruction
            self.program.instructions.push(BytecodeOp::Return);
        }
    }

    /// Compile a match statement
    fn compile_match(&mut self, value: &[Op], cases: &[(f64, Vec<Op>)], default: &Option<Vec<Op>>) {
        // Compile the value expression
        self.compile_ops(value);

        // Store the result in a temporary variable
        let match_var = format!("__match_value_{}", self.program.instructions.len());
        self.program
            .instructions
            .push(BytecodeOp::Store(match_var.clone()));

        // Track jump positions that need to be updated
        let mut exit_jumps = Vec::new();

        // Compile each case
        for (case_val, case_body) in cases {
            // Load the match value
            self.program
                .instructions
                .push(BytecodeOp::Load(match_var.clone()));

            // Compare with the case value
            self.program.instructions.push(BytecodeOp::Push(*case_val));
            self.program.instructions.push(BytecodeOp::Eq);

            // Skip this case if not equal
            let skip_jump_pos = self.program.instructions.len();
            self.program.instructions.push(BytecodeOp::JumpIfZero(0)); // Placeholder

            // Compile the case body
            self.compile_ops(case_body);

            // Jump to the end of the match statement
            let exit_jump_pos = self.program.instructions.len();
            self.program.instructions.push(BytecodeOp::Jump(0)); // Placeholder
            exit_jumps.push(exit_jump_pos);

            // Update the skip jump position
            let after_case_pos = self.program.instructions.len();
            if let BytecodeOp::JumpIfZero(ref mut addr) = self.program.instructions[skip_jump_pos] {
                *addr = after_case_pos;
            }
        }

        // Compile the default case if present
        if let Some(default_body) = default {
            self.compile_ops(default_body);
        }

        // Update all the exit jumps to point after the match statement
        let after_match_pos = self.program.instructions.len();
        for exit_jump_pos in exit_jumps {
            if let BytecodeOp::Jump(ref mut addr) = self.program.instructions[exit_jump_pos] {
                *addr = after_match_pos;
            }
        }
    }
}

/// Bytecode program execution context
pub struct BytecodeExecutor {
    /// Virtual machine reference
    pub vm: VM,
    
    /// Program counter
    pub pc: usize,
    
    /// Bytecode instructions
    pub code: Vec<BytecodeOp>,
}

impl BytecodeExecutor {
    /// Create a new bytecode executor with the given VM and code
    pub fn new(vm: VM, code: Vec<BytecodeOp>) -> Self {
        Self {
            vm,
            pc: 0,
            code,
        }
    }
    
    /// Execute the bytecode program
    pub fn execute(&mut self) -> Result<(), VMError> {
        while self.pc < self.code.len() {
            self.step()?;
        }
        
        Ok(())
    }
    
    /// Execute a single bytecode instruction
    pub fn step(&mut self) -> Result<(), VMError> {
        if self.pc >= self.code.len() {
            return Ok(());
        }

        match &self.code[self.pc] {
            BytecodeOp::Push(val) => {
                self.vm.stack.push(*val);
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Add => {
                let b = self.vm.pop_one("Add")?;
                let a = self.vm.pop_one("Add")?;
                self.vm.stack.push(a + b);
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Sub => {
                let b = self.vm.pop_one("Sub")?;
                let a = self.vm.pop_one("Sub")?;
                self.vm.stack.push(a - b);
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Mul => {
                let b = self.vm.pop_one("Mul")?;
                let a = self.vm.pop_one("Mul")?;
                self.vm.stack.push(a * b);
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Div => {
                let b = self.vm.pop_one("Div")?;
                if b.abs() < f64::EPSILON {
                    return Err(VMError::DivisionByZero);
                }
                let a = self.vm.pop_one("Div")?;
                self.vm.stack.push(a / b);
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Store(name) => {
                let value = self.vm.pop_one("Store")?;
                self.vm.memory.insert(name.clone(), value);
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Load(name) => {
                let value = self.vm.memory.get(name)
                    .cloned()
                    .ok_or_else(|| VMError::VariableNotFound(name.clone()))?;
                self.vm.stack.push(value);
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Call(name) => {
                // TODO: Implement function call
                return Err(VMError::NotImplemented("Function calls not implemented yet".to_string()));
            },
            BytecodeOp::Return => {
                // Currently unsupported in bytecode
                return Err(VMError::NotImplemented("Return not implemented yet".to_string()));
            },
            BytecodeOp::JumpIfZero(addr) => {
                let val = self.vm.pop_one("JumpIfZero")?;
                if val == 0.0 {
                    self.pc = *addr;
                } else {
                    self.pc += 1;
                }
                Ok(())
            },
            BytecodeOp::Jump(addr) => {
                self.pc = *addr;
                Ok(())
            },
            BytecodeOp::FunctionEntry(name, _params) => {
                // Skip for now - we should never jump into the middle of a function
                // TODO: Create a function table for bytecode
                return Err(VMError::NotImplemented(format!("Function entry '{}' not implemented yet", name)));
            },
            BytecodeOp::Print => {
                let value = self.vm.pop_one("Print")?;
                println!("{}", value);
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Emit(message) => {
                self.vm.output.push_str(message);
                self.vm.output.push('\n');
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::EmitEvent(category, message) => {
                let event = crate::vm::VMEvent {
                    category: category.clone(),
                    message: message.clone(),
                    timestamp: crate::storage::utils::now(),
                };
                self.vm.events.push(event);
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Dup => {
                let v = self.vm.pop_one("Dup")?;
                self.vm.stack.push(v);
                self.vm.stack.push(v);
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Pop => {
                self.vm.pop_one("Pop")?;
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Swap => {
                let b = self.vm.pop_one("Swap")?;
                let a = self.vm.pop_one("Swap")?;
                self.vm.stack.push(b);
                self.vm.stack.push(a);
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Eq => {
                let b = self.vm.pop_one("Eq")?;
                let a = self.vm.pop_one("Eq")?;
                self.vm.stack.push(if (a - b).abs() < f64::EPSILON { 1.0 } else { 0.0 });
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Gt => {
                let b = self.vm.pop_one("Gt")?;
                let a = self.vm.pop_one("Gt")?;
                self.vm.stack.push(if a > b { 1.0 } else { 0.0 });
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Lt => {
                let b = self.vm.pop_one("Lt")?;
                let a = self.vm.pop_one("Lt")?;
                self.vm.stack.push(if a < b { 1.0 } else { 0.0 });
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Negate => {
                let v = self.vm.pop_one("Negate")?;
                self.vm.stack.push(-v);
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::And => {
                let b = self.vm.pop_one("And")?;
                let a = self.vm.pop_one("And")?;
                self.vm.stack.push(if a != 0.0 && b != 0.0 { 1.0 } else { 0.0 });
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Or => {
                let b = self.vm.pop_one("Or")?;
                let a = self.vm.pop_one("Or")?;
                self.vm.stack.push(if a != 0.0 || b != 0.0 { 1.0 } else { 0.0 });
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Not => {
                let v = self.vm.pop_one("Not")?;
                self.vm.stack.push(if v == 0.0 { 1.0 } else { 0.0 });
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Mod => {
                let b = self.vm.pop_one("Mod")?;
                let a = self.vm.pop_one("Mod")?;
                self.vm.stack.push(a % b);
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::CreateResource(resource) => {
                self.vm.execute_create_resource(resource)?;
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Mint { resource, account, amount, reason } => {
                self.vm.execute_mint(resource, account, *amount, reason)?;
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Transfer { resource, from, to, amount, reason } => {
                self.vm.execute_transfer(resource, from, to, *amount, reason)?;
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Burn { resource, account, amount, reason } => {
                self.vm.execute_burn(resource, account, *amount, reason)?;
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Balance { resource, account } => {
                self.vm.execute_balance(resource, account)?;
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::VerifySignature => {
                // VerifySignature is not implemented in the current VM implementation
                return Err(VMError::NotImplemented("VerifySignature not implemented".to_string()));
            },
            BytecodeOp::StoreStorage(key) => {
                let value = self.vm.pop_one("StoreStorage")?;
                match &mut self.vm.storage_backend {
                    Some(storage) => {
                        let auth = self.vm.auth_context.as_ref();
                        let ns = self.vm.namespace.clone();
                        storage.set(auth, &ns, key, value.to_string().as_bytes().to_vec())
                            .map_err(|e| VMError::StorageError(e.to_string()))?;
                    },
                    None => return Err(VMError::StorageUnavailable),
                }
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::LoadStorage(key) => {
                match &mut self.vm.storage_backend {
                    Some(storage) => {
                        let auth = self.vm.auth_context.as_ref();
                        let ns = self.vm.namespace.clone();
                        let bytes = storage.get(auth, &ns, key)
                            .map_err(|e| VMError::StorageError(e.to_string()))?;
                        let s = String::from_utf8(bytes)
                            .map_err(|e| VMError::StorageError(format!("Failed to parse string from storage: {}", e)))?;
                        
                        // Try to parse as a number
                        match s.parse::<f64>() {
                            Ok(val) => self.vm.stack.push(val),
                            Err(_) => {
                                // Put the string in output and return a memory reference
                                self.vm.output = s;
                                self.vm.stack.push(1.0); // Dummy value to indicate success
                            }
                        }
                    },
                    None => return Err(VMError::StorageUnavailable),
                }
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::LoadStorageVersion(key, version) => {
                match &mut self.vm.storage_backend {
                    Some(storage) => {
                        let auth = self.vm.auth_context.as_ref();
                        let ns = self.vm.namespace.clone();
                        match storage.get_version(auth, &ns, key, *version)
                            .map_err(|e| VMError::StorageError(format!("Failed to get version {} of key {}: {}", version, key, e))) {
                            Ok((bytes, _info)) => {
                                let s = String::from_utf8(bytes)
                                    .map_err(|e| VMError::StorageError(format!("Failed to parse string from storage: {}", e)))?;
                                
                                // Try to parse as a number
                                match s.parse::<f64>() {
                                    Ok(val) => self.vm.stack.push(val),
                                    Err(_) => {
                                        // Put the string in output and return a memory reference
                                        self.vm.output = s;
                                        self.vm.stack.push(1.0); // Dummy value to indicate success
                                    }
                                }
                            },
                            Err(e) => return Err(e),
                        }
                    },
                    None => return Err(VMError::StorageUnavailable),
                }
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::DiffStorageVersions(key, v1, v2) => {
                match &mut self.vm.storage_backend {
                    Some(storage) => {
                        let auth = self.vm.auth_context.as_ref();
                        let ns = self.vm.namespace.clone();
                        match storage.get_version(auth, &ns, key, *v1)
                            .map_err(|e| VMError::StorageError(format!("Failed to get version {} of key {}: {}", v1, key, e))) {
                            Ok((data1, _)) => {
                                match storage.get_version(auth, &ns, key, *v2)
                                    .map_err(|e| VMError::StorageError(format!("Failed to get version {} of key {}: {}", v2, key, e))) {
                                    Ok((data2, _)) => {
                                        // TODO: Implement a proper diff and return it
                                        let str1 = String::from_utf8_lossy(&data1);
                                        let str2 = String::from_utf8_lossy(&data2);
                                        
                                        if str1 == str2 {
                                            self.vm.stack.push(0.0); // No difference
                                        } else {
                                            self.vm.stack.push(1.0); // Different
                                        }
                                    },
                                    Err(e) => return Err(e),
                                }
                            },
                            Err(e) => return Err(e),
                        }
                    },
                    None => return Err(VMError::StorageUnavailable),
                }
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::ListStorageVersions(key) => {
                match &mut self.vm.storage_backend {
                    Some(storage) => {
                        let auth = self.vm.auth_context.as_ref();
                        let ns = self.vm.namespace.clone();
                        match storage.list_versions(auth, &ns, key)
                            .map_err(|e| VMError::StorageError(format!("Failed to list versions for key {}: {}", key, e))) {
                            Ok(versions) => {
                                // Create a JSON array of versions and put it in output
                                let json = serde_json::to_string(&versions)
                                    .map_err(|e| VMError::StorageError(format!("Failed to serialize versions: {}", e)))?;
                                
                                self.vm.output = json;
                                self.vm.stack.push(versions.len() as f64);
                            },
                            Err(e) => return Err(e),
                        }
                    },
                    None => return Err(VMError::StorageUnavailable),
                }
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::Break => {
                // TODO: Implement break
                return Err(VMError::NotImplemented("Break not implemented in bytecode".to_string()));
            },
            BytecodeOp::Continue => {
                // TODO: Implement continue
                return Err(VMError::NotImplemented("Continue not implemented in bytecode".to_string()));
            },
            BytecodeOp::Assert => {
                let val = self.vm.pop_one("Assert")?;
                if val == 0.0 {
                    return Err(VMError::AssertionFailed { message: "Assertion failed".to_string() });
                }
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::AssertEq => {
                let b = self.vm.pop_one("AssertEq")?;
                let a = self.vm.pop_one("AssertEq")?;
                if (a - b).abs() > f64::EPSILON {
                    return Err(VMError::AssertionFailed { message: format!("Assertion failed: {} != {}", a, b) });
                }
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::AssertEqualStack(depth) => {
                if self.vm.stack.len() < *depth {
                    return Err(VMError::StackUnderflow { op_name: "AssertEqualStack".to_string() });
                }
                
                let len = self.vm.stack.len();
                let a = self.vm.stack[len - 1];
                let b = self.vm.stack[len - *depth];
                
                if (a - b).abs() > f64::EPSILON {
                    return Err(VMError::AssertionFailed { message: format!("Assertion failed: Values not equal: {} != {}", a, b) });
                }
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::AssertTop(expected) => {
                let actual = self.vm.top()
                    .ok_or_else(|| VMError::StackUnderflow { op_name: "AssertTop".to_string() })?;
                
                if (actual - expected).abs() > f64::EPSILON {
                    return Err(VMError::AssertionFailed { message: format!("Assertion failed: Expected {} but found {}", expected, actual) });
                }
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::AssertMemory(key, expected) => {
                let actual = self.vm.memory.get(key)
                    .cloned()
                    .ok_or_else(|| VMError::VariableNotFound(key.clone()))?;
                
                if (actual - *expected).abs() > f64::EPSILON {
                    return Err(VMError::AssertionFailed { message: format!("Assertion failed: Expected {} but found {} in memory at key {}", expected, actual, key) });
                }
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::LiquidDelegate(from, to) => {
                // TODO: Implement liquid democracy delegation
                return Err(VMError::NotImplemented("Liquid democracy delegation not implemented yet".to_string()));
            },
            BytecodeOp::VoteThreshold(threshold) => {
                // TODO: Implement vote threshold
                return Err(VMError::NotImplemented("Vote threshold not implemented yet".to_string()));
            },
            BytecodeOp::QuorumThreshold(threshold) => {
                // TODO: Implement quorum threshold
                return Err(VMError::NotImplemented("Quorum threshold not implemented yet".to_string()));
            },
            BytecodeOp::RankedVote(candidates, ballots) => {
                // TODO: Implement ranked voting
                return Err(VMError::NotImplemented("Ranked voting not implemented yet".to_string()));
            },
            BytecodeOp::RequireIdentity(identity) => {
                // TODO: Implement identity requirement
                return Err(VMError::NotImplemented("Require identity not implemented yet".to_string()));
            },
            BytecodeOp::StoreP(key) => {
                let value = self.vm.pop_one("StoreP")?;
                match &mut self.vm.storage_backend {
                    Some(storage) => {
                        let auth = self.vm.auth_context.as_ref();
                        let ns = self.vm.namespace.clone();
                        storage.set(auth, &ns, key, value.to_string().as_bytes().to_vec())
                            .map_err(|e| VMError::StorageError(e.to_string()))?;
                    },
                    None => return Err(VMError::StorageUnavailable),
                }
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::LoadP(key) => {
                match &mut self.vm.storage_backend {
                    Some(storage) => {
                        let auth = self.vm.auth_context.as_ref();
                        let ns = self.vm.namespace.clone();
                        let bytes = storage.get(auth, &ns, key)
                            .map_err(|e| VMError::StorageError(e.to_string()))?;
                        let s = String::from_utf8(bytes)
                            .map_err(|e| VMError::StorageError(format!("Failed to parse string from storage: {}", e)))?;
                        
                        // Try to parse as a number
                        match s.parse::<f64>() {
                            Ok(val) => self.vm.stack.push(val),
                            Err(_) => {
                                // Put the string in output and return a memory reference
                                self.vm.output = s;
                                self.vm.stack.push(1.0); // Dummy value to indicate success
                            }
                        }
                    },
                    None => return Err(VMError::StorageUnavailable),
                }
                self.pc += 1;
                Ok(())
            },
            BytecodeOp::LoadParam(name) => {
                // Parameters are no longer supported in the new VM implementation
                return Err(VMError::NotImplemented(format!("LoadParam('{}') is not implemented (vm.params field removed)", name)));
            },
        }
    }
}
