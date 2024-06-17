Script Area
===========

Commands
--------

### `:[label]` - Label
Specifies a location that can be branched to.
- `[label]` - A unique number between 0 and 1023 (inclusive).

### `w [cycles]` - Wait
Pauses execution of the script for a short time while the SPC700 continues playing.
The duration of the pause is specified in SPC700 clock cycles: there are 2,048,000 cycles in a second.
- `[cycles]` - Duration parameter (class: numeric)

> [!CAUTION]  
> A runtime error occurs if `[cycles]` evaluates to 0.

### `m [pSRC] [pDST]` - Move
Assign the value at `[pSRC]` to `[pDST]`.
- `[pSRC]` - Source parameter (class: source)
- `[pDST]` - Destination parameter (class: destination)

### `c [CMP1] [CMP2]` - Compare
Store values into the comparison registers `[CMP1]` and `[CMP2]` for use in the conditional branch commands and by the dynamic parameters.
- `[CMP1]` - First parameter (class: source)
- `[CMP2]` - Second parameter (class: source)

### `a [pVAL] [pDST]` - Add
Compute `[pDST] + [pVAL]` and store the result in `[pDST]`.
- `[pVAL]` - Value parameter (class: source)
- `[pDST]` - Destination parameter (class: destination)

### `s [pVAL] [pDST]` - Subtract
Compute `[pDST] - [pVAL]` and store the result in `[pDST]`.
- `[pVAL]` - Value parameter (class: source)
- `[pDST]` - Destination parameter (class: destination)

### `u [pVAL] [pDST]` - Multiply
Compute `[pDST] * [pVAL]` and store the result in `[pDST]`.
- `[pVAL]` - Value parameter (class: source)
- `[pDST]` - Destination parameter (class: destination)

### `d [pVAL] [pDST]` - Divide
Compute `[pDST] / [pVAL]` and store the result in `[pDST]`.
- `[pVAL]` - Value parameter (class: source)
- `[pDST]` - Destination parameter (class: destination)

> [!CAUTION]  
> A runtime error occurs if `[pVAL]` evaluates to 0.

### `n [pVAL] [op] [pDST]` - Numeric Computation
Perform a computation with `[pVAL]` and `[pDST]` and store it into `[pDST]`, based on the value of `[op]`.
- `[pVAL]` - Value parameter (class: source)
- `[pDST]` - Destination parameter (class: destination)
- `[op]` - Operation:
  - `+`: Addition
  - `-`: Subtraction
  - `*`: Multiplication
  - `/`: Signed division by `[pVAL]`
  - `\ `: Unsigned division by `[pVAL]`
  - `%`: Remainder by `[pVAL]`
  - `$`: Modulus (Euclidean remainder) by `[pVAL]`
  - `&`: AND
  - `|`: OR
  - `^`: XOR
  - `<`: Left shift by `[pVAL]`
  - `_`: Arithmetic right shift by `[pVAL]`
  - `>`: Logical right shift by `[pVAL]`
  - `!`: NOT of `[pVAL]`

> [!CAUTION]  
> A runtime error occurs if `[pVAL]` evaluates to 0 when `[op]` is one of `/` or `\ `.

### `b[xx] [tgt]` - Branch
Branch to `[tgt]` if the condition is satisfied.
- `[tgt]` - Target parameter, one of:
  - Numeric literal (`[label]`/`#[label]`) - branches to label `[label] % 1024`.
  - Label parameter (`l[label]`) - branches to label `[label]`.
  - Working memory (`w[work]`) - branches to label `w[work] % 1024`.
- `b[xx]` - Conditions:
  - `bra` - Branch unconditionally
  - `beq` - Branch if `[CMP2]` is equal to `[CMP1]`
  - `bne` - Branch if `[CMP2]` is not equal to `[CMP1]`
  - `bge` - Branch if `[CMP2]` is greater than or equal to `[CMP1]`
  - `ble` - Branch if `[CMP2]` is less than or equal to `[CMP1]`
  - `bgt` - Branch if `[CMP2]` is greater than `[CMP1]`
  - `blt` - Branch if `[CMP2]` is less than `[CMP1]`
  - `bcc` - Branch if `[CMP2]` is greater than or equal to `[CMP1]` (unsigned)
  - `blo` - Branch if `[CMP2]` is less than or equal to `[CMP1]` (unsigned)
  - `bhi` - Branch if `[CMP2]` is greater than `[CMP1]` (unsigned)
  - `bcs` - Branch if `[CMP2]` is less than `[CMP1]` (unsigned)

> [!CAUTION]  
> A runtime error occurs if `[tgt]` references a non-existent label or points to the data area.

### `r` - Return
Jumps to the instruction after the last executed branch. The call stack can contain up to 64 entries.

