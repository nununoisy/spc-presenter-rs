Data Area
=========

Commands
--------

### `:[label]` - Label
Specifies a location that can be referenced in the script area with `l[label]`.
- `[label]` - A unique number between 0 and 1023 (inclusive).

### `#i "[file]"`/`#it "[file]"` - Include Text
Include the contents of `[file]` at this location.
`[file]` is evaluated relative to the SPC file's parent directory.

> [!WARNING]  
> Includes are only evaluated as a single pass to prevent infinite recursion.
> If you include a file that contains another include, the nested include will
> be ignored.

### `#ib "[file]"` - Include Binary
Include the contents of `[file]` at this location as raw binary data.
`[file]` is evaluated relative to the SPC file's parent directory.

Data
----

Any text in the data section that is not a command will be parsed as hexadecimal
data. Whitespace is ignored, so you may segment your data however you'd like.

```
; All of these are equivalent:
6F 91 47 EB 10 7A 7F CD 98 13 E7 CA 9E 7F 1A 4E
6F91 47EB 107A 7FCD 9813 E7CA 9E7F 1A4E
6F9147EB 107A7FCD 9813E7CA 9E7F1A4E
6F9147EB107A7FCD9813E7CA9E7F1A4E
6 F 9 1 4 7 EB 1 0 7 A 7 FCD 9 8 1 3 E 7 CA 9 E 7 F 1 A 4 E
```
