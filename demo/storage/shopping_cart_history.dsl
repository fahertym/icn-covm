# Shopping Cart with History
# This demo shows how to use storage versioning to maintain a shopping cart history
# Run with: cargo run -- run --program demo/storage/shopping_cart_history.dsl --storage-backend file --storage-path ./filestorage

emit "Shopping Cart with Version History Demo"
emit "--------------------------------------"

# Initialize cart with first item
emit "1. Creating cart with Coffee ($5.99)..."
push 5.99
emit "Cart Contents:"
dup
emit "  Total: $5.99"
emit "-------------------------"
storep cart_key
emit "Initial cart stored"

# Add second item
emit "2. Adding Muffin ($3.50)..."
loadp cart_key
push 3.50
add
emit "Cart Contents:"
dup
emit "  Total: $9.49"
emit "-------------------------"
storep cart_key
emit "Updated cart stored"

# Add third item
emit "3. Adding Sandwich ($8.75)..."
loadp cart_key
push 8.75
add
emit "Cart Contents:"
dup
emit "  Total: $18.24"
emit "-------------------------"
storep cart_key
emit "Updated cart stored"

# Show cart history
emit ""
emit "4. Retrieving cart history..."
emit "=== Shopping Cart History ==="

# Load current version
emit "Current cart (latest version):"
loadp cart_key
dumpstack

# Load version 1 (first version)
emit "Initial cart (version 1):"
loadversionp cart_key 1
dumpstack

# Load version 2 (after first update)
emit "After first update (version 2):"
loadversionp cart_key 2
dumpstack

# Load version 3 (after second update)
emit "After second update (version 3):"
loadversionp cart_key 3
dumpstack

emit ""
emit "Shopping cart demo completed!" 