If call stack storage is disabled, it is re-enabled before the jump occurs.

> [!CAUTION]  
> A runtime error occurs if the call stack is empty.

### `r0` - Disable Call Stack Storage
Disables storing return addresses in the branch call stack. This is useful for processing loops in subroutines, to prevent overwriting the return address.

### `r1` - Enable Call Stack Storage
Enables storing return addresses in the branch call stack.

### `f` - Flush Input Port Writes
Writes cached input port values to the SPC700.
If input port writes are disabled, they are first re-enabled.
Writes are performed in this order: `i1`, `i2`, `i3`, `i0`.
After writing the values, this command waits until `o0` is equal to `i0`.
This command is useful for quick data transfer to a real SPC700.

### `f0` - Disable Input Port Writes

Disables immediate input port writes to the SPC700.

### `f1` - Enable Input Port Writes

Enables immediate input port writes to the SPC700.

### `wi [port]` - Wait For Input Port Read

Pauses the script until the SPC700 reads the input port `[port]`, then sets `[CMP1]` to the number of SPC700 clock cycles that elapsed while waiting.

### `wo [port]` - Wait For Output Port Write

Pauses the script until the SPC700 writes to the output port `[port]`, then sets `[CMP1]` to the number of SPC700 clock cycles that elapsed while waiting.

### `q` - Quit

Immediately stops execution of the script. Playback continues.

### `nop` - No-Operation

Does nothing.

> [!CAUTION]  
> A runtime error always occurs. This allows you to stub out part of a line by preceding it with a `nop`.

### `#i "[file]"`/`#it "[file]"` - Include
Include the contents of `[file]` at this location.
`[file]` is evaluated relative to the SPC file's parent directory.

> [!WARNING]  
> Includes are only evaluated as a single pass to prevent infinite recursion.
> If you include a file that contains another include, the nested include will
> be ignored.

### `e` - Exit Script Area
Exits the script area. The section after this becomes the extended area.

> [!NOTE]  
> It is recommended to use `::` after `e` to maintain compatibility with Script700 SE:
> ```
> e
> ::
> ```

Parameters
----------

### `#[num]` - Numeric Literal
32-bit numeric literal.

### `i[port]` - SPC700 Input Port
One of the four SPC700 input port values. 8-bit writes only.

If `[port]` exceeds 4, then the port number will be `[port] % 4`.

Values written by scripts to `i[port]` are accessible from the SPC700 by reading ARAM `$00F4 + [port]`.

### `o[port]` - SPC700 Output Port
One of the four SPC700 output port values. 8-bit reads only.

If `[port]` exceeds 4, then the port number will be `[port] % 4`.

Values written by the SPC700 at ARAM `$00F4 + [port]` are accessible by scripts as `o[port]`.

### `w[work]` - Script700 Working Memory
8 32-bit values, start at 0.

### `r[width][addr]` - SPC700 ARAM

- `b` - 8-bit (default)
- `w` - 16-bit
- `d` - 32-bit

### `x[addr]` - SPC700 IPL ROM

8-bit reads only.

### `d[width][offset]` - Script700 Data Area

- `b` - 8-bit (default)
- `w` - 16-bit
- `d` - 32-bit

### `l[label]` - Data Area Label

Pointer to label specified with `:[label]`.

Dynamic Parameters
------------------

If you use a `?` in place of a number, it is replaced with:
- the value of `[CMP1]` in the first parameter, or
- the value of `[CMP2]` in the second parameter.

Example:
```
; Typical method:
m w0 w1  ; work[0] -> work[1]

; With dynamic parameters:
c #0 #1  ; 0 -> CMP1, 1 -> CMP2
m w? w?  ; work[CMP1] -> work[CMP2]

; Dereferencing: (work[CMP1] -> work[work[CMP2]])
c #0 #1  ; 0 -> CMP1, 1 -> CMP2
m #? w?  ; CMP1 -> CMP1, work[CMP2] -> CMP2
m w? w?  ; work[CMP1] -> work[CMP2]
```

Supported forms:
- `#?`
- `i?`
- `o?`
- `w?`
- `r?`
- `rb?`
- `rw?`
- `rd?`
- `x?`
- `d?`
- `db?`
- `dw?`
- `dd?`
- `l?`

Parameter Classes
----------------

### Source Parameters
Parameters for data sources.
If the parameter type is omitted, source parameters default to being output ports (`o[port]`).

### Destination Parameters
Parameters for data destinations.
If the parameter type is omitted, destination parameters default to being input ports (`i[port]`).
Destination parameters may not be numeric literals (`#[num]`) or labels (`l[label]`).

### Numeric Parameters
Parameters for numeric data that isn't necessarily associated with memory.
If the parameter type is omitted, numeric parameters default to being numeric literals (`#[num]`).
