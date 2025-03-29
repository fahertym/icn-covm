# String operations demo
# This demonstrates the typed value system's string handling capabilities
# Requires the typed-values feature flag to be enabled

# Define a string value
push "Hello, "
store greeting

# Define another string
push "World!"
store name

# Concatenate strings
load greeting
load name
add
store full_greeting

# Use string in a conditional
load full_greeting
push "Hello, World!"
eq
if:
    emit "Greeting matches expected value"
else:
    emit "Unexpected greeting value"

# String multiplication (repetition)
push "Na"
push 4
mul
push " Batman!"
add
emit 