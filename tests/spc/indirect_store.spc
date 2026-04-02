; indirect_store.spc — Store and load via address indirection
; Tests addrl + store/load combination for record-like patterns
; Simulates: var r: record a, b, c: integer end;
;            r.a := 5; r.b := 7; r.c := r.a * r.b;
;            write(r.c);  => 35
; Expected output: 35

.proc main 3
    ; local[0]=r.a, local[1]=r.b, local[2]=r.c
    ; r.a := 5 via indirect store
    push_s 5
    addrl 0
    store
    ; r.b := 7 via indirect store
    push_s 7
    addrl 1
    store
    ; r.c := r.a * r.b via indirect load
    addrl 0
    load
    addrl 1
    load
    mul
    addrl 2
    store
    ; Print r.c (35) as decimal
    addrl 2
    load
    dup
    push_s 10
    div
    push_s 48
    add
    sys 1
    push_s 10
    mod
    push_s 48
    add
    sys 1
    push_s 10
    sys 1
    halt
.end
