Examples
========

AddMusicK
---------

### Enable Yoshi Drums

```
; w0: Number of milliseconds to wait before enabling drums
; In this case, 57 seconds
m #57000 w0

; Wait loop
r0 ; Disable call stack
:0
w #2048 ; 2048 cycles / 2048 kHz = 1 ms
s #1 w0 ; Decrement loop counter
c #0 w0 ; Loop if counter is not zero
bne 0

; Enable Yoshi drums
m #2 i1 ; Write $02 to port 1

; End
q
```

Effects:
- Write `#1` to play the jump SFX
- Write `#2` to enable the Yoshi drums
- Write `#3` to disable the Yoshi drums
- Write `#7` to pause the music
- Write `#8` to unpause the music

### Grinder SFX
```
r0
:0
m #0 i1
w 65536
m #4 i1
w 196608
bra 0
```
