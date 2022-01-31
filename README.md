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
e.g. `bindsym $mod+Tab exec sway_bfocus next wrap tabbed`.

## Usage

```
sway_bfocus (prev|next) (wrap|nowrap) (splith|splitv|tabbed|stacked)+
```

The command takes
a direction, a wrapping setting, and one or more target layouts.
It works like the regular focus commands,
but will only focus neighbors in the first parent container
that matches one of the layout targets.

### Example setup

The following setup should cover most use cases:

Focus          | Keybind          |Command
---------------|------------------|---------------------------------------
up             | `$mod+k`         | `sway_bfocus prev nowrap splitv`
down           | `$mod+j`         | `sway_bfocus next nowrap splitv`
left           | `$mod+h`         | `sway_bfocus prev nowrap splith`
right          | `$mod+l`         | `sway_bfocus next nowrap splith`
prev tab/stack | `$mod+Shift+Tab` | `sway_bfocus prev wrap tabbed stacked`
next tab/stack | `$mod+Tab`       | `sway_bfocus next wrap tabbed stacked`

## TODO

- Showcase video
- Float support
- Ignore singletons when wrapping
- Directional focus between outputs
- Workspace wraparound
- Moving containers?
