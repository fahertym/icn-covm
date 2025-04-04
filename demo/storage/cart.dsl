# Simple Shopping Cart Demo
# This demonstrates using persistent storage to track a shopping cart total
# Run with: cargo run -- run --program demo/storage/cart.dsl --storage-backend file --storage-path ./filestorage

# Attempt to load the cart item count
emit "Checking cart status:"
loadp cart_count
dumpstack

# Check if the cart exists (might not exist on first run)
push 0.0
eq
if:
    # First run: initialize cart count to 0
    emit "Cart doesn't exist yet, initializing empty cart"
    push 0.0
    storep cart_count
    push 0.0
    storep cart_total
    # Output message
    emit "Initialized empty cart"
else:
    # Cart already exists, show current state
    emit "Cart exists with items:"
    loadp cart_count
    dumpstack
    emit "Current cart total:"
    loadp cart_total
    dumpstack

# Add an item to the cart
emit "Adding Programming Book to cart..."
# Increment cart count
loadp cart_count
push 1.0
add
storep cart_count

# Add item price to total
loadp cart_total
push 29.99
add
storep cart_total

# Show updated cart
emit "Updated cart status:"
emit "Item count:"
loadp cart_count
dumpstack
emit "Cart total:"
loadp cart_total
dumpstack

# Final message
emit "Shopping cart updated successfully."
emit "Run this program again to add more items and see persistence!" 