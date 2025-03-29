# Define some utility functions
def add(x, y):
    load x
    load y
    add
    return

def multiply_and_print(x, y):
    load x
    emit "First number:"
    load y
    emit "Second number:"
    load x
    load y
    mul
    emit "Product:"
    return

def countdown(n):
    load n
    push 0
    lt
    if:
        push 0
        return
    else:
        load n
        emit "Current value:"
        load n
        push 1
        sub
        store n
        load n
        push 0
        gt
        if:
            load n
            call countdown
        else:
            push 0
        return

# Fibonacci sequence implementation
def fib(n):
    load n
    push 1
    lt
    if:
        load n
        return
    else:
        load n
        push 1
        sub
        store n
        load n
        call fib
        load n
        push 2
        sub
        store n
        load n
        call fib
        add
        return

# Factorial implementation
def factorial(n):
    load n
    push 1
    lt
    if:
        push 1
        return
    else:
        load n
        push 1
        sub
        store n
        load n
        call factorial
        load n
        mul
        return

# GCD implementation using Euclidean algorithm
def gcd(a, b):
    load b
    push 0
    eq
    if:
        load a
        return
    else:
        load a
        load b
        mod
        load b
        store a
        store b
        call gcd
        return

# Main program
emit "Defining functions..."

emit "Testing add with 20 and 22:"
push 20
push 22
call add

emit "Testing multiply_and_print with 6 and 7:"
push 6
push 7
call multiply_and_print

emit "Starting countdown from 5:"
push 5
call countdown

emit "Calculating Fibonacci numbers..."
push 10
call fib
emit "Fibonacci(10) ="

emit "Calculating factorial..."
push 5
call factorial
emit "Factorial(5) ="

emit "Calculating GCD..."
push 48
push 18
call gcd
emit "GCD(48, 18) ="

emit "Stack manipulation demo:"
push 1
push 2
push 3
dup
swap
over
dumpstack

emit "Memory operations demo:"
push 42
store x
push 24
store y
load x
load y
add
emit "x + y ="
dumpmemory

# Governance-inspired VM demonstration
# This program shows all the new operations: Match, Break, Continue, EmitEvent, and AssertEqualStack

# Define a voting function that processes votes and returns a decision
def process_votes(proposal_id, support_votes, against_votes):
    # Store total votes for reporting
    load support_votes
    load against_votes
    add
    store total_votes
    
    # Calculate support percentage
    load support_votes
    push 100
    mul
    load total_votes
    div
    store support_percentage
    
    # Emit event with voting statistics
    emitevent "governance" "Votes processed for proposal"
    
    # Use match statement to determine outcome based on support percentage
    load support_percentage
    match:
        value:
            # We already have the percentage on the stack
        case 50:
            emitevent "governance" "Exact tie - proposal is rejected"
            push 0  # Return 0 for rejection
            return
        case 66:
            emitevent "governance" "Exact 66% support - proposal passes threshold"
            push 1  # Return 1 for approval
            return
        default:
            # Check if support >= 67% (super majority)
            load support_percentage
            push 67
            lt
            if:
                emitevent "governance" "Proposal rejected - insufficient support"
                push 0  # Return 0 for rejection
            else:
                emitevent "governance" "Proposal approved with supermajority"
                push 1  # Return 1 for approval
            return

# Simulate governance voting process
emitevent "governance" "Starting governance simulation"

# Initialize proposals and their vote counts
push 3  # Number of proposals
store proposal_count

push 0  # Current proposal index
store current_proposal

# Loop through proposals
while:
    load current_proposal
    load proposal_count
    lt
    
    # Process a proposal
    load current_proposal
    emit "Processing proposal"
    
    # Simulate vote counting for valid proposals (odd numbered)
    load current_proposal
    push 2
    mod
    push 0
    eq
    if:
        # Skip even numbered proposals (demo of 'continue')
        emitevent "governance" "Skipping even-numbered proposal"
        load current_proposal
        push 1
        add
        store current_proposal
        continue
    
    # Trigger special case for proposal 1
    load current_proposal
    push 1
    eq
    if:
        # Special handling for important proposal (demo of 'break')
        emitevent "governance" "Critical proposal detected"
        emitevent "governance" "Emergency protocol activated"
        break
    
    # Simulate vote count for this proposal
    push 135  # Support votes
    store support_votes
    
    push 65  # Against votes
    store against_votes
    
    # Calculate proposal outcome using our governance function
    load current_proposal
    load support_votes 
    load against_votes
    call process_votes
    
    # Check vote result consistency (should be 0 or 1)
    dup
    push 0
    eq
    push 1
    eq
    or
    not
    if:
        emitevent "governance" "CRITICAL ERROR: Invalid vote result"
    
    # Store the result for this proposal
    load current_proposal
    push 10
    mul
    add
    store result
    
    # Next proposal
    load current_proposal
    push 1
    add
    store current_proposal

# Demonstrate AssertEqualStack with vote results
push 1  # Approval (expected outcome)
push 1  # Approval (expected outcome)
push 1  # Approval (expected outcome)
assertequalstack 3

emitevent "governance" "Governance simulation completed"
dumpstack
dumpmemory 