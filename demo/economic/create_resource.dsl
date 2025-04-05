store "community_token" resource_id
store "Community Token" resource_name
store "currency" resource_type
store "coop:test_community" resource_issuer
store "A token for community governance and resource sharing" resource_description
store "COMM" resource_symbol

storep "resources/community_token" "{\"id\":\"community_token\",\"name\":\"Community Token\",\"description\":\"A token for community governance and resource sharing\",\"resource_type\":\"currency\",\"issuer_namespace\":\"coop:test_community\",\"created_at\":1618531200000,\"metadata\":{\"symbol\":\"COMM\",\"decimals\":\"2\"},\"transferable\":true,\"divisible\":true}"

emitevent "economic" "Created new economic resource: Community Token (COMM)"

mint community_token "founder1" 1000.0 "Founder allocation"
mint community_token "founder2" 1000.0 "Founder allocation"
mint community_token "community_fund" 8000.0 "Community fund initial allocation"

emit "Initial token allocations complete"

transfer community_token "community_fund" "project_team" 500.0 "Funding for Project Alpha"

emit "Project funding complete"

emitevent "economic" "Completed resource creation and initial allocation demo" 