; byte_array.spc — Byte array access via loadb/storeb using globals
; Uses global word slots as a byte buffer (3 bytes per word)
; Stores 'A','B','C','D' as bytes and prints them
; Expected output: ABCD

; 2 words = 6 bytes of storage (enough for 4 characters)
.global buf 2

.proc main 1
    ; Store 'A' (65) at buf+0
    push_s 65
    addrg buf
    storeb
    ; Store 'B' (66) at buf+1
    push_s 66
    addrg buf
    push_s 1
    add
    storeb
    ; Store 'C' (67) at buf+2
    push_s 67
    addrg buf
    push_s 2
    add
    storeb
    ; Store 'D' (68) at buf+3
    push_s 68
    addrg buf
    push_s 3
    add
    storeb
    ; Loop: read and print each byte
    push_s 0
    storel 0
bloop:
    loadl 0
    push_s 4
    ge
    jnz bdone
    ; load buf[i]
    addrg buf
    loadl 0
    add
    loadb
    sys 1
    ; i++
    loadl 0
    push_s 1
    add
    storel 0
    jmp bloop
bdone:
    push_s 10
    sys 1
    halt
.end
