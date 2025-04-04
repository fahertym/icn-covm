# Version Difference Demonstrator
# This demo shows how to compare different versions of stored values

emit "Starting version difference demo..."

# Create a sequence of versions with increasingly larger values
emit "1. Creating a sequence of versions with different values"

# Store initial value
emit "   Storing initial value (10.0)"
push 10.0
storep data_key
emit "   Initial value stored (v1)"

# Store second version
emit "   Updating to version 2 (15.0)"
push 15.0
storep data_key
emit "   Updated to v2"

# Store third version 
emit "   Updating to version 3 (25.0)"
push 25.0
storep data_key
emit "   Updated to v3"

# Store fourth version with a bigger jump
emit "   Updating to version 4 (50.0)" 
push 50.0
storep data_key
emit "   Updated to v4"

# List all versions to confirm
emit ""
emit "2. Listing all available versions:"
listversionsP data_key
emit "Found versions count: " dumpstack

# Compare different versions
emit ""
emit "3. Comparing different versions:"

# Compare version 1 vs version 2 (difference: 5.0)
emit "   Difference between v1 and v2:"
diffversionsp data_key 1 2
emit "   Absolute difference: " dumpstack

# Compare version 1 vs version 4 (difference: 40.0)
emit "   Difference between v1 and v4:"
diffversionsp data_key 1 4
emit "   Absolute difference: " dumpstack

# Compare version 2 vs version 3 (difference: 10.0)
emit "   Difference between v2 and v3:"
diffversionsp data_key 2 3
emit "   Absolute difference: " dumpstack

# Compare consecutive versions and sum changes
emit ""
emit "4. Calculating total change across all versions:"
emit "   Adding up all consecutive version differences:"

# Get v1 to v2 difference
diffversionsp data_key 1 2
# Get v2 to v3 difference 
diffversionsp data_key 2 3
# Add them
add
# Get v3 to v4 difference
diffversionsp data_key 3 4
# Add to running sum
add
emit "   Total accumulated change: " dumpstack

# Compare with direct v1 to v4 difference
emit "   Direct v1 to v4 difference:"
diffversionsp data_key 1 4
emit "   Direct difference: " dumpstack

emit ""
emit "Version difference demo completed successfully!" 