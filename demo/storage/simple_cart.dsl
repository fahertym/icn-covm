# Simple Shopping Cart Demo
# This demonstrates using persistent storage to track items in a cart
# Run with: cargo run -- run --program demo/storage/simple_cart.dsl --storage-backend file --storage-path ./filestorage
# Item 1: Programming Book (code 1001) - $29.99
# Item 2: Coffee Mug (code 1002) - $12.50

# First, check if we have a cart count already
emit "Initializing cart..."
loadp cart_count
push 0.0
eq
if:
    # First time - no cart exists yet
    emit "Creating new cart"
    push 0.0
    storep cart_count
    push 0.0
else:
    emit "Cart exists with items:"
    loadp cart_count
    dumpstack
end

# Add first item (if this is the first run)
loadp cart_count
push 0.0
eq
if:
    # Add book item
    emit "Adding first item: Programming Book (code 1001)"
    
    # Store item code at index 0
    push 1001.0
    storep cart_item_0_code
    
    # Store quantity at index 0
    push 1.0
    storep cart_item_0_qty
    
    # Store price at index 0
    push 29.99
    storep cart_item_0_price
    
    # Increment cart count
    push 1.0
    storep cart_count
    
    emit "First item added"
else:
    emit "Items already in cart"
end

# Add second item (check if we already have exactly 1 item)
loadp cart_count
push 1.0
eq
if:
    # Add coffee mug item
    emit "Adding second item: Coffee Mug (code 1002)"
    
    # Store item code at index 1
    push 1002.0
    storep cart_item_1_code
    
    # Store quantity at index 1
    push 2.0
    storep cart_item_1_qty
    
    # Store price at index 1
    push 12.50
    storep cart_item_1_price
    
    # Increment cart count
    loadp cart_count
    push 1.0
    add
    storep cart_count
    
    emit "Second item added"
else:
    emit "Cart already has multiple items or is empty"
end

# Display cart contents
emit "============================="
emit "SHOPPING CART CONTENTS:"
emit "============================="

loadp cart_count
push 0.0
eq
if:
    emit "Cart is empty"
else:
    # Check for item 0
    emit "Item 1:"
    loadp cart_item_0_code
    emit "Code:"
    dumpstack
    emit "Name: Programming Book"
    loadp cart_item_0_qty
    emit "Quantity:"
    dumpstack
    loadp cart_item_0_price
    emit "Price:"
    dumpstack
    emit "-----------------------------"
    
    # Check for item 1
    loadp cart_count
    push 1.0
    gt
    if:
        emit "Item 2:"
        loadp cart_item_1_code
        emit "Code:"
        dumpstack
        emit "Name: Coffee Mug"
        loadp cart_item_1_qty
        emit "Quantity:"
        dumpstack
        loadp cart_item_1_price
        emit "Price:"
        dumpstack
        emit "-----------------------------"
    else:
        emit "No more items"
    end
end

# Calculate cart total
emit "Calculating cart total..."
push 0.0  # Start with 0

# Add item 0 price * quantity if it exists
loadp cart_count
push 0.0
gt
if:
    loadp cart_item_0_price
    loadp cart_item_0_qty
    mul
    add
end

# Add item 1 price * quantity if it exists
loadp cart_count
push 1.0
gt
if:
    loadp cart_item_1_price
    loadp cart_item_1_qty
    mul
    add
end

emit "Cart total:"
dumpstack

emit "Shopping cart data has been saved!"
emit "Run the program again to see your saved cart!" 