# Simple test for if/else conditions in the VM
# In this VM, 0.0 is considered TRUE for if statements
# Any non-zero value is considered FALSE

# Test 1: Direct if with 0.0 (TRUE condition)
push 0.0  # Push TRUE condition directly
emit "Test 1: Pushing 0.0 (TRUE) directly"
if:
    emit "TRUE BRANCH: If condition with 0.0 executed correctly"
    push 42
else:
    emit "ERROR: Else branch executed with 0.0 condition"
    push 99

emit "Result from Test 1 (should be 42):"
dumpstack

# Test 2: Direct if with 1.0 (FALSE condition)
push 1.0  # Push FALSE condition directly
emit "Test 2: Pushing 1.0 (FALSE) directly"
if:
    emit "ERROR: Then branch executed with 1.0 condition"
    push 77
else:
    emit "FALSE BRANCH: Else condition with 1.0 executed correctly"
    push 55

emit "Result from Test 2 (should be 55):"
dumpstack

# Test 3: Using GT operator - Stack ordering is important
# For TRUE condition: When we do "push 5, push 10, gt" 
# the VM compares 10 > 5 which is true (pushes 0.0)
push 5    # Second value (popped second)
push 10   # First value (popped first)
gt        # Compares 10 > 5, which is true, pushes 0.0
emit "Test 3: Using GT operator (10 > 5)"
dumpstack
if:
    emit "TRUE BRANCH: GT true condition (10 > 5) executed correctly"
    push 33
else:
    emit "ERROR: Else branch executed with true GT condition"
    push 88

emit "Result from Test 3 (should be 33):"
dumpstack

# Test 4: Using GT operator for FALSE condition
# When we do "push 10, push 5, gt" the VM compares 5 > 10
# which is false (pushes 1.0)
push 10   # Second value (popped second)
push 5    # First value (popped first)
gt        # Compares 5 > 10, which is false, pushes 1.0
emit "Test 4: Using GT operator (5 > 10)"
dumpstack
if:
    emit "ERROR: Then branch executed with false GT condition"
    push 66
else:
    emit "FALSE BRANCH: GT false condition (5 > 10) executed correctly"
    push 44

emit "Result from Test 4 (should be 44):"
dumpstack 