# Quorum Threshold Demo
emit "QUORUM THRESHOLD DEMO"
emit "===================="
emit ""
emit "This demo demonstrates the quorum threshold operation"
emit "which ensures adequate participation in governance decisions."
emit ""

# Set up initial voting power for all members
push 1.0
store "alice_power"
push 1.0
store "bob_power"
push 1.0
store "carol_power"
push 1.0
store "dave_power"
push 1.0
store "eve_power"

emit "Initial voting power:"
emit "Alice: 1.0, Bob: 1.0, Carol: 1.0, Dave: 1.0, Eve: 1.0"
emit ""
emit "Total possible votes: 5.0"
emit ""

# Scenario 1: Good participation
emit "SCENARIO 1: GOOD PARTICIPATION"
emit "============================="
emit "In this scenario, 4 out of 5 members vote."
emit "This represents 80% participation."
emit ""

# Check if enough members participated (quorum)
# Total possible votes = 5 members x 1 vote each = 5 votes
# Total votes cast = 4 votes
push 5.0  # Total possible votes
push 4.0  # Total votes cast
quorumthreshold 0.6  # 60% minimum participation

if:
    emit "✓ Quorum met! 4 out of 5 possible votes cast (80% participation)."
    emit "  The vote results are valid and can be processed."
else:
    emit "✗ Quorum not met! Vote is invalid due to insufficient participation."

emit ""

# Scenario 2: Poor participation
emit "SCENARIO 2: POOR PARTICIPATION"
emit "=============================="
emit "In this scenario, only 2 out of 5 members vote."
emit "This represents 40% participation."
emit ""

# Check if enough members participated (quorum)
# Total possible votes = 5 members x 1 vote each = 5 votes
# Total votes cast = 2 votes
push 5.0  # Total possible votes
push 2.0  # Total votes cast
quorumthreshold 0.6  # 60% minimum participation

if:
    emit "✓ Quorum met! 2 out of 5 possible votes cast (40% participation)."
    emit "  The vote results are valid and can be processed."
else:
    emit "✗ Quorum not met! Vote is invalid due to insufficient participation."

emit ""

# Scenario 3: Exact threshold
emit "SCENARIO 3: EXACT THRESHOLD"
emit "=========================="
emit "In this scenario, exactly 3 out of 5 members vote."
emit "This represents 60% participation, equal to the threshold."
emit ""

# Check if enough members participated (quorum)
# Total possible votes = 5 members x 1 vote each = 5 votes
# Total votes cast = 3 votes
push 5.0  # Total possible votes
push 3.0  # Total votes cast
quorumthreshold 0.6  # 60% minimum participation

if:
    emit "✓ Quorum met! 3 out of 5 possible votes cast (60% participation)."
    emit "  The vote results are valid and can be processed."
else:
    emit "✗ Quorum not met! Vote is invalid due to insufficient participation."

emit ""
emit "DEMO COMPLETE"
emit ""
emit "The quorum threshold operation ensures that decisions"
emit "have sufficient participation to be considered legitimate." 