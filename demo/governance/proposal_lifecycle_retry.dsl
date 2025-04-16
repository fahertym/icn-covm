# Proposal with Retry Logic
title: "Proposal with Retry Logic"
author: "test-author"
description: "A test proposal that demonstrates retry logic"
quorum: 0.3
threshold: 0.5

# This proposal will initially fail because it tries to read a key that doesn't exist
# On retry, the key will exist and the proposal will succeed
operations:
  - op: push
    val: "test-key"
  
  - op: store_key
    namespace: "test-namespace"
    key: "init-key"
    val: "initial-value"

  - op: read_key
    namespace: "test-namespace"
    key: "missing/key"
    
  - op: convert_to_string
  
  - op: store_key
    namespace: "test-namespace"
    key: "result-key" 