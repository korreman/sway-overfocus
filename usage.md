Syntax:

    sway_bfocus [--i3] <targets>

Targets:

    {split|group|float|output}-{u|d|l|r}{s|w|t|i}

Layout:

    split - horizontal and vertical splits
    group - tabs (horizontal) and stacks (vertical)
    float - floating containers
    workspace - workspaces, right/down is next, left/up is previous
    output - outputs

Direction:

    u - up
    d - down
    l - left
    r - right

Edge action:

    s - stop, do nothing
    w - wraparound to first or last container
    i - spill over and focus the inactive focus of container adjacent to parent
    t - spill over and traverse (focus the container closest to the current)

sway_bfocus runs a focus command that only considers the specified targets
while ignoring all other containers. Each target consists of a layout type,
a direction, and an edge case behavior.

Example:

    sway_bfocus split-lt float-lt output-ls
    sway_bfocus --i3 split-lt float-lt output-ls

This command will move left, though only between splits, floats, and outputs.
Tabs will be skipped, and a visible container physically left of the current one
will be focused instead.
