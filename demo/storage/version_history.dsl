# Version History Demonstrator
# This demo shows how to list and manage versions of stored values

# First, store multiple versions of a value to have something to list
emit "Creating version history..."

# Store version 1
emit "1. Storing document version 1.0"
push 1.0
storep document_key
emit "Initial document stored (v1)"

# Store version 2
emit "2. Updating document to version 2.0"
push 2.0
storep document_key
emit "Document updated (v2)"

# Store version 3 
emit "3. Updating document to version 3.0"
push 3.0
storep document_key
emit "Document updated (v3)"

# Store version 4
emit "4. Updating document to version 4.0" 
push 4.0
storep document_key
emit "Document updated (v4)"

# List all versions
emit ""
emit "5. Listing all available versions:"
listversionsP document_key
emit "Found versions count: " dumpstack

# Demonstrate accessing specific versions
emit ""
emit "6. Accessing specific versions:"

# Load version 1 (first version)
emit "Document version 1:"
loadversionp document_key 1
dumpstack

# Load version 3 (intermediate version)
emit "Document version 3:"
loadversionp document_key 3
dumpstack

# Load current version
emit "Current document version:"
loadp document_key
dumpstack

emit ""
emit "Version history demo completed successfully!" 