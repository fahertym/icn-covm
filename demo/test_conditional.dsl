# Test for conditionals

# Test true condition
push 5
push 10
gt

if:
    emit "True condition: 10 > 5"
    push 42
else:
    emit "ERROR: Should not run else branch for true condition"
    push 0

emit "Value from true condition (should be 42):"
dumpstack

# Test false condition
push 10
push 5
gt

if:
    emit "ERROR: Should not run then branch for false condition" 
    push 99
else:
    emit "False condition: 5 is not > 10"
    push 42

emit "Value from false condition (should be 42):"
dumpstack
