#![allow(dead_code)] // Allow dead code during development

use crate::events::Event;
use crate::storage::auth::AuthContext;
use crate::storage::errors::{StorageError, StorageResult};
use crate::storage::traits::{StorageBackend, StorageExtensions, StorageTransaction}; // Import the combined Storage trait
use crate::storage::implementations::in_memory::InMemoryStorage;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use thiserror::Error;

// Import Clone for the generic bound
use std::marker::Send;
use std::marker::Sync;
use std::fmt::Debug;

use crate::identity::{Identity, Profile, IdentityError}; // Keep Identity, Profile, IdentityError
use crate::compiler::parse_dsl;
use crate::storage::{StorageTransaction};

/// Error variants that can occur during VM execution
#[derive(Debug, Error, Clone, PartialEq)]
pub enum VMError {
    /// Stack underflow occurs when trying to pop more values than are available
    #[error("Stack underflow during {op_name}")]
    StackUnderflow { op_name: String },

    /// Division by zero error
    #[error("Division by zero")]
    DivisionByZero,

    /// Error when a variable is not found in memory
    #[error("Variable not found: {0}")]
    VariableNotFound(String),

    /// Error when a function is not found
    #[error("Function not found: {0}")]
    FunctionNotFound(String),

    /// Error when maximum recursion depth is exceeded
    #[error("Maximum recursion depth exceeded")]
    MaxRecursionDepth,

    /// Error when a condition expression is invalid
    #[error("Invalid condition: {0}")]
    InvalidCondition(String),

    /// Error when an assertion fails
    #[error("Assertion failed: {message}")]
    AssertionFailed { message: String },

    /// I/O error during execution
    #[error("IO error: {0}")]
    IOError(String),

    /// Error in the REPL
    #[error("REPL error: {0}")]
    ReplError(String),

    /// Error with parameter handling
    #[error("Parameter error: {0}")]
    ParameterError(String),
    
    /// Loop control signal (break/continue)
    #[error("Loop control: {0}")]
    LoopControl(String),

    /// Feature not implemented
    #[error("Not implemented: {0}")]
    NotImplemented(String),

    /// Storage-related error
    #[error("Storage error: {0}")]
    StorageError(String),

    /// Storage backend is unavailable or not configured
    #[error("Storage backend is unavailable or not configured")]
    StorageUnavailable,

    /// Parameter not found
    #[error("Parameter not found: {0}")]
    ParameterNotFound(String),
    
    /// Identity not found
    #[error("Identity not found: {0}")]
    IdentityNotFound(String),
    
    /// Invalid signature
    #[error("Invalid signature for identity {identity_id}: {reason}")]
    InvalidSignature {
        identity_id: String,
        reason: String,
    },
    
    /// Membership check failed
    #[error("Membership check failed for identity {identity_id} in namespace {namespace}")]
    MembershipCheckFailed {
        identity_id: String,
        namespace: String,
    },
    
    /// Delegation check failed
    #[error("Delegation check failed from {delegator_id} to {delegate_id}")]
    DelegationCheckFailed {
        delegator_id: String,
        delegate_id: String,
    },
    
    /// Identity context unavailable
    #[error("Identity context unavailable")]
    IdentityContextUnavailable,

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),
}

