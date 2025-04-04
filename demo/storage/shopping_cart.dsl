# Shopping Cart Demo
# This demonstrates using persistent storage to implement a simple shopping cart
# Run with: cargo run -- run --program demo/storage/shopping_cart.dsl --storage-backend file --storage-path ./filestorage

# Helper function to display the current cart contents
def display_cart():
    emit "========================="
    emit "SHOPPING CART CONTENTS:"
    emit "========================="
    
    # Try to load the cart item count
    loadp cart_items
    
    # Check if cart exists
    push 0.0
    eq
    if:
        emit "Cart is empty"
    else:
        # Cart exists, display items
        push 0.0
        # Loop through each item
        while:
            # Stack has: counter
            # Check if we're done
            dup
            loadp cart_items
            lt
            if:
                # Get the current counter
                dup
                
                # Load the item name for this index
                dup
                concat "cart_item_name_"
                loadp
                emit "Item:"
                dumpstack
                
                # Load the quantity for this index
                swap
                dup
                concat "cart_item_qty_"
                loadp
                emit "Quantity:"
                dumpstack
                
                # Load the price for this index
                swap
                dup
                concat "cart_item_price_"
                loadp
                emit "Price:"
                dumpstack
                
                emit "-------------------------"
                
                # Increment the counter
                push 1.0
                add
                
                # Continue loop
                push 1.0
            else:
                # We're done
                pop
                push 0.0
            
            # End condition is on stack
        end
    end
    return

# Helper function to add an item to the cart
# Usage: push "item_name" push quantity push price call add_to_cart
def add_to_cart(name, quantity, price):
    # First, try to load the cart item count
    loadp cart_items
    push 0.0
    eq
    if:
        # No cart yet, initialize to 0
        push 0.0
        storep cart_items
        push 0.0
    else:
        # Cart exists, keep existing value
        loadp cart_items
    end
    
    # We now have the cart item count on the stack
    # Store item details using the count as index
    
    # Store item name
    dup
    concat "cart_item_name_"
    load name
    storep
    
    # Store quantity
    dup
    concat "cart_item_qty_"
    load quantity
    storep
    
    # Store price
    dup
    concat "cart_item_price_"
    load price
    storep
    
    # Increment cart item count
    push 1.0
    add
    
    # Store updated count
    storep cart_items
    
    emit "Item added to cart"
    return

# Main program
emit "Welcome to the Shopping Cart Demo"
emit "This demonstrates persistent storage with a shopping cart application"

# Display initial cart
call display_cart

# Menu loop
push 1.0
while:
    # Stack has: choice
    dup
    push 0.0
    eq
    if:
        # Exit loop
        pop
        push 0.0
    else:
        emit "\nSHOPPING CART MENU"
        emit "1. Add item to cart"
        emit "2. Display cart"
        emit "3. Exit"
        emit "Enter your choice (1-3):"
        
        # Simulate menu selection based on current choice
        dup
        push 1.0
        eq
        if:
            # First run: Add a book
            emit "Adding book to cart..."
            push "Programming Book"
            push 1.0
            push 29.99
            call add_to_cart
            
            # Next choice
            pop
            push 2.0
            push 1.0
        else:
            dup
            push 2.0
            eq
            if:
                # Display cart
                emit "Displaying cart..."
                call display_cart
                
                # Next choice
                pop
                push 1.0
                push 1.0
            else:
                dup
                push 3.0
                eq
                if:
                    # Exit
                    emit "Thank you for shopping!"
                    pop
                    push 0.0
                else:
                    # Another item - this is when choice is 1.0 again
                    # Add coffee mug
                    emit "Adding coffee mug to cart..."
                    push "Coffee Mug"
                    push 2.0
                    push 12.50
                    call add_to_cart
                    
                    # Next choice
                    pop
                    push 3.0
                    push 1.0
                end
            end
        end
    end
end

emit "Exiting program. Your cart has been saved."
emit "Run the program again to see your saved cart!" 