Extended Area
=============

Commands
--------

### `m [SRCn]` - Mute Sample
Mute/unmute a specific sample.
- `[SRCn]`: source index in decimal, or `!` to target every sample.

### `c [SRCn] [NEWSRCn]` - Swap Samples
Replace the sample at `[SRCn]` with `[NEWSRCn]`.
- `[SRCn]`: source index in decimal, or `!` to target every sample.
- `[NEWSRCn]`: replacement source index in decimal.

### `d [SRCn] [SHIFT]` - Pitch Shift Sample
Pitch shift a sample.
- `[SRCn]`: source index in decimal, or `!` to target every sample.
- `[SHIFT]`: frequency shift amount (TODO: units?)

### `v`

TODO (probably will not support)

### `#i "[file]"`/`#it "[file]"` - Include
Include the contents of `[file]` at this location.
`[file]` is evaluated relative to the SPC file's parent directory.

> [!WARNING]  
> Includes are only evaluated as a single pass to prevent infinite recursion.
> If you include a file that contains another include, the nested include will
> be ignored.

### `e` - Exit Extended Area
Exits the extended area. The section after this becomes the data area.
