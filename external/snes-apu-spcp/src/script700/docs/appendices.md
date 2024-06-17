Appendices
==========

Number Format
-------------

By default, numbers are parsed as decimal (base-10).

Numbers prefixed with `$` or `0x` are parsed as hexadecimal (base-16).

For example, you can express the number 255 (decimal) in any of the following ways:
- `#255`
- `#$ff`
- `#0xFF`

Error Handling
--------------

### Invalid Tokens

If the parser encounters a command/parameter it does not recognize while parsing, the current command
and any characters on the current line following it are ignored.

This can be used to implement compatibility checks, like this:
```
s w0 w1 bra 1
bra 2

:1
; The `s` command is supported.

:2
; The `s` command was not recognized, so the `bra 1` was ignored.
```

This can also be used to make comments:
```
n #1 - w0 ; work[0]-- (Recommended)
n #1 + w0 // work[0]++
n #4 * w0 ' work[0] *= 4
n #4 / w0 # work[0] /= 4
```
> [!WARNING]  
> Using a character other than `;` is undefined behavior, as that
> character may be added as a command later.

### Runtime Errors

If an error occurs during runtime, it's treated like an invalid token:
the current command and any characters on the current line following it are ignored.

```
d #? w0 bra 1
bra 2

:1
; The division succeeded.

:2
; [CMP1] was 0, so the division failed.
```
