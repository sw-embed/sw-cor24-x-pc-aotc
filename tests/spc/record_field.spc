; record_field.spc — Record field access via address + offset
; Simulates: type Point = record x, y: integer end;
;            var p: Point;
;            p.x := 10; p.y := 20;
;            write(p.x + p.y);  => 30
; Expected output: 30

.proc main 2
    ; local[0] = p.x, local[1] = p.y
    ; Store p.x := 10
    push_s 10
    storel 0
    ; Store p.y := 20
    push_s 20
    storel 1
    ; Load p.x and p.y, add them
    loadl 0
    loadl 1
    add
    ; Print result as decimal
    ; 30 = '3' '0'
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
