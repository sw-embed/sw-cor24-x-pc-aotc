; memcpy_test.spc — Block copy test using globals
; Copies "ABC" from src to dst, prints both to verify
; Expected output: ABCABC

; 2 words = 6 bytes each for src and dst
.global src 2
.global dst 2

.proc main 0
    ; Initialize src with 'A','B','C'
    push_s 65
    addrg src
    storeb
    push_s 66
    addrg src
    push_s 1
    add
    storeb
    push_s 67
    addrg src
    push_s 2
    add
    storeb
    ; Copy 3 bytes from src to dst
    addrg src
    addrg dst
    push_s 3
    memcpy
    ; Print dst[0..2]
    addrg dst
    loadb
    sys 1
    addrg dst
    push_s 1
    add
    loadb
    sys 1
    addrg dst
    push_s 2
    add
    loadb
    sys 1
    ; Print src[0..2] (verify not corrupted)
    addrg src
    loadb
    sys 1
    addrg src
    push_s 1
    add
    loadb
    sys 1
    addrg src
    push_s 2
    add
    loadb
    sys 1
    push_s 10
    sys 1
    halt
.end
