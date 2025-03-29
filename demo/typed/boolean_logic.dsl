# Boolean logic demo
# Demonstrates boolean operations with the typed value system
# Requires the typed-values feature flag to be enabled

# Define boolean values
push true
store a
push false 
store b

# Boolean AND
load a
load a
and
if:
    emit "true AND true = true"

load a
load b
and
if:
    emit "true AND false = true (unexpected!)"
else:
    emit "true AND false = false"

# Boolean OR
load a
load b
or
if:
    emit "true OR false = true"

load b
load b
or
if:
    emit "false OR false = true (unexpected!)"
else:
    emit "false OR false = false"

# Boolean NOT
load a
not
if:
    emit "NOT true = true (unexpected!)"
else:
    emit "NOT true = false"

load b
not
if:
    emit "NOT false = true"

# Boolean in expressions
push 10
push 5
gt
push 3
push 7
lt
and
if:
    emit "10 > 5 AND 3 < 7" 