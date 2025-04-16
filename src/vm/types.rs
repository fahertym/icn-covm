//! Type definitions for the virtual machine
//!
//! This module contains the core data types used by the VM, including operations,
//! call frames, loop control, and events.
//!
//! Centralizing type definitions in this module:
//! - Establishes a single source of truth for VM data structures
//! - Prevents circular dependencies between modules
//! - Facilitates serialization and deserialization
//! - Makes the type system more maintainable
//! - Provides a clear boundary for extending VM functionality with new operations
//!
//! The primary types defined here include:
//! - `Op`: The main operation enum that defines all VM instructions
//! - `CallFrame`: Function call scope management
//! - `LoopControl`: Loop control flow signals
//! - `VMEvent`: Event structure for tracking VM activity

use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

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

    /// Minimum deliberation period before a proposal can be voted on
    ///
    /// This operation specifies how long a proposal must be in the deliberation
    /// state before it can transition to voting. It helps ensure that community
    /// members have adequate time to discuss the proposal.
    ///
    /// The deliberation period is represented as a Duration.
    MinDeliberation(Duration),

    /// Define when a proposal expires after being opened for voting
    ///
    /// This operation sets the timeframe within which votes must be cast.
    /// After this duration has passed, the proposal will automatically expire
    /// if it has not been executed.
    ///
    /// The expiration period is represented as a Duration.
    ExpiresIn(Duration),

    /// Require a specific role to participate in the proposal
    ///
    /// This operation specifies that only members with a certain role
    /// can vote on the proposal. It can be used to restrict voting to
    /// specific community members.
    RequireRole(String),

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

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Op::Push(val) => write!(f, "Push({})", val),
            Op::Add => write!(f, "Add"),
            Op::Sub => write!(f, "Sub"),
            Op::Mul => write!(f, "Mul"),
            Op::Div => write!(f, "Div"),
            Op::Mod => write!(f, "Mod"),
            Op::Store(name) => write!(f, "Store({})", name),
            Op::Load(name) => write!(f, "Load({})", name),
            Op::If { .. } => write!(f, "If"),
            Op::Loop { count, .. } => write!(f, "Loop({})", count),
            Op::While { .. } => write!(f, "While"),
            Op::Emit(msg) => write!(f, "Emit({})", msg),
            Op::Negate => write!(f, "Negate"),
            Op::AssertTop(val) => write!(f, "AssertTop({})", val),
            Op::DumpStack => write!(f, "DumpStack"),
            Op::DumpMemory => write!(f, "DumpMemory"),
            Op::AssertMemory { key, expected } => write!(f, "AssertMemory({}, {})", key, expected),
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
            Op::Def { name, .. } => write!(f, "Def({})", name),
            Op::Call(name) => write!(f, "Call({})", name),
            Op::Return => write!(f, "Return"),
            Op::Nop => write!(f, "Nop"),
            Op::Match { .. } => write!(f, "Match"),
            Op::Break => write!(f, "Break"),
            Op::Continue => write!(f, "Continue"),
            Op::EmitEvent { category, message } => write!(f, "EmitEvent({}, {})", category, message),
            Op::AssertEqualStack { depth } => write!(f, "AssertEqualStack({})", depth),
            Op::DumpState => write!(f, "DumpState"),
            Op::RankedVote { candidates, ballots } => {
                write!(f, "RankedVote({} candidates, {} ballots)", candidates, ballots)
            }
            Op::LiquidDelegate { from, to } => write!(f, "LiquidDelegate({} -> {})", from, to),
            Op::VoteThreshold(threshold) => write!(f, "VoteThreshold({})", threshold),
            Op::QuorumThreshold(threshold) => write!(f, "QuorumThreshold({})", threshold),
            Op::MinDeliberation(period) => write!(f, "MinDeliberation({:?})", period),
            Op::ExpiresIn(period) => write!(f, "ExpiresIn({:?})", period),
            Op::RequireRole(role) => write!(f, "RequireRole({})", role),
            Op::StoreP(key) => write!(f, "StoreP({})", key),
            Op::LoadP(key) => write!(f, "LoadP({})", key),
            Op::LoadVersionP { key, version } => write!(f, "LoadVersionP({}, v{})", key, version),
            Op::ListVersionsP(key) => write!(f, "ListVersionsP({})", key),
            Op::DiffVersionsP { key, v1, v2 } => write!(f, "DiffVersionsP({}, v{}, v{})", key, v1, v2),
            Op::VerifyIdentity { identity_id, .. } => write!(f, "VerifyIdentity({})", identity_id),
            Op::CheckMembership { identity_id, namespace } => {
                write!(f, "CheckMembership({}, {})", identity_id, namespace)
            }
            Op::CheckDelegation { delegator_id, delegate_id } => {
                write!(f, "CheckDelegation({} -> {})", delegator_id, delegate_id)
            }
            Op::VerifySignature => write!(f, "VerifySignature"),
            Op::CreateResource(resource) => write!(f, "CreateResource({})", resource),
            Op::Mint { resource, account, amount, .. } => {
                write!(f, "Mint({} of {} to {})", amount, resource, account)
            }
            Op::Transfer { resource, from, to, amount, .. } => {
                write!(f, "Transfer({} of {} from {} to {})", amount, resource, from, to)
            }
            Op::Burn { resource, account, amount, .. } => {
                write!(f, "Burn({} of {} from {})", amount, resource, account)
            }
            Op::Balance { resource, account } => write!(f, "Balance({} for {})", resource, account),
            Op::GetIdentity(id) => write!(f, "GetIdentity({})", id),
            Op::RequireValidSignature { voter, .. } => write!(f, "RequireValidSignature({})", voter),
            Op::IfPassed(_) => write!(f, "IfPassed"),
            Op::Else(_) => write!(f, "Else"),
            Op::IncrementReputation { identity_id, amount, .. } => {
                write!(f, "IncrementReputation({}, {:?})", identity_id, amount)
            }
            Op::Macro(name) => write!(f, "Macro({})", name),
        }
    }
}

/// A function call frame, containing the local memory scope and parameters
#[derive(Clone, Debug)]
pub struct CallFrame {
    /// Local memory for function scope
    pub memory: HashMap<String, f64>,
    
    /// Function parameters
    pub params: HashMap<String, f64>,
    
    /// Return value
    pub return_value: Option<f64>,
    
    /// Function name (for debugging)
    pub function_name: String,
}

/// Loop control signals used in execution
#[derive(Clone, Debug, PartialEq)]
pub enum LoopControl {
    /// No special control flow
    None,
    
    /// Break out of loop
    Break,
    
    /// Continue to next iteration
    Continue,
}

/// An event emitted by the VM during execution
#[derive(Clone, Debug)]
pub struct VMEvent {
    /// Category of the event
    pub category: String,

    /// Event message or payload
    pub message: String,

    /// Timestamp when the event occurred
    pub timestamp: u64,
} 