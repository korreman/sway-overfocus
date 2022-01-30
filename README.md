# `sway_bfocus`

"Better" tab and stack navigation for Sway WM.
Proof of concept.

This program lets you
create one set of keybinds exclusively for cycling through tabs/stacks,
and another set exclusively for navigating between splits.
The result is that switching focus generally can be performed in one action
rather than some sequence of `focus parent` and `focus [direction]` actions.

## Installation

Clone the repository,
run `cargo build --release`,
and copy the executable to a location in your`$PATH`
(`~/.local/bin/` is probably a good choice).

Commands can then be added to the config,
e.g. `bindsym $mod+k exec sway_bfocus splitv backward nocycle`.

## Usage

```
sway_bfocus (splith|splitv|tabbed|stacked) (forward|backward) (cycle|nocycle)
```

The command takes a layout target, a direction, and a cycle setting.
It works similarly to the regular focus commands,
but will only focus neighbors in the first parent container
that matches the layout target.

### Example

See below for a simple configuration.
This setup doesn't handle stacks,
but should be enough for most other use cases.
Consider using a different keybind for focusing the previous tab,
as the suggestion is dangerously close to `$mod+Shift+q`.

Focus    | Keybind          |Command
---------|------------------|-----------------------------------------
up       | `$mod+k`         | `sway_bfocus splitv backward nocycle`
down     | `$mod+j`         | `sway_bfocus splitv forward nocycle`
left     | `$mod+h`         | `sway_bfocus splith backward nocycle`
right    | `$mod+l`         | `sway_bfocus splith forward nocycle`
prev tab | `$mod+Shift+Tab` | `sway_bfocus tabbed backward cycle`
next tab | `$mod+Tab`       | `sway_bfocus tabbed forward cycle`

## TODO

- Showcase video
- Float support
- Ignoring singletons
- Multiple layout targets
- Directional focus between outputs
- Workspace wraparound
- Moving containers?
