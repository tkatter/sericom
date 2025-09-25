# Appearance

All of the configuration options under the `[appearance]` section share the same
set of values. Generally speaking, it shouldn't matter how you write the value
i.e. "darkgreen", "dark-green", "GREEN", "dark blue". However, if you come across
an issue, feel free to open a github [issue](https://github.com/tkatter/sericom).

> [!NOTE] The "dark-" variants
> I use Ghostyy for my terminal, and for me, all of the "dark-" variants produce the same
> color as their normal counterpart. I'm not sure if it is related to Ghostyy or if it is
> this way across other terminals. If it is a common issue, I may remove the "dark-" variants
> in the future to avoid confusion, unless I can find a way to make them work properly.

The valid options are as follows:

- Black,
- Blue,
- Cyan,
- Dark-Blue,
- Dark-Cyan,
- Dark-Green,
- Dark-Grey,
- Dark-Magenta,
- Dark-Red,
- Dark-Yellow,
- Default
- Green,
- Grey,
- Magenta,
- Red,
- White,
- Yellow,

# Defaults

- `out-dir`: Path to a directory
- `exit-script`: Path to a script that will be run after writing to a file
  - Currently only works on Linux
  - The script will be passed an environment variable `SERICOM_OUT_FILE` which
    has the absolute path to the file that was just written
