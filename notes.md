# Supporting floats and outputs switching

The hard thing about outputs and floats is
that they aren't meaningfully arranged in the tree.
It's necessary to change focus based on coordinates+dimensions.

This doesn't immediately connect with the current interface and design.
So how should the API and design be instead?

## Option 1: Replace with directional commands

This is obviously more in line with how things are actually arranged,
but it would need a separate interface from tabs and stacks.
The interface would be too fractured.

## Option 2: Add `floath`, `floatv` pseudolayouts

This would be consistent with the current prev/next syntax.
On the other hand, it allows for incompatible combinations of target layouts.
Eg. `sway_bfocus next nowrap floath floatv`.
I don't think that's a big deal though.
You can view `next floath` as a synonym for `float right`.

The hybrid command to go right would then be:

    sway_bfocus next splith floath outputh

All new target layouts:

    float, floath, floatv, output, outputh, outputv

You could represent a focus target as
(tabs and stacks are horizontal and vertical groups):

    Movement = Type, Orientation, Direction, wrap?, traverse?
    Type = split | group | float | output
    Orientation = vertical | horizontal
    Direction = next | previous

## Wrapping, DFS

When spilling into a new output, say from the right,
the leftmost container is selected.
This is inconsistent with the rest of movement,
but it adds two new aspects to consider,
as it should be possible to:

1. Specify depth-first traversal rather than inactive focus.
2. Specify different behaviors for each layouts in a target union.

## Alternative logic for directional movement

So floats are arranged by their centers.
I think I'd prefer it if sorted by upper left corner instead.
Then again, that isn't as agnostic as the centers.

It could be kind of cool if you could navigate directionally
in a way that is more consistent with how things look on screen.
Then I'd consider doing up/down/left/right.
But I think that might be impossible without making things too unpredictable
or leaving containers unreachable.

# Sway defaults

Moving between floating containers will order containers on center axis,
only on the relevant axis.

Spilling into the next output will select the closest container
rather than the inactive focus.

Adjacent outputs are selected by the closest distance
from center of current output to closest point within other outputs.

At least that's what I think it's supposed to do.
The `wlroots` code seems to be doing something kind of different.
Honestly looks like a bug.
