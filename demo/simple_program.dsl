push 0
store done

emitevent "program" "Starting countdown from count"

load count
emit "Starting value: "

load count
store counter

load count
push 10
lt
not
if:
    load counter
    push 1
    sub
    store counter
    load counter
    emit "Value 1: "
    
    load counter
    push 0
    gt
    if:
        load counter
        push 1
        sub
        store counter
        load counter 
        emit "Value 2: "
    
    load counter
    push 0
    gt
    if:
        load counter
        push 1
        sub
        store counter
        load counter 
        emit "Value 3: "
    
    load counter
    push 0
    gt
    if:
        load counter
        push 1
        sub
        store counter
        load counter 
        emit "Value 4: "
    
    load counter
    push 0
    gt
    if:
        load counter
        push 1
        sub
        store counter
        load counter 
        emit "Value 5: "

emitevent "program" "Countdown complete!"

load threshold
push 0
eq
not
if:
    load threshold
    emit "Threshold parameter found: "
else:
    emitevent "params" "No threshold parameter provided"

push 0 