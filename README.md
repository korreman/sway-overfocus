# `sway_bfocus`

"Better" basic focusing commands for Sway WM.
Proof of concept.

This program allows you to separate the action of moving between splits
from the action of cycling through tabs and stacks.
This improves ergonomics of navigating in nested layouts that mix the two.

## Usage

```
sway_bfocus (splith|splitv|tabbed|stacked) (forward|backward) (cycle|nocycle)
```

The command takes a layout target, a direction, and a cycle setting.
It works similarly to the regular focus commands,
but will only focus neighbors in the first parent container
that matches the layout target.

### Example

An example setup:

Focus    | Keybind          |Command
---------|------------------|-----------------------------------------
up       | `$mod+k`         | `sway_bfocus splitv backward nocycle`
down     | `$mod+j`         | `sway_bfocus splitv forward nocycle`
left     | `$mod+h`         | `sway_bfocus splith backward nocycle`
right    | `$mod+l`         | `sway_bfocus splith forward nocycle`
prev tab | `$mod+Shift+Tab` | `sway_bfocus tabbed backward cycle`
next tab | `$mod+Tab`       | `sway_bfocus tabbed forward cycle`

Be wary of the keybind for focusing the previous tab,
as it is dangerously close to `$mod+Shift+Q`.

Remember to add `exec` in your sway config, 
e.g. `$mod+k exec sway_bfocus splitv backward nocycle`


## Installation
Clone the repository, 
run `cargo build --release`, 
copy the executable to somewhere on `$PATH`. 
(`~/.local/bin/` is probably a good choice). 



## TODO

- Showcase video
- Float support
- Ignoring singletons
- Multiple layout targets
- Directional focus between outputs
- Workspace wraparound
- Moving containers?
