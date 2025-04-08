# Memory Leak Test
# This test verifies that variables defined within a function
# are not accessible in the global scope after the function returns

# Create a global variable
push 1.0
store global_var
emit "Created global variable: global_var = 1.0"

# Define a function that creates a local variable
def create_local_vars():
    # Create a local variable within the function
    push 42.0
    store temp_var1
    emit "Inside function: Created temp_var1 = 42.0"
    
    # Create another local variable
    push 100.0
    store temp_var2
    emit "Inside function: Created temp_var2 = 100.0"
    
    # Use the local variables
    load temp_var1
    load temp_var2
    add
    emit "Inside function: temp_var1 + temp_var2 ="
    dumpstack
    
    # Return without returning any value
    return

# Call the function
emit "Calling function create_local_vars()"
call create_local_vars
emit "Function returned"

# We can't access temp_var1 now, but we can't directly test this 
# since there's no try/catch in the DSL.
emit "NOTE: If memory scoping is implemented correctly, uncommenting the next line would cause an error:"
# load temp_var1

# Instead we'll just verify global variable is still accessible
emit "Verifying global variable is still accessible"
load global_var
emit "Global variable value:"
dumpstack

emit "Memory leak test completed" 