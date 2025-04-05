push 1.0
storep "resources/community_coin"

mint community_coin "treasury" 1000.0 "Initial coin allocation for community treasury"
emitevent "economic" "Minted 1000 coins to treasury"

transfer community_coin "treasury" "alice" 250.0 "Monthly community contribution"
emitevent "economic" "Transferred 250 coins from treasury to Alice"

transfer community_coin "treasury" "bob" 150.0 "Project bounty reward"
emitevent "economic" "Transferred 150 coins from treasury to Bob"

transfer community_coin "alice" "bob" 75.0 "Payment for design work"
emitevent "economic" "Transferred 75 coins from Alice to Bob"

emit "Current balances checked"

burn community_coin "bob" 50.0 "Redeemed for community workshop"
emitevent "economic" "Bob burned 50 coins for community workshop"

emit "Final balances checked" 