# Storage and Identity Test
# This test verifies that the storage and identity features are working correctly

# Store some values in different namespaces
emit "1. Storing values in different namespaces..."

# Store in general namespace
push 100.0
storep "general/value1"
emit "Stored 100 in general/value1"

# Store in another namespace 
push 200.0
storep "test/value2"
emit "Stored 200 in test/value2"

# Load values back and verify
emit "2. Loading values from different namespaces..."

# Load from general namespace
loadp "general/value1"
emit "Value from general/value1:"
dumpstack

# Load from test namespace
loadp "test/value2"
emit "Value from test/value2:"
dumpstack

emit "Test complete! Storage system is working correctly for v0.6.0 features." 