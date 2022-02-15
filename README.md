# `sway_bfocus`

"Better" focus navigation for the Sway and i3 window managers.

![Demo GIF](demo.gif)

The primary objective of this program is to
create one set of keybinds exclusively for cycling through tabs/stacks,
and another set exclusively for navigating between splits.
The result is that switching focus generally can be performed in one action
rather than some sequence of `focus parent` and `focus [direction]` actions.

## Installation instructions

The project compiles to a standalone binary
that interfaces with the WM using `swaymsg`
(or `i3-msg` when given an `--i3` flag).

Build with `cargo build --release` using `rustc` v1.58 or higher.
Copy the binary from `target/release/sway_bfocus`
to a location in your `$PATH`,
typically `~/.local/bin`.
Then insert/replace keybinds to run `exec "sway_bfocus..."` commands
in your sway configuration.

See the [usage](usage.md) page for details on constructing focus commands.
The following config section is a good starting point,
but commands can be configured granularly to suit your needs.

    bindsym $mod+h exec 'sway_bfocus split-lt float-lt output-ls'
    bindsym $mod+j exec 'sway_bfocus split-dt float-dt output-ds'
    bindsym $mod+k exec 'sway_bfocus split-ut float-ut output-us'
    bindsym $mod+l exec 'sway_bfocus split-rt float-rt output-rs'
    bindsym $mod+Tab exec 'sway_bfocus group-lw group-dw'
    bindsym $mod+Shift+Tab exec 'sway_bfocus group-rw group-uw'
