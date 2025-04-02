# Cooperative Contract Language (CCL) Grammar Specification

## Introduction

The Cooperative Contract Language (CCL) is a domain-specific language designed for expressing governance logic, voting mechanisms, and democratic decision-making processes in cooperatives. CCL is executed on the ICN Cooperative Virtual Machine (ICN-COVM), which interprets and executes the governance logic defined in CCL programs.

This document provides a formal specification of the CCL grammar, including its syntax, semantic rules, and execution model.

## Language Overview

CCL is a stack-based language with block-structured control flow, designed to balance human readability with precise execution semantics. It features:

- Stack-based execution for data manipulation
- Block-structured control flow with indentation-based scoping
- Specialized primitives for cooperative governance
- Memory operations for storing and retrieving values
- Function definitions and calls with memory isolation

## Lexical Structure

### Comments

Comments in CCL start with a hash character (`#`) and continue to the end of the line:

```
# This is a comment
push 1.0  # This is an inline comment
```

### Literals

CCL supports the following literals:

- **Numbers**: Floating-point numbers (e.g., `42.0`, `3.14`, `-1.5`)
- **Strings**: Text enclosed in double quotes (e.g., `"hello"`, `"alice"`)

### Identifiers

Identifiers are used for variable names, function names, and other named entities:

- Must start with a letter or underscore
- Can contain letters, digits, and underscores
- Case-sensitive

Examples: `counter`, `alice_vote`, `total_power`

### Keywords

Keywords are reserved and cannot be used as identifiers:

```
push, pop, add, sub, mul, div, mod, store, load, if, else, while, loop, break, continue, 
return, emit, emitevent, def, call, match, negate, and, or, not, eq, gt, lt, dup, swap, 
over, liquiddelegate, rankedvote, votethreshold, quorumthreshold
```

## Syntax

CCL uses indentation (similar to Python) to denote blocks of code. Each block starts with a colon (`:`) and is indented relative to its parent.

### Basic Commands

```
push <number>              # Push a number onto the stack
push <string>              # Push a string onto the stack (internally converted to an ID)
pop                        # Remove the top value from the stack
store <name>               # Pop a value and store it in memory with the given name
load <name>                # Push the value of a variable onto the stack
emit <string>              # Output a string to the console
emitevent <category> <msg> # Emit a categorized event
```

### Arithmetic and Logic

```
add       # Pop two values, add them, push the result
sub       # Pop two values a and b, push (b - a)
mul       # Pop two values, multiply them, push the result
div       # Pop two values a and b, push (b / a)
mod       # Pop two values a and b, push (b % a)
negate    # Pop a value, push its negation
eq        # Pop two values, push 1.0 if equal, 0.0 if not
gt        # Pop two values a and b, push 1.0 if b > a, 0.0 if not
lt        # Pop two values a and b, push 1.0 if b < a, 0.0 if not
and       # Pop two values, push 1.0 if both non-zero, 0.0 otherwise
or        # Pop two values, push 1.0 if either non-zero, 0.0 otherwise
not       # Pop a value, push 1.0 if it's 0.0, 0.0 otherwise
```

### Stack Manipulation

```
dup       # Duplicate the top value on the stack
swap      # Swap the top two values on the stack
over      # Copy the second value to the top of the stack
```

### Control Flow

```
if:
    # Code executed if top of stack is 0.0 (truthy)
else:
    # Code executed if top of stack is not 0.0 (falsey)

while:
    # Condition code (must leave a value on the stack)
:do
    # Body executed while condition is truthy

loop <count>:
    # Body executed <count> times

break      # Exit the innermost loop
continue   # Skip to the next iteration of the innermost loop

match:
    case <value>:
        # Code executed if top of stack equals <value>
    case <value>:
        # Code executed if top of stack equals <value>
    default:
        # Code executed if no case matches
```

### Functions

```
def <name>(<param1>, <param2>, ...):
    # Function body
    return  # Optional explicit return

call <name>  # Call a function
```

### Governance Operations