/// Operation types for the virtual machine
///
/// The VM executes these operations in sequence, manipulating the stack,
/// memory, and control flow according to each operation's semantics.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Op {
    /// Push a numeric value onto the stack
    Push(f64),

    /// Pop two values, add them, and push the result
    Add,

    /// Pop two values, subtract the top from the second, and push the result
    Sub,

    /// Pop two values, multiply them, and push the result
    Mul,

    /// Pop two values, divide the second by the top, and push the result
    Div,

    /// Pop two values, compute the modulo of the second by the top, and push the result
    Mod,

    /// Pop a value and store it in memory with the given name
    Store(String),

    /// Load a value from memory and push it onto the stack
    Load(String),

    /// Conditional execution based on a condition
    ///
    /// The condition is evaluated, and if it's non-zero, the 'then' branch
    /// is executed. Otherwise, the 'else_' branch is executed if present.
    If {
        condition: Vec<Op>,
        then: Vec<Op>,
        else_: Option<Vec<Op>>,
    },

    /// Execute a block of operations a fixed number of times
    Loop { count: usize, body: Vec<Op> },

    /// Execute a block of operations while a condition is true
    While { condition: Vec<Op>, body: Vec<Op> },

    /// Emit a message to the output
    Emit(String),

    /// Negate the top value on the stack
    Negate,

    /// Assert that the top value on the stack equals the expected value
    AssertTop(f64),

    /// Display the current stack contents
    DumpStack,

    /// Display the current memory contents
    DumpMemory,

    /// Assert that a value in memory equals the expected value
    AssertMemory { key: String, expected: f64 },

    /// Pop a value from the stack
    Pop,

    /// Compare the top two values for equality
    Eq,

    /// Compare if the second value is greater than the top value
    Gt,

    /// Compare if the second value is less than the top value
    Lt,

    /// Logical NOT of the top value
    Not,

    /// Logical AND of the top two values
    And,

    /// Logical OR of the top two values
    Or,

    /// Duplicate the top value on the stack
    Dup,

    /// Swap the top two values on the stack
    Swap,

    /// Copy the second value to the top of the stack
    Over,

    /// Define a function with a name, parameters, and body
    Def {
        name: String,
        params: Vec<String>,
        body: Vec<Op>,
    },

    /// Call a named function
    Call(String),

    /// Return from a function
    Return,

    /// No operation, does nothing
    Nop,

    /// Match a value against several cases
    ///
    /// Evaluates 'value', then checks it against each case.
    /// If a match is found, executes the corresponding operations.
    /// If no match is found and a default is provided, executes the default.
    Match {
        value: Vec<Op>,
        cases: Vec<(f64, Vec<Op>)>,
        default: Option<Vec<Op>>,
    },

    /// Break out of the innermost loop
    Break,

    /// Continue to the next iteration of the innermost loop
    Continue,

    /// Emit an event with a category and message
    EmitEvent { category: String, message: String },

    /// Assert that all values in a depth of the stack are equal
    AssertEqualStack { depth: usize },

    /// Display the entire VM state
    DumpState,
    
    /// Execute a ranked-choice vote with candidates and ballots
    ///
    /// Pops a series of ballots from the stack, each containing ranked preferences.
    /// Each ballot is an array of candidate IDs in order of preference.
    /// The winner is determined using instant-runoff voting.
    /// The result is pushed back onto the stack.
    ///
    /// The number of candidates must be at least 2.
    /// The number of ballots must be at least 1.
    RankedVote {
        /// Number of candidates in the election
        candidates: usize,
        
        /// Number of ballots to process
        ballots: usize,
    },

    /// Delegate voting power from one member to another
    ///
    /// This operation creates a delegation relationship where the 'from' member
    /// delegates their voting rights to the 'to' member. The VM maintains a
    /// delegation graph and ensures there are no cycles.
    ///
    /// The delegation can be revoked by calling with an empty 'to' string.
    LiquidDelegate {
        /// The member delegating their vote
        from: String,
        
        /// The member receiving the delegation (or empty string to revoke)
        to: String,
    },

    /// Check if the total voting power meets a required threshold
    ///
    /// This operation compares the top value on the stack (total voting power)
    /// with the specified threshold. If the value is greater than or equal to
    /// the threshold, it pushes 0.0 (truthy) onto the stack; otherwise it pushes
    /// 1.0 (falsey).
    ///
    /// This is typically used for conditional execution in governance systems
    /// to ensure sufficient support before taking action.
    VoteThreshold(f64),

    /// Check if the participation meets a required quorum threshold
    ///
    /// This operation takes two values from the stack:
    /// 1. The top value is the total possible votes (from all eligible voters)
    /// 2. The second value is the total votes cast (actual participation)
    ///
    /// It compares the ratio of votes cast to possible votes against the
    /// specified threshold. If the participation ratio is greater than or equal to
    /// the threshold, it pushes 0.0 (truthy) onto the stack; otherwise it pushes
    /// 1.0 (falsey).
    ///
    /// This is typically used to ensure sufficient participation in governance
    /// decisions before accepting the results.
    QuorumThreshold(f64),

    /// Pop a value and store it in persistent storage with the given key
    ///
    /// This operation takes the top value from the stack and stores it in
    /// the persistent storage backend under the specified key.
    /// The value is removed from the stack.
    StoreP(String),

    /// Load a value from persistent storage and push it onto the stack
    ///
    /// This operation retrieves a value from the persistent storage backend
    /// using the specified key and pushes it onto the stack.
    /// If the key does not exist, an error is returned.
    LoadP(String),

    /// Load a specific version of a value from persistent storage
    ///
    /// This operation retrieves a specific version of a value from the
    /// persistent storage backend using the specified key and version number.
    /// The version is pushed onto the stack.
    /// If the key or version does not exist, an error is returned.
    LoadVersionP { key: String, version: u64 },

    /// List all versions of a value in persistent storage
    ///
    /// This operation retrieves a list of all available versions for a key
    /// in the persistent storage backend. It pushes the number of versions
    /// onto the stack, and emits version metadata like timestamps and authors.
    /// If the key does not exist, an error is returned.
    ListVersionsP(String),

    /// Compare two versions of a value in persistent storage
    ///
    /// This operation compares two specific versions of a value from the
    /// persistent storage backend using the specified key and version numbers.
    /// It emits information about differences between the versions and pushes
    /// the numeric difference onto the stack if the values are numeric.
    /// If the key or versions do not exist, an error is returned.
    DiffVersionsP { key: String, v1: u64, v2: u64 },

    /// Verify an identity's digital signature
    ///
    /// This operation checks if a digital signature is valid for a given
    /// identity. It requires the identity ID, the message that was signed,
    /// and the signature to verify.
    /// 
    /// Pushes 1.0 to the stack if the signature is valid, 0.0 otherwise.
    VerifyIdentity {
        /// The identity ID to verify against
        identity_id: String,
        
        /// The message that was signed (as a string)
        message: String,
        
        /// The signature to verify (base64 encoded)
        signature: String,
    },
    
    /// Check if an identity is a member of a cooperative or namespace
    /// 
    /// This operation verifies that the specified identity belongs to
    /// the given cooperative or namespace. It can be used to enforce
    /// membership-based access control.
    /// 
    /// Pushes 1.0 to the stack if the identity is a member, 0.0 otherwise.
    CheckMembership {
        /// The identity ID to check
        identity_id: String,
        
        /// The cooperative or namespace to check membership in
        namespace: String,
    },
    
    /// Check if one identity has delegated authority to another
    /// 
    /// This operation verifies that one identity (the delegator) has
    /// delegated authority to another identity (the delegate). It can
    /// be used to implement delegation chains and proxy actions.
    /// 
    /// Pushes 1.0 to the stack if the delegation exists, 0.0 otherwise.
    CheckDelegation {
        /// The delegating identity
        delegator_id: String,
        
        /// The identity receiving delegation
        delegate_id: String,
    },

    /// Verify a cryptographic signature
    /// 
    /// Pops: message, signature, public_key, scheme
    /// Pushes: 1.0 if valid, 0.0 if invalid
    VerifySignature,

    /// Create a new economic resource
    ///
    /// This operation creates a new economic resource with the specified identifier.
    /// The resource details should be stored in persistent storage.
    CreateResource(String),
    
    /// Mint new units of a resource and assign to an account
    ///
    /// This operation creates new units of an existing resource and
    /// assigns them to a specified account. It can be used for initial
    /// allocation or ongoing issuance of resources.
    Mint {
        /// Resource identifier
        resource: String,
        
        /// Account identifier
        account: String,
        
        /// Amount to mint
        amount: f64,
        
        /// Optional reason for minting
        reason: Option<String>,
    },
    
    /// Transfer resource units between accounts
    ///
    /// This operation moves units of a resource from one account to another.
    /// It checks that the source account has sufficient balance.
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
        reason: Option<String>,
    },
    
    /// Burn/destroy resource units from an account
    ///
    /// This operation removes units of a resource from circulation by
    /// "burning" them from a specified account.
    Burn {
        /// Resource identifier
        resource: String,
        
        /// Account to burn from
        account: String,
        
        /// Amount to burn
        amount: f64,
        
        /// Optional reason for burning
        reason: Option<String>,
    },
    
    /// Get the balance of a resource for an account
    ///
    /// This operation queries the current balance of a specified resource
    /// for a given account and pushes the result onto the stack.
    Balance {
        /// Resource identifier
        resource: String,
        
        /// Account to check
        account: String,
    },

    /// Get an identity from storage by its ID
    ///
    /// This operation retrieves an identity from storage using its ID.
    /// The identity information is made available for subsequent operations.
    /// If the identity doesn't exist, an error is returned.
    GetIdentity(String),
    
    /// Require a valid signature for a message
    ///
    /// This operation verifies that a signature is valid for a given message
    /// and was signed by the specified voter. It uses the public key from the
    /// voter's identity to verify the signature.
    /// If the signature is invalid, an error is returned and execution stops.
    RequireValidSignature {
        /// The voter ID whose signature should be verified
        voter: String,
        
        /// The message that was signed
        message: String,
        
        /// The signature to verify (base64 encoded)
        signature: String,
    },

    /// Execute a block of operations if a proposal passes
    ///
    /// This operation checks if a proposal has passed (met quorum and threshold)
    /// and executes the provided block of operations if it has.
    IfPassed(Vec<Op>),
    
    /// Execute a block of operations if a proposal fails
    ///
    /// This operation executes the provided block of operations if a proposal
    /// has failed (did not meet quorum or threshold).
    Else(Vec<Op>),

    /// Increment reputation for an identity
    ///
    /// This operation increments the reputation score for the specified identity.
    /// The reputation is stored in persistent storage and can be used for
    /// governance weighting and other reputation-based features.
    IncrementReputation {
        /// The identity ID to increment reputation for
        identity_id: String,
        
        /// The amount to increment by (default 1.0)
        amount: Option<f64>,
        
        /// The reason for the reputation increment
        reason: Option<String>,
    },

    /// Execute a macro
    ///
    /// This operation executes a macro, which is a special operation that
    /// expands into a sequence of other operations.
    #[serde(skip)]
    Macro(String),
}

impl std::fmt::Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Op::Push(v) => write!(f, "Push {}", v),
            Op::Add => write!(f, "Add"),
            Op::Sub => write!(f, "Sub"),
            Op::Mul => write!(f, "Mul"),
            Op::Div => write!(f, "Div"),
            Op::Mod => write!(f, "Mod"),
            Op::Store(key) => write!(f, "Store {}", key),
            Op::Load(key) => write!(f, "Load {}", key),
            Op::If { .. } => write!(f, "If {{ ... }}"), // Simplified representation for complex ops
            Op::Loop { count, .. } => write!(f, "Loop {} {{ ... }}", count),
            Op::While { .. } => write!(f, "While {{ ... }}"),
            Op::Emit(msg) => write!(f, "Emit \"{}\"", msg),
            Op::Negate => write!(f, "Negate"),
            Op::AssertTop(expected) => write!(f, "AssertTop {}", expected),
            Op::DumpStack => write!(f, "DumpStack"),
            Op::DumpMemory => write!(f, "DumpMemory"),
            Op::AssertMemory { key, expected } => write!(f, "AssertMemory {} == {}", key, expected),
            Op::Pop => write!(f, "Pop"),
            Op::Eq => write!(f, "Eq"),
            Op::Gt => write!(f, "Gt"),
            Op::Lt => write!(f, "Lt"),
            Op::Not => write!(f, "Not"),
            Op::And => write!(f, "And"),
            Op::Or => write!(f, "Or"),
            Op::Dup => write!(f, "Dup"),
            Op::Swap => write!(f, "Swap"),
            Op::Over => write!(f, "Over"),
            Op::Def { name, params, .. } => write!(f, "Def {}({}) {{ ... }}", name, params.join(", ")),
            Op::Call(name) => write!(f, "Call {}", name),
            Op::Return => write!(f, "Return"),
            Op::Nop => write!(f, "Nop"),
            Op::Match { .. } => write!(f, "Match {{ ... }}"),
            Op::Break => write!(f, "Break"),
            Op::Continue => write!(f, "Continue"),
            Op::EmitEvent { category, message } => write!(f, "EmitEvent Category: {}, Message: {}", category, message),
            Op::AssertEqualStack { depth } => write!(f, "AssertEqualStack {}", depth),
            Op::DumpState => write!(f, "DumpState"),
            Op::RankedVote { candidates, ballots } => write!(f, "RankedVote(cand={}, ballots={})", candidates, ballots),
            Op::LiquidDelegate { from, to } => write!(f, "LiquidDelegate {} -> {}", from, to),
            Op::VoteThreshold(t) => write!(f, "VoteThreshold {}", t),
            Op::QuorumThreshold(q) => write!(f, "QuorumThreshold {}", q),
            Op::StoreP(key) => write!(f, "StoreP {}", key),
            Op::LoadP(key) => write!(f, "LoadP {}", key),
            Op::LoadVersionP { key, version } => write!(f, "LoadVersionP {}:{}", key, version),
            Op::ListVersionsP(key) => write!(f, "ListVersionsP {}", key),
            Op::DiffVersionsP { key, v1, v2 } => write!(f, "DiffVersionsP {} v{}..v{}", key, v1, v2),
            Op::VerifyIdentity { identity_id, .. } => write!(f, "VerifyIdentity {}", identity_id),
            Op::CheckMembership { identity_id, namespace } => write!(f, "CheckMembership {} in {}", identity_id, namespace),
            Op::CheckDelegation { delegator_id, delegate_id } => write!(f, "CheckDelegation {} -> {}", delegator_id, delegate_id),
            Op::VerifySignature => write!(f, "VerifySignature"),
            Op::CreateResource(id) => write!(f, "CreateResource {}", id),
            Op::Mint { resource, account, amount, .. } => write!(f, "Mint {} {} to {}", amount, resource, account),
            Op::Transfer { resource, from, to, amount, .. } => write!(f, "Transfer {} {} from {} to {}", amount, resource, from, to),
            Op::Burn { resource, account, amount, .. } => write!(f, "Burn {} {} from {}", amount, resource, account),
            Op::Balance { resource, account } => write!(f, "Balance {} for {}", resource, account),
            Op::GetIdentity(id) => write!(f, "GetIdentity {}", id),
            Op::RequireValidSignature { voter, .. } => write!(f, "RequireValidSignature {}", voter),
            Op::IfPassed(..) => write!(f, "IfPassed {{ ... }}"),
            Op::Else(..) => write!(f, "Else {{ ... }}"),
            Op::IncrementReputation { identity_id, amount, reason } => {
                write!(f, "IncrementReputation Identity: {}, Amount: {:?}, Reason: {:?}",
                       identity_id, amount, reason)
            },
            Op::Macro(name) => write!(f, "Macro {}", name), // Note: Macro is #[serde(skip)], might not be needed depending on usage
        }
    }
}

#[derive(Debug)]
struct CallFrame {
    // Local memory for function scope
    memory: HashMap<String, f64>,
    // Function parameters
    params: HashMap<String, f64>,
    // Return value
    return_value: Option<f64>,
    // Function name (for debugging)
    function_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum LoopControl {
    None,
    Break,
    Continue,
}

/// Event emitted by the VM during execution
#[derive(Debug, Clone, PartialEq)]
pub struct VMEvent {
    /// Category of the event
    pub category: String,
    
    /// Event message or payload
    pub message: String,
    
