; array_access.spc — Array indexing via global storage and indirect memory
; Simulates: var arr: array[0..4] of integer;
;            arr[0]:=72; arr[1]:=101; arr[2]:=108; arr[3]:=108; arr[4]:=111;
;            for i:=0 to 4 do write(chr(arr[i]));
;            writeln;
; Expected output: Hello

; 5 words for array, 1 word for loop index
.global arr 5
.global idx 1

.proc main 0
    ; arr[0] := 72 ('H')
    push_s 72
    storeg arr
    ; arr[1] := 101 ('e')  (arr + 1 word offset)
    push_s 101
    addrg arr
    push_s 3
    add
    store
    ; arr[2] := 108 ('l')
    push_s 108
    addrg arr
    push_s 6
    add
    store
    ; arr[3] := 108 ('l')
    push_s 108
    addrg arr
    push_s 9
    add
    store
    ; arr[4] := 111 ('o')
    push_s 111
    addrg arr
    push_s 12
    add
    store
    ; idx := 0
    push_s 0
    storeg idx
loop:
    ; if idx > 4 then exit
    loadg idx
    push_s 4
    gt
    jnz done
    ; compute addr of arr[idx]: base + idx*3
    addrg arr
    loadg idx
    push_s 3
    mul
    add
    ; load arr[idx]
    load
    ; print as char
    sys 1
    ; idx := idx + 1
    loadg idx
    push_s 1
    add
    storeg idx
    jmp loop
done:
    ; newline
    push_s 10
    sys 1
    halt
.end
