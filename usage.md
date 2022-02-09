Syntax:

    sway_bfocus [target...]

Targets:

    [split|group|float|output]-[u|d|l|r][s|w|t|i]

Layouts:

    split - horizontal and vertical splits
    group - tabs (vertical) and stacks (horizontal)
    float - floating containers
    output - outputs

Directions:

    u - up
    d - down
    l - left
    r - right

Over-edge action:

    s - stop, do nothing
    w - wraparound to first or last container
    t - spill over and traverse (focus the container closest to the current)
    i - spill over and focus the inactive focus of adjacent parent container

This is a directional focus command that only considers the specified targets.
Each target consists of a layout type, a direction, and an edge case behavior.

Example:

    sway_bfocus split-lt float-lt output-ls

This command will move left, though only between splits, floats, and outputs.
Tabs will be skipped, and a visible container physically left of the current one
will be selected instead.
