; array_sum.spc — Sum elements of an integer array via global storage
; a[0]:=10; a[1]:=20; a[2]:=30; a[3]:=40;
; sum := 0; for i:=0 to 3 do sum := sum + a[i];
; write(sum);  => 100
; Expected output: 100

.global arr 4
.global sum 1
.global idx 1

.proc main 0
    ; a[0] := 10
    push_s 10
    storeg arr
    ; a[1] := 20
    push_s 20
    addrg arr
    push_s 3
    add
    store
    ; a[2] := 30
    push_s 30
    addrg arr
    push_s 6
    add
    store
    ; a[3] := 40
    push_s 40
    addrg arr
    push_s 9
    add
    store
    ; sum := 0
    push_s 0
    storeg sum
    ; idx := 0
    push_s 0
    storeg idx
sloop:
    ; if idx > 3 then exit
    loadg idx
    push_s 3
    gt
    jnz sdone
    ; sum := sum + a[idx]
    addrg arr
    loadg idx
    push_s 3
    mul
    add
    load
    loadg sum
    add
    storeg sum
    ; idx++
    loadg idx
    push_s 1
    add
    storeg idx
    jmp sloop
sdone:
    ; Print sum (100) as decimal: '1' '0' '0'
    loadg sum
    ; hundreds digit
    dup
    push_s 100
    div
    push_s 48
    add
    sys 1
    ; tens digit
    dup
    push_s 100
    mod
    push_s 10
    div
    push_s 48
    add
    sys 1
    ; ones digit
    push_s 100
    mod
    push_s 10
    mod
    push_s 48
    add
    sys 1
    push_s 10
    sys 1
    halt
.end
