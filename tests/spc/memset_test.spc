; memset_test.spc — Memory fill test using globals
; Fills buffer with 'X' (88) then prints
; Expected output: XXXX

; 2 words = 6 bytes
.global buf 2

.proc main 0
    ; Fill 4 bytes with 'X' (88)
    addrg buf
    push_s 88
    push_s 4
    memset
    ; Print buf[0..3]
    addrg buf
    loadb
    sys 1
    addrg buf
    push_s 1
    add
    loadb
    sys 1
    addrg buf
    push_s 2
    add
    loadb
    sys 1
    addrg buf
    push_s 3
    add
    loadb
    sys 1
    push_s 10
    sys 1
    halt
.end