    /// Timestamp when the event occurred
    pub timestamp: u64,
}

/// Virtual Machine for executing ICN-COVM bytecode
pub struct VM<S> // Make VM generic over storage type S
where
    S: Storage + Send + Sync + Clone + 'static, // Add Clone bound
{
    /// Stack for operands
    pub stack: Vec<f64>,
    
    /// Memory for storing variables
    pub memory: HashMap<String, f64>,
    
    /// Function map for storing subroutines
    pub functions: HashMap<String, (Vec<String>, Vec<Op>)>,
    
    /// Call stack for tracking function calls
    pub call_stack: Vec<usize>,
    
    /// Call frames for function memory scoping
    pub call_frames: Vec<CallFrame>,
    
    /// Output from the program
    pub output: String,
    
    /// Event log for recording significant actions
    pub events: Vec<VMEvent>,
    
    /// Authentication context for the current execution
    pub auth_context: Option<AuthContext>,
    
    /// Storage namespace for current execution
    pub namespace: String,
    
    // Store the concrete storage type S
    pub storage_backend: Option<S>,
    transaction_active: bool, // Keep track if WE started a transaction
}

// Implement VM for the generic S
impl<S> VM<S>
where
    S: Storage + Send + Sync + Clone + 'static,
{
    // VM::new - creates default InMemoryStorage if no backend provided initially
    // This needs rethinking. new() maybe shouldn't have storage?
    // Let's make new() NOT have storage, require with_storage_backend.
    pub fn new() -> Self {
         // Cannot create InMemoryStorage here as S is generic
         // Users must use with_storage_backend
         Self {
            stack: Vec::new(),
            memory: HashMap::new(),
            functions: HashMap::new(),
            call_stack: Vec::new(),
            call_frames: Vec::new(),
            output: String::new(),
            events: Vec::new(),
            auth_context: None,
            namespace: "default".to_string(),
            storage_backend: None, // No storage by default
            transaction_active: false,
        }
    }

    // Takes a concrete S
    pub fn with_storage_backend(backend: S) -> Self {
        let mut vm = Self::new();
        vm.storage_backend = Some(backend);
        vm
    }

    // Takes a concrete S
    pub fn set_storage_backend(&mut self, backend: S) {
        self.storage_backend = Some(backend);
    }
    
    // Need to expose these setters for tests/external use now that VM is generic
    pub fn set_auth_context(&mut self, auth: AuthContext) {
        self.auth_context = Some(auth);
    }
    pub fn set_namespace(&mut self, namespace: &str) {
        self.namespace = namespace.to_string();
    }
     pub fn get_auth_context(&self) -> Option<&AuthContext> {
        self.auth_context.as_ref()
    }

    // Storage operation helper now uses generic S
    fn storage_operation<F, T>(&mut self, operation_name: &str, mut f: F) -> Result<T, VMError>
    where
        F: FnMut(&mut S, Option<&AuthContext>, &str) -> StorageResult<(T, Option<VMEvent>)>,
    {
        if let Some(storage) = self.storage_backend.as_mut() {
            let auth_ctx = self.auth_context.as_ref();
            let namespace = &self.namespace;
            match f(storage, auth_ctx, namespace) { // Pass concrete &mut S
                 Ok((result, event_opt)) => {
                    if let Some(event) = event_opt {
                        // Need emit_event method
                        self.internal_emit_event(&event.category, &event.message);
                    }
                    Ok(result)
                }
                 Err(crate::storage::errors::StorageError::Unauthorized(op, ns, key)) => { // Assuming Unauthorized exists
                     let user = auth_ctx.map(|a| a.identity_did()).unwrap_or("anonymous");
                     Err(VMError::StorageError(format!(
                         "Unauthorized: User '{}' cannot perform '{}' on '{}/{}'",
                         user, op, ns, key.unwrap_or_default()
                     )))
                 }
                Err(e) => Err(VMError::StorageError(format!("Storage op '{}' failed: {}", operation_name, e))),
            }
        } else {
            Err(VMError::StorageUnavailable)
        }
    }
    
    // Internal helper for emitting events, needed by storage_operation
    fn internal_emit_event(&mut self, category: &str, message: &str) {
        use chrono::Utc; // Add import if not present
         let event = VMEvent {
            category: category.to_string(),
            message: message.to_string(),
            timestamp: Utc::now().timestamp_millis() as u64, 
        };
        println!("[EVENT:{}] {}", category, message);
        self.events.push(event);
    }

    // Fork implementation - now clones S
    pub fn fork(&mut self) -> Result<Self, VMError> { // Returns Self (VM<S>)
        println!("Forking VM...");
        let storage = self.storage_backend.as_mut()
            .ok_or(VMError::StorageUnavailable)?;
        
        // Begin transaction on original storage
        // Assuming begin_transaction doesn't need auth context based on E0061 fix needed later
        storage.begin_transaction() 
            .map_err(|e| VMError::StorageError(format!("Failed to begin transaction: {}", e)))?;
        self.transaction_active = true; 
        println!("Transaction begun on original VM storage.");

        // Clone the VM state, including the storage backend S
        Ok(Self { // Create new VM<S>
            stack: self.stack.clone(),
            memory: self.memory.clone(),
            functions: self.functions.clone(),
            call_stack: Vec::new(),
            call_frames: Vec::new(),
            output: String::new(),
            events: Vec::new(),
            auth_context: self.auth_context.clone(),
            namespace: self.namespace.clone(),
            storage_backend: self.storage_backend.clone(), // Clone the Option<S>
            transaction_active: true, // Fork operates within the transaction
        })
    }

     // Commit transaction (called on original VM)
    pub fn commit_fork_transaction(&mut self) -> Result<(), VMError> {
        if !self.transaction_active {
            return Err(VMError::StorageError("No active transaction to commit".to_string()));
        }
        println!("Committing transaction on original VM storage...");
        let storage = self.storage_backend.as_mut().ok_or(VMError::StorageUnavailable)?;
        // Assuming commit_transaction doesn't need auth context based on E0061 fix needed later
        storage.commit_transaction() 
            .map_err(|e| VMError::StorageError(format!("Failed to commit transaction: {}", e)))?;
        self.transaction_active = false;
        println!("Transaction committed.");
        Ok(())
    }

    // Rollback transaction (called on original VM)
    pub fn rollback_fork_transaction(&mut self) -> Result<(), VMError> {
        if !self.transaction_active {
            println!("No active transaction to roll back.");
            return Ok(()); 
        }
        println!("Rolling back transaction on original VM storage...");
        let storage = self.storage_backend.as_mut().ok_or(VMError::StorageUnavailable)?;
        // Assuming rollback_transaction doesn't need auth context based on E0061 fix needed later
        storage.rollback_transaction() 
             .map_err(|e| VMError::StorageError(format!("Failed to rollback transaction: {}", e)))?;
        self.transaction_active = false;
        println!("Transaction rolled back.");
        Ok(())
    }

    // Accessors previously available directly on VM
    pub fn top(&self) -> Option<f64> {
        self.stack.last().copied()
    }
    pub fn pop_one(&mut self, op_name: &str) -> Result<f64, VMError> {
        self.stack.pop().ok_or(VMError::StackUnderflow { op_name: op_name.to_string() })
    }
     pub fn pop_two(&mut self, op_name: &str) -> Result<(f64, f64), VMError> {
        if self.stack.len() < 2 {
            return Err(VMError::StackUnderflow { op_name: op_name.to_string() });
        }
        let top = self.stack.pop().unwrap();
        let second = self.stack.pop().unwrap();
        Ok((second, top))
    }

    // Execute method needs adapting
    pub fn execute(&mut self, ops: &[Op]) -> Result<(), VMError> {
        self.execute_inner(ops.to_vec())
    }

    // execute_inner needs adapting
    fn execute_inner(&mut self, ops: Vec<Op>) -> Result<(), VMError> {
        // Define a helper struct for deserializing the IncrementReputation payload
        #[derive(Deserialize)]
        struct ReputationIncrementPayload {
            identity_id: String,
            amount: u64,
        }

        for op in ops {
            match op {
                Op::Push(v) => { self.stack.push(*v); Ok(()) },
                Op::Add => { let (a, b) = self.pop_two("Add")?; self.stack.push(a + b); Ok(()) },
                Op::Sub => { let (a, b) = self.pop_two("Sub")?; self.stack.push(a - b); Ok(()) },
                Op::Mul => { let (a, b) = self.pop_two("Mul")?; self.stack.push(a * b); Ok(()) },
                Op::Div => {
                    let (a, b) = self.pop_two("Div")?;
                    if b == 0.0 { Err(VMError::DivisionByZero) } else { self.stack.push(a / b); Ok(()) }
                },
                Op::Mod => {
                     let (a, b) = self.pop_two("Mod")?;
                     if b == 0.0 { Err(VMError::DivisionByZero) } else { self.stack.push(a % b); Ok(()) }
                },
                Op::Store(key) => {
                    let value = self.pop_one("Store")?;
                    if let Some(frame) = self.call_frames.last_mut() {
                         frame.memory.insert(key.clone(), value);
                    } else {
                         self.memory.insert(key.clone(), value);
                    }
                     Ok(())
                },
                Op::Load(key) => {
                     let value = if let Some(frame) = self.call_frames.last() {
                         frame.memory.get(key).or_else(|| frame.params.get(key)).copied()
                     } else {
                         self.memory.get(key).copied()
                     };
                     match value {
                         Some(v) => { self.stack.push(v); Ok(()) }
                         None => Err(VMError::VariableNotFound(key.clone()))
                     }
                },
                 Op::Emit(msg) => { println!("{}", msg); self.output.push_str(msg); self.output.push('\n'); Ok(()) },
                 Op::Negate => { let v = self.pop_one("Negate")?; self.stack.push(-v); Ok(()) },
                 Op::AssertTop(expected) => { 
                     let v = self.pop_one("AssertTop")?; 
                     if (v - expected).abs() > 1e-9 { // Floating point comparison
                         Err(VMError::AssertionFailed { message: format!("AssertTop failed: expected {}, got {}", expected, v) }) 
                     } else { Ok(()) }
                 },
                 Op::DumpStack => { println!("Stack: {:?}", self.stack); Ok(()) },
                 Op::DumpMemory => { println!("Memory: {:?}", self.memory); Ok(()) }, // TODO: Dump call frame memory too
                 Op::AssertMemory { key, expected } => {
                    let value = self.memory.get(key).copied(); // TODO: Check call frame memory
                    match value {
                        Some(v) if (v - expected).abs() < 1e-9 => Ok(()),
                        Some(v) => Err(VMError::AssertionFailed { message: format!("AssertMemory failed for key '{}': expected {}, got {}", key, expected, v) }),
                        None => Err(VMError::AssertionFailed { message: format!("AssertMemory failed: key '{}' not found", key) })
                    }
                 },
                 Op::Pop => { self.pop_one("Pop")?; Ok(()) },
                 Op::Eq => { let (a, b) = self.pop_two("Eq")?; self.stack.push(if (a - b).abs() < 1e-9 { 1.0 } else { 0.0 }); Ok(()) },
                 Op::Gt => { let (a, b) = self.pop_two("Gt")?; self.stack.push(if a > b { 1.0 } else { 0.0 }); Ok(()) },
                 Op::Lt => { let (a, b) = self.pop_two("Lt")?; self.stack.push(if a < b { 1.0 } else { 0.0 }); Ok(()) },
                 Op::Not => { let v = self.pop_one("Not")?; self.stack.push(if v == 0.0 { 1.0 } else { 0.0 }); Ok(()) },
                 Op::And => { let (a, b) = self.pop_two("And")?; self.stack.push(if a != 0.0 && b != 0.0 { 1.0 } else { 0.0 }); Ok(()) },
                 Op::Or => { let (a, b) = self.pop_two("Or")?; self.stack.push(if a != 0.0 || b != 0.0 { 1.0 } else { 0.0 }); Ok(()) },
                 Op::Dup => { let v = self.top().ok_or(VMError::StackUnderflow { op_name: "Dup".to_string() })?; self.stack.push(v); Ok(()) },
                 Op::Swap => { let (a, b) = self.pop_two("Swap")?; self.stack.push(b); self.stack.push(a); Ok(()) },
                 Op::Over => { 
                     if self.stack.len() < 2 { return Err(VMError::StackUnderflow { op_name: "Over".to_string() }); }
                     let second = self.stack[self.stack.len() - 2]; self.stack.push(second); Ok(())
                 },
                 Op::Nop => Ok(()),
                 Op::Break => { return Ok(()); },
                 Op::Continue => { return Ok(()); },
                 Op::DumpState => { println!("{:#?}", self); Ok(()) }, // Requires VM to implement Debug
                 Op::Return => { 
                      if let Some(frame_idx) = self.call_stack.pop() {
                          let frame = self.call_frames.pop().unwrap(); // Should always exist if call_stack had entry
                          if let Some(ret_val) = frame.return_value {
                              self.stack.push(ret_val);
                          }
                           Ok(())
                      } else {
                           // Return from top level - effectively halt
                           println!("Return from top level.");
                           // This might need better handling - perhaps a Halt Op or specific error
                           return Ok(()); // Exit execute_inner successfully
                      }
                 },
                // --- Control Flow --- 
                Op::If { condition, then, else_ } => {
                    let cond_result = self.execute_conditional_block(&condition)?;
                    if cond_result != 0.0 {
                        self.execute_inner(then.clone())?;
                    } else if let Some(else_ops) = else_ {
                        self.execute_inner(else_ops.clone())?;
                    }
                    Ok(())
                },
                 Op::While { condition, body } => {
                    loop {
                         let cond_result = self.execute_conditional_block(&condition)?;
                         if cond_result == 0.0 { break; } // Condition false, exit loop

                         // Execute body, handling break/continue
                         let body_result = self.execute_inner(body.clone());
                          match body_result {
                            Ok(_) => {} // Continue loop
                            Err(VMError::LoopControl(ref ctrl)) if ctrl == "break" => break, // Break out of this loop
                            Err(VMError::LoopControl(ref ctrl)) if ctrl == "continue" => continue, // Continue to next iteration
                            Err(e) => return Err(e), // Propagate other errors
                          }
                    }
                     Ok(())
                 },
                Op::Loop { count, body } => {
                    for _ in 0..*count {
                        let body_result = self.execute_inner(body.clone());
                        match body_result {
                            Ok(_) => {} // Continue loop
                            Err(VMError::LoopControl(ref ctrl)) if ctrl == "break" => break, // Break out of this loop
                            Err(VMError::LoopControl(ref ctrl)) if ctrl == "continue" => continue, // Continue to next iteration
                            Err(e) => return Err(e), // Propagate other errors
                        }
                    }
                    Ok(())
                },
                 Op::Def { name, params, body } => {
                     self.functions.insert(name.clone(), (params.clone(), body.clone()));
                     Ok(())
                 },
                 Op::Call(name) => {
                     self.execute_call(&name)
                 },
                 Op::Match { value, cases, default } => {
                     let match_val = self.execute_conditional_block(&value)?;
                     let mut matched = false;
                     for (case_val, case_ops) in cases {
                         if (match_val - case_val).abs() < 1e-9 {
                             self.execute_inner(case_ops.clone())?;
                             matched = true;
                             break;
                         }
                     }
                     if !matched {
                         if let Some(default_ops) = default {
                             self.execute_inner(default_ops.clone())?;
                         }
                     }
                     Ok(())
                 },
                // --- Storage Ops --- (Adapted for generic S)
                Op::StoreP(key) => {
                    if let Some(storage) = self.storage_backend.as_mut() {
                        let value = self.pop_one("StoreP")?;
                         storage.set(self.auth_context.as_ref(), &self.namespace, key, value.to_string().as_bytes().to_vec())
                             .map_err(|e| VMError::StorageError(e.to_string()))
                    } else {
                         Err(VMError::StorageUnavailable)
                    }
                },
                Op::LoadP(key) => {
                     if let Some(storage) = self.storage_backend.as_ref() { // Use as_ref for read
                        let value_bytes = storage.get(self.auth_context.as_ref(), &self.namespace, key)
                            .map_err(|e| VMError::StorageError(e.to_string()))?;
                        let value_str = String::from_utf8(value_bytes).map_err(|e| VMError::StorageError(format!("LoadP failed decode: {}", e)))?;
                        let value = value_str.parse::<f64>().map_err(|e| VMError::StorageError(format!("LoadP failed parse: {}", e)))?;
                        self.stack.push(value);
                        Ok(())
                    } else {
                         Err(VMError::StorageUnavailable)
                    }
                },
                 Op::LoadVersionP { key, version } => {
                     if let Some(storage) = self.storage_backend.as_ref() {
                         match storage.get_version(self.auth_context.as_ref(), &self.namespace, key, *version) {
                             Ok((bytes, info)) => {
                                 // Fix println! formatting
                                 println!("[STORAGE] Loaded version {} (by {}, at {})", info.version, info.created_by, info.timestamp);
                                 let value_str = String::from_utf8(bytes).map_err(|e| VMError::StorageError(format!("LoadVersionP failed decode: {}", e)))?;
                                 let value = value_str.parse::<f64>().map_err(|e| VMError::StorageError(format!("LoadVersionP failed parse: {}", e)))?;
                                 self.stack.push(value);
                                 Ok(())
                             }
                             Err(e) => Err(VMError::StorageError(e.to_string()))
                         }
                    } else {
                         Err(VMError::StorageUnavailable)
                    }
                 },
                 Op::ListVersionsP(key) => {
                     if let Some(storage) = self.storage_backend.as_ref() {
                         match storage.list_versions(self.auth_context.as_ref(), &self.namespace, key) {
                             Ok(versions) => {
                                 println!("[STORAGE] Versions for key '{}':", key);
                                 for info in &versions {
                                      println!("  - Version: {}, By: {}, At: {}", info.version, info.created_by, info.timestamp);
                                 }
                                 self.stack.push(versions.len() as f64);
                                 Ok(())
                             }
                             Err(e) => Err(VMError::StorageError(e.to_string()))
                         }
                     } else {
                         Err(VMError::StorageUnavailable)
                     }
                 },
                 Op::DiffVersionsP { key, v1, v2 } => {
                      if let Some(storage) = self.storage_backend.as_ref() {
                         match storage.diff_versions(self.auth_context.as_ref(), &self.namespace, key, *v1, *v2) {
                             Ok(diff) => {
                                 println!("[STORAGE] Diff for key '{}' between v{} and v{}:", key, v1, v2);
                                 // Need to check diff contents, not call is_empty()
                                 let is_different = !diff.added.is_empty() || !diff.removed.is_empty(); 
                                 println!("  Differences found: {}", is_different);
                                 self.stack.push(if is_different { 1.0 } else { 0.0 }); 
                                 Ok(())
                             }
                             Err(e) => Err(VMError::StorageError(e.to_string()))
                         }
                     } else {
                         Err(VMError::StorageUnavailable)
                     }
                 },
                 // --- Event/Reputation Ops --- 
                 Op::EmitEvent { category, message } => {
                     self.internal_emit_event(&category, &message);
                     Ok(())
                 },
                  Op::IncrementReputation { identity_id, amount, reason } => {
                     // TODO: Pass reason to execute_increment_reputation if needed
                     let rep_amount = amount.unwrap_or(1.0); // Default increment is 1.0
                     self.execute_increment_reputation(&identity_id, rep_amount as u64)?;
                     Ok(())
                 },
                 // --- Auth/Identity Ops --- 
                  Op::RequireValidSignature { voter, message, signature } => {
                    // Need execute_require_valid_signature helper
                    println!("RequireValidSignature Op (Not Implemented)");
                    Ok(())
                 },
                   Op::VerifyIdentity { identity_id, message, signature } => {
                      // Need execute_verify_identity helper
                      println!("VerifyIdentity Op (Not Implemented)");
                      self.stack.push(1.0); // Assume valid for now
                      Ok(())
                   },
                   Op::CheckMembership { identity_id, namespace } => {
                      // Need execute_check_membership helper
                      println!("CheckMembership Op (Not Implemented): {} in {}", identity_id, namespace);
                      // Placeholder storage logic commented out
                      self.stack.push(1.0); // Assume member for now
                      Ok(())
                   },
                   Op::CheckDelegation { delegator_id, delegate_id } => {
                      // Need execute_check_delegation helper
                      println!("CheckDelegation Op (Not Implemented): {} -> {}", delegator_id, delegate_id);
                      // Placeholder storage logic commented out
                      self.stack.push(1.0); // Assume delegated for now
                      Ok(())
                   },
                   // --- Economic Ops --- 
                    Op::CreateResource(id) => {
                       // Need execute_create_resource helper
                       println!("CreateResource Op (Not Implemented): {}", id);
                       // Placeholder storage logic commented out
                       Ok(())
                   },
                   Op::Mint { resource, account, amount, reason } => {
                      // Need execute_mint helper
                      println!("Mint Op (Not Implemented): {} {} to {} (Reason: {:?})", amount, resource, account, reason);
                      // Placeholder storage logic commented out
                      Ok(())
                   },
                   Op::Transfer { resource, from, to, amount, reason } => {
                      // Need execute_transfer helper
                      println!("Transfer Op (Not Implemented): {} {} from {} to {} (Reason: {:?})", amount, resource, from, to, reason);
                      // Placeholder storage logic commented out
                      Ok(())
                   },
                   Op::Burn { resource, account, amount, reason } => {
                      // Need execute_burn helper
                      println!("Burn Op (Not Implemented): {} {} from {} (Reason: {:?})", amount, resource, account, reason);
                      // Placeholder storage logic commented out
                      Ok(())
                   },
                   Op::Balance { resource, account } => {
                      // Need execute_balance helper
                      println!("Balance Op (Not Implemented): {} for {}", resource, account);
                      // Placeholder storage logic commented out
                      self.stack.push(100.0); // Assume balance 100.0
                      Ok(())
                   },
                   // --- Other Ops --- 
                  Op::VerifySignature => {
                     // Placeholder logic - needs stack manipulation
                     println!("VerifySignature Op (Not Implemented)");
                     // Assume it pops 3, pushes 1 if valid
                     // let _sig = self.pop_one("VerifySignature")?; // Use pop_one
                     // let _msg = self.pop_one("VerifySignature")?;
                     // let _key = self.pop_one("VerifySignature")?; 
                     self.stack.push(1.0); // Assume valid for now
                     Ok(())
                  },
                Op::RankedVote { candidates, ballots } => {
                    // Assume candidates and ballots are keys to lists in storage?
                    // This needs a concrete implementation using storage.
                    println!("RankedVote: Candidates={}, Ballots={}", candidates, ballots);
                    // Placeholder: Push 1.0 (success) or 0.0 (failure) based on a hypothetical vote outcome
                    self.stack.push(1.0);
                    Ok(())
                },
                Op::LiquidDelegate { from, to } => {
                    // Needs storage interaction to record delegation
                    println!("LiquidDelegate: From={}, To={}", from, to);
                    // Placeholder storage logic commented out
                    Ok(())
                },
                Op::VoteThreshold(threshold) => {
                    self.stack.push(*threshold as f64);
                    Ok(())
                },
                Op::QuorumThreshold(quorum) => {
                    self.stack.push(*quorum as f64);
                    Ok(())
                },
                Op::AssertEqualStack { .. } => Err(VMError::NotImplemented("AssertEqualStack Op".to_string())),
                Op::IfPassed(_) => Err(VMError::NotImplemented("IfPassed Op".to_string())),
                Op::Else(_) => Err(VMError::NotImplemented("Else Op".to_string())),
                Op::Macro(_) => Err(VMError::NotImplemented("Macro Op should be expanded before execution".to_string())),

            }; // End match op
        }
        Ok(())
    }
    
     // Helper for conditional blocks (If, While, Match value)
     // Executes ops and expects a single value (0.0 for false, non-zero for true) on the stack.
     fn execute_conditional_block(&mut self, ops: &[Op]) -> Result<f64, VMError> {
         // Use a temporary stack to isolate execution
         let original_stack = std::mem::take(&mut self.stack);
         let result = self.execute_inner(ops.to_vec())?;
         let final_value = self.pop_one("Conditional Block");
         // Restore original stack
         self.stack = original_stack;

         result?; // Check for execution errors
         final_value // Return the value left on the temp stack
     }
     
     // Helper for calling functions
     fn execute_call(&mut self, name: &str) -> Result<(), VMError> {
        // ... (Implementation likely needs minimal changes if it uses stack/memory directly) ...
         Err(VMError::NotImplemented("execute_call needs review".to_string()))
     }

     // Helper methods like execute_increment_reputation need adapting
    pub fn execute_increment_reputation(&mut self, identity_id: &str, amount: u64) -> Result<(), VMError> {
        let increment = amount as f64;

        // Use storage_operation helper
        let current_score: i64 = self.storage_operation("GetReputation", |storage, auth, namespace| {
            let key = format!("scores/{}", identity_id);
            // Need to use the correct namespace "reputation" here
            let score = storage.get(auth, "reputation", &key) 
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            Ok((score, None))
        })?;

        let new_score = current_score + increment as i64; // Add increment

         println!(
            "[REPUTATION] Incrementing {} for {}. Reason: {}. New score: {}",
            identity_id, "reputation", increment, new_score
        );

        // Use storage_operation helper to set
        self.storage_operation("UpdateReputation", |storage, auth, namespace| {
            let reputation_key = format!("scores/{}", identity_id);
            // Need to use the correct namespace "reputation" here
            storage.set(auth, "reputation", &reputation_key, new_score.to_string().into_bytes())?;
            // Create event manually as storage_operation doesn't get event from set
             use chrono::Utc;
             let event = VMEvent {
                category: "reputation".to_string(),
                message: format!(
                    "Incremented score for {} to {}. Reason: {}",
                    identity_id, new_score, increment
                ),
                timestamp: Utc::now().timestamp_millis() as u64,
            };
            Ok(((), Some(event))) // Return unit and the event
        })?;

        Ok(())
    }

    // Add stubs or adapt other execute helpers similarly
    // pub fn execute_create_resource(&mut self, resource_id: &str) -> Result<(), VMError> { ... }
    // pub fn execute_mint(&mut self, resource: &str, account: &str, amount: f64, reason: &Option<String>) -> Result<(), VMError> { ... }
    // pub fn execute_transfer(&mut self, resource: &str, from: &str, to: &str, amount: f64, reason: &Option<String>) -> Result<(), VMError> { ... }
    // pub fn execute_burn(&mut self, resource: &str, account: &str, amount: f64, reason: &Option<String>) -> Result<(), VMError> { ... }
    // pub fn execute_balance(&mut self, resource: &str, account: &str) -> Result<(), VMError> { ... }
    // pub fn execute_get_identity(&mut self, id: &str) -> Result<(), VMError> { ... } 
    // pub fn execute_require_valid_signature(...) -> Result<(), VMError> { ... } 
    // pub fn execute_verify_identity(...) -> Result<(), VMError> { ... } 
    // pub fn execute_check_membership(...) -> Result<(), VMError> { ... } 
    // pub fn execute_check_delegation(...) -> Result<(), VMError> { ... } 

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::auth::AuthContext;
    use crate::identity::{Identity, Profile}; // Import Profile if needed, remove others for now
    // Add imports for key generation in tests
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;
    use did_key::generate;
    use did_key::DidKey;

    fn create_test_identity(id: &str, identity_type: &str) -> Identity {
        // Generate a new key pair for the test identity
        let mut csprng = OsRng{};
        let key_pair: SigningKey = SigningKey::generate(&mut csprng);

        // Create a DID using did:key method
        let did = generate::<DidKey>(&key_pair.verifying_key());

        // Construct the identity using the actual Identity::new
        // Assume public_username is derived from id, private name is None for tests
        let public_username = format!("{}_user", id);
        let mut identity = Identity::new(did.to_string(), public_username, None, key_pair)
                            .expect("Failed to create test identity"); // Assuming new can fail

        // Add metadata
        // identity.add_metadata("coop_id", "test_coop"); // add_metadata might not exist anymore

        identity
    }

    fn setup_identity_context() -> AuthContext {
        // Create an auth context with identities and roles
        let member_id = "member1";
        let test_identity = create_test_identity(member_id, "member");
        let mut auth = AuthContext::new(test_identity.did.clone()); // Use the actual DID
        
        // Add some roles
        auth.add_role("test_coop", "member");
        auth.add_role("coops/test_coop", "member");
        auth.add_role("coops/test_coop/proposals", "proposer");
        
        // Add identities to registry
        auth.register_identity(test_identity);
        auth.register_identity(create_test_identity("member2", "member"));
        auth.register_identity(create_test_identity("test_coop", "cooperative"));
        
        // Remove outdated test setup using Credential, DelegationLink, MemberProfile
        // These types are not currently defined in src/identity.rs

        auth
    }

    #[test]
    fn test_identity_verification() {
        let auth = setup_identity_context();
        let mut vm = VM::new();
        vm.set_auth_context(auth);
        vm.mock_storage_operations(); // Use mock storage for tests
        
        // Test verifying a signature (using the mock that always returns true if identity exists)
        let ops = vec![
            Op::VerifyIdentity { 
                identity_id: "member1".to_string(),
                message: "test message".to_string(),
                signature: "mock signature".to_string(),
            },
        ];
        
        vm.execute(&ops).unwrap();
        assert_eq!(vm.top(), Some(1.0)); // Should be true (1.0)
        
        // Test with non-existent identity
        let ops = vec![
            Op::VerifyIdentity { 
                identity_id: "nonexistent".to_string(),
                message: "test message".to_string(),
                signature: "mock signature".to_string(),
            },
        ];
        
        vm.execute(&ops).unwrap();
        assert_eq!(vm.top(), Some(0.0)); // Should be false (0.0)
    }

    #[test]
    fn test_membership_check() {
        let auth = setup_identity_context();
        let mut vm = VM::new();
        vm.set_auth_context(auth);
        vm.mock_storage_operations(); // Use mock storage for tests
        
        // Test checking membership in a namespace where the member belongs
        let ops = vec![
            Op::CheckMembership { 
                identity_id: "member1".to_string(),
                namespace: "coops/test_coop".to_string(),
            },
        ];
        
        vm.execute(&ops).unwrap();
        assert_eq!(vm.top(), Some(1.0)); // Should be true (1.0)
        
        // Test with a namespace where the member doesn't belong
        let ops = vec![
            Op::CheckMembership { 
                identity_id: "member1".to_string(),
                namespace: "coops/other_coop".to_string(),
            },
        ];
        
        vm.execute(&ops).unwrap();
        assert_eq!(vm.top(), Some(0.0)); // Should be false (0.0)
    }

    #[test]
    fn test_delegation_check() {
        let auth = setup_identity_context();
        let mut vm = VM::new();
        vm.set_auth_context(auth);
        vm.mock_storage_operations(); // Use mock storage for tests
        
        // Test checking a valid delegation
        let ops = vec![
            Op::CheckDelegation { 
                delegator_id: "member2".to_string(),
                delegate_id: "member1".to_string(),
            },
        ];
        
        vm.execute(&ops).unwrap();
        assert_eq!(vm.top(), Some(1.0)); // Should be true (1.0)
        
        // Test with invalid delegation
        let ops = vec![
            Op::CheckDelegation { 
                delegator_id: "member1".to_string(),
                delegate_id: "member2".to_string(),
            },
        ];
        
        vm.execute(&ops).unwrap();
        assert_eq!(vm.top(), Some(0.0)); // Should be false (0.0)
    }
    
    #[test]
    fn test_storage_operations_mock() {
        let mut vm = VM::new();
        
        // Create and set an auth context with proper permissions
        let mut auth = AuthContext::new("test_user");
        auth.add_role("global", "admin"); // Add admin role for global namespace
        auth.add_role("default", "writer"); // Add writer role for the default namespace
        auth.add_role("default", "reader"); // Add reader role for the default namespace
        vm.set_auth_context(auth);
        
        // Create a storage backend directly instead of using mock_storage_operations()
        let mut storage = InMemoryStorage::new();
        
        // Initialize storage with copied auth context
        let auth_context = vm.get_auth_context().unwrap().clone();
        storage.create_account(Some(&auth_context), "test_user", 1024 * 1024).unwrap();
        storage.create_namespace(Some(&auth_context), "default", 1024 * 1024, None).unwrap();
        
        // Set the storage backend
        vm.set_storage_backend(storage);
        vm.set_namespace("default");
        
        // Test storing and loading values
        let ops = vec![
            Op::Push(42.0),
            Op::StoreP("test_key".to_string()),
            Op::LoadP("test_key".to_string()),
        ];
        
        vm.execute(&ops).unwrap();
        assert_eq!(vm.top(), Some(42.0));
    }
}