```
liquiddelegate <from> <to>            # Delegate voting power from one member to another
rankedvote <candidates> <ballots>     # Conduct a ranked-choice vote
votethreshold <threshold>             # Check if support meets a threshold
quorumthreshold <threshold>           # Check if participation meets a threshold
```

### Debug Operations

```
dumpstack    # Display the contents of the stack
dumpmemory   # Display the contents of memory
asserttop <value>  # Assert that the top of the stack equals <value>
```

## Formal Grammar

The following grammar is specified in Extended Backus-Naur Form (EBNF):

```ebnf
program        ::= statement*

statement      ::= simple_statement | compound_statement

simple_statement ::= 
                  push_stmt | 
                  pop_stmt | 
                  store_stmt | 
                  load_stmt | 
                  arithmetic_stmt | 
                  logic_stmt | 
                  stack_stmt | 
                  emit_stmt | 
                  function_call_stmt |
                  delegate_stmt |
                  vote_stmt |
                  threshold_stmt |
                  debug_stmt |
                  COMMENT

compound_statement ::= 
                  if_stmt | 
                  while_stmt | 
                  loop_stmt | 
                  match_stmt | 
                  function_def_stmt

push_stmt      ::= "push" (NUMBER | STRING)
pop_stmt       ::= "pop"
store_stmt     ::= "store" IDENTIFIER
load_stmt      ::= "load" IDENTIFIER
arithmetic_stmt ::= "add" | "sub" | "mul" | "div" | "mod" | "negate"
logic_stmt     ::= "eq" | "gt" | "lt" | "and" | "or" | "not"
stack_stmt     ::= "dup" | "swap" | "over"
emit_stmt      ::= "emit" STRING | "emitevent" STRING STRING
function_call_stmt ::= "call" IDENTIFIER
delegate_stmt  ::= "liquiddelegate" STRING STRING
vote_stmt      ::= "rankedvote" NUMBER NUMBER
threshold_stmt ::= "votethreshold" NUMBER | "quorumthreshold" NUMBER
debug_stmt     ::= "dumpstack" | "dumpmemory" | "asserttop" NUMBER

if_stmt        ::= "if" ":" INDENT statement+ DEDENT 
                  ["else" ":" INDENT statement+ DEDENT]

while_stmt     ::= "while" ":" INDENT statement+ DEDENT 
                  ":do" INDENT statement+ DEDENT

loop_stmt      ::= "loop" NUMBER ":" INDENT statement+ DEDENT

match_stmt     ::= "match" ":" INDENT 
                  ("case" NUMBER ":" INDENT statement+ DEDENT)+ 
                  ["default" ":" INDENT statement+ DEDENT] DEDENT

function_def_stmt ::= "def" IDENTIFIER "(" [IDENTIFIER ("," IDENTIFIER)*] ")" ":" 
                  INDENT statement+ DEDENT

COMMENT        ::= "#" ANY_CHAR*
IDENTIFIER     ::= (LETTER | "_") (LETTER | DIGIT | "_")*
NUMBER         ::= ["-"] DIGIT+ ["." DIGIT+]
STRING         ::= "\"" ANY_CHAR* "\""
INDENT         ::= increase in indentation level
DEDENT         ::= decrease in indentation level
```

## Semantic Rules

### Stack Operation

CCL is a stack-based language, where most operations manipulate values on a shared data stack:

- `push` adds a value to the top of the stack
- Most operations consume values from the stack and push results back
- The stack is preserved across block boundaries within the same function
- Function calls create a new stack frame

### Memory Model

CCL has a hierarchical memory model:

- **Global Memory**: Accessible throughout the program
- **Function Memory**: Private to each function call, initialized with parameters
- **Block Memory**: Not isolated; part of the enclosing function memory

Variables are created using the `store` operation and accessed using the `load` operation.

### Type System

CCL currently uses floating-point numbers (`f64`) as its primary data type, with strings handled specially:

- Numbers are represented as 64-bit floating-point values
- Strings are used for identities, references, and output
- In conditional contexts, `0.0` is considered truthy (success), while any other value is falsey (failure)

### Control Flow

Control flow in CCL is based on the top value of the stack:

- `if` statements execute their body if the top of the stack is `0.0` (truthy)
- `while` loops continue as long as their condition pushes `0.0`
- `loop` statements execute a fixed number of times

### Function Calls

Functions in CCL have their own memory scope:

1. Parameters are passed by value
2. A new memory scope is created for each function call
3. The `return` statement explicitly returns from a function
4. If no explicit `return` is provided, the function implicitly returns the top of the stack

### Governance Operations

CCL includes specialized operations for cooperative governance:

- `liquiddelegate` establishes a delegation relationship between members
- `rankedvote` conducts an instant-runoff vote with ranked ballots
- `votethreshold` checks if a proposal has sufficient support
- `quorumthreshold` verifies adequate participation in a vote

## Execution Model

CCL programs are executed by the ICN-COVM in the following stages:

1. **Parsing**: The CCL text is parsed into a sequence of operations
2. **Compilation** (optional): Operations are compiled to bytecode
3. **Execution**: The VM executes operations sequentially, maintaining the stack and memory

Error handling follows these principles:

- Syntax errors are detected during parsing
- Type errors and stack underflows are detected during execution
- Division by zero and other runtime errors cause program termination

## Examples

### Simple Arithmetic

```
push 3.0
push 4.0
add     # Stack now contains 7.0
push 2.0
mul     # Stack now contains 14.0
emit "Result:"
emit "14.0"
```

### Variable Storage

```
push 10.0
store counter
load counter
push 1.0
add
store counter   # counter is now 11.0
```

### Conditional Logic

```
push 5.0
push 10.0
gt              # Is 10 > 5? (Push 1.0 for true)
if:
    emit "Ten is greater than five"
else:
    emit "This won't be printed"
```

### Loop with Break

```
push 0.0
store i
loop 10:
    load i
    push 1.0
    add
    store i
    
    load i
    push 5.0
    eq
    if:
        break   # Exit the loop when i equals 5
    
    emit "Iteration"
```

### Function Definition and Call

```
def add_two(x, y):
    load x
    load y
    add
    return

push 3.0
push 4.0
call add_two  # Stack now contains 7.0
```

### Liquid Democracy

```
liquiddelegate "alice" "bob"   # Alice delegates to Bob
liquiddelegate "carol" "alice" # Carol delegates to Alice (and thus to Bob)
```

### Ranked-Choice Voting

```
# Push ballot data onto the stack
# Each ballot has 3 rankings (last choice first)
push 2.0  # Third choice: candidate 2
push 0.0  # Second choice: candidate 0
push 1.0  # First choice: candidate 1

push 0.0  # Third choice: candidate 0
push 2.0  # Second choice: candidate 2
push 1.0  # First choice: candidate 1

push 1.0  # Third choice: candidate 1
push 0.0  # Second choice: candidate 0
push 2.0  # First choice: candidate 2

# Run ranked-choice vote with 3 candidates and 3 ballots
rankedvote 3 3
store winner
```

### Threshold Checks

```
push 100.0  # Total possible votes
push 65.0   # Votes cast
quorumthreshold 0.5  # Check if participation >= 50%

if:
    emit "Quorum reached"
    
    push 42.0   # Votes in favor
    push 65.0   # Total votes cast
    votethreshold 0.5  # Check if support >= 50%
    
    if:
        emit "Proposal passes"
    else:
        emit "Proposal fails: insufficient support"
else:
    emit "Proposal invalid: quorum not reached"
```

## Conclusion

The Cooperative Contract Language (CCL) provides a powerful yet accessible means to express governance logic for cooperatives. Its stack-based execution model, block-structured syntax, and specialized governance primitives make it well-suited for implementing democratic decision-making processes in a transparent and verifiable manner.

This grammar specification serves as the authoritative reference for CCL's syntax and semantics, guiding both human authors and machine implementations. 