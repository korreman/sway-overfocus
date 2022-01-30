# `sway_bfocus`

Better basic focusing commands for Sway WM.
Proof of concept.

## What/why?

This program allows you to separate the action of moving between splits
from the action of cycling through tabs and stacks.
This improves ergonomics of navigating in nested layouts that mix the two.

## Usage

```
sway_bfocus (splith|splitv|tabbed|stacked) (forward|backward) (cycle|nocycle)
```

The command takes a layout target, a direction, and a cycle setting.
It behaves like the normal `focus` commands,
but pretends that only the containers with a matching layout target exists.

Moving up the tree from the focused container,
we find the first ancestor that matches the target layout.
The next or previous child of that ancestor is then focused
down to a `focused_inactive` leaf window,
just as with the original commands.

The cycle setting decides what should happen
when navigating past the first or last child of a container.
If cycling, focus will wrap around.
If not cycling, focus "spills into" the next neighbor over,
just like the default commands.
That is, we move on to the next matching ancestor
and focus a neighbor from this instead.

### Example

Recommended setup:

Focus    | Keybind          |Command
---------|------------------|-----------------------------------------
up       | `$mod+k`         | `sway_bettertabs vsplit backward nocycle`
down     | `$mod+j`         | `sway_bettertabs vsplit forward nocycle`
left     | `$mod+h`         | `sway_bettertabs hsplit backward nocycle`
right    | `$mod+l`         | `sway_bettertabs hsplit forward nocycle`
prev tab | `$mod+Shift+Tab` | `sway_bettertabs tabbed backward cycle`
next tab | `$mod+Tab`       | `sway_bettertabs tabbed forward cycle`

Although you should be wary of that previous tab keybind,
it is dangerously close to `$mod+Shift+Q`.

## TODO

- Float support
- Ignoring singletons
- Multiple layout targets
- Directional focus between outputs
- Workspace wraparound
- Moving containers?
