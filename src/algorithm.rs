//! Neighbor-finding algorithm.
use super::tree::{Layout, Rect, Tree, Vec2};

/// A target with which to search for a neighbor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    /// The kind of neighbor to find.
    pub kind: Kind,
    /// Whether to find the succeeding or preceding neighbor.
    pub backward: bool,
    /// Whether to switch horizontally or vertically.
    pub vertical: bool,
    /// Moving-into-edge handling.
    pub edge_mode: EdgeMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Split,
    Group,
    Float,
    Workspace,
    Output,
}

/// Describes what to do when attempting to move past the last or first child of a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeMode {
    /// Do nothing, don't change focus.
    Stop,
    /// Wrap around and focus the first or last child.
    Wrap,
    /// Spill over, focus the closest node in new parent.
    Traverse,
    /// Spill over, focus the inactive focus of the new parent.
    Inactive,
}

/// Find a neighbor matching one of the `targets`.
pub fn neighbor<'a>(mut t: &'a Tree, targets: &[Target]) -> Option<&'a Tree> {
    // Go down the focus path and collect matching parents.
    let mut matching_parents = Vec::new();
    while !t.focused {
        if let Some(target) = match_targets(t, targets) {
            if t.nodes.len() > 1 {
                matching_parents.push((target, t));
            }
        }
        if let Some(new_t) = t.focus_local() {
            t = new_t;
        } else {
            break;
        }
    }
    // Search backwards through the stack of parents for a valid neighbor.
    // Returns an `Option<Option<_>>`,
    // `Some(None)` is used to stop early if matching a target with `EdgeMode::Stop`.
    let neighbor = matching_parents.iter().rev().find_map(|(t, p)| {
        let n = neighbor_local(p, t);
        if t.edge_mode == EdgeMode::Stop {
            Some(n)
        } else {
            n.map(Some)
        }
    });
    Some(select_leaf(neighbor??, targets))
}

/// Finds a parent that contains direct children matching one of the `targets`.
fn match_targets(tree: &Tree, targets: &[Target]) -> Option<Target> {
    let res = *targets
        .iter()
        .find(|target| match (target.kind, tree.layout) {
            (Kind::Float, Layout::Floats)
            // Outputs contain workspaces
            | (Kind::Workspace, Layout::Output)
            // Root contains outputs
            | (Kind::Output, Layout::Root) => true,
            // For splits and groups, orientation must also match
            (Kind::Split, Layout::Split { vertical })
            | (Kind::Group, Layout::Group { vertical }) => vertical == target.vertical,
            _ => false,
        })?;
    Some(res)
}

/// Tries to find a neighbor of the focused child of the top node in `tree`,
/// according to the given target.
fn neighbor_local<'a>(tree: &'a Tree, target: &Target) -> Option<&'a Tree> {
    let focus_idx = tree.focus_idx()?;

    if target.kind == Kind::Float || target.kind == Kind::Output {
        let focused = tree.nodes[focus_idx].rect;

        // Floats and outputs are chosen based on dimensions and position.
        // Both filter by a criteria,
        // then choose a container based on some distance measure.

        // Selects either the `x` or `y` component based on verticality
        let component = |v: Vec2| if target.vertical { v.y } else { v.x };
        // Computes the middle across the selected component
        let middle = |r: Rect| component(r.pos) + component(r.dim) / 2;

        // TODO: Handle perfectly aligned floats
        // Filtering predicate
        let pred = |a: Rect, b: Rect| {
            let (a, b) = if target.backward { (b, a) } else { (a, b) };
            match target.kind {
                // For floats, the middle must be past the focused middle on the chosen axis
                Kind::Float => middle(a) <= middle(b),
                // For outputs, their rects must be strictly past the focused rect
                Kind::Output => component(a.pos) + component(a.dim) <= component(b.pos),
                _ => unreachable!(),
            }
        };

        // Distance measure
        let dist = |n: Rect| match target.kind {
            // Floats are chosen by component-wise distance
            Kind::Float => (middle(n) - middle(focused)).saturating_abs(),
            // Outputs are chosen by euclidean distance from focused center to closest point
            Kind::Output => {
                let center = Vec2 {
                    x: focused.pos.x + focused.dim.x / 2,
                    y: focused.pos.y + focused.dim.y / 2,
                };
                let p = n.closest_point(center);
                (center.x - p.x) * (center.x - p.x) + (center.y - p.y) * (center.y - p.y)
            }
            _ => unreachable!(),
        };

        // Only select from nodes besides the focused one
        let mut nodes: Vec<&Tree> = tree.nodes.iter().collect();
        nodes.remove(focus_idx);

        // Filter by predicate and select closest node
        let mut res = nodes
            .iter()
            .filter(|n| pred(focused, n.rect))
            .min_by_key(|n| dist(n.rect));
        // If wrapping, filter by flipped (not negated) predicate and select furthest node
        if target.edge_mode == EdgeMode::Wrap {
            let wrap_target = nodes
                .iter()
                .filter(|n| pred(n.rect, focused))
                .max_by_key(|n| dist(n.rect));
            res = res.or(wrap_target);
        }
        res.copied()
    } else {
        // The remaining targets can be chosen by index, disregarding verticality
        let len = tree.nodes.len();
        // Add length to avoid underflow
        let idx = focus_idx + len;
        let idx = if target.backward { idx - 1 } else { idx + 1 };
        let idx = if target.edge_mode == EdgeMode::Wrap {
            // If wrapping, calculate modulo the number of children
            Some(idx % len)
        } else if len <= idx && idx < len * 2 {
            // Otherwise perform a range check and subtract length again
            Some(idx - len)
        } else {
            None
        };
        idx.map(|idx| &tree.nodes[idx])
    }
}

/// Find a leaf in a (presumed) neighboring container, respecting target edge-modes
fn select_leaf<'a>(mut t: &'a Tree, targets: &[Target]) -> &'a Tree {
    loop {
        // Match the current node with targets
        let target = match_targets(t, targets);
        let new_t = match target {
            // If the target has traversal mode,
            // choose the closest neighbor to focused node.
            // Fx. if moving right, the leftmost child is selected.
            Some(target) if target.edge_mode == EdgeMode::Traverse => {
                // For floats, this entails finding the left/right/top/bottom-most node
                if target.kind == Kind::Float {
                    let center = |n: &&Tree| {
                        if target.vertical {
                            n.rect.pos.y + n.rect.dim.y / 2
                        } else {
                            n.rect.pos.x + n.rect.dim.x / 2
                        }
                    };
                    if target.backward {
                        t.nodes.iter().max_by_key(center)
                    } else {
                        t.nodes.iter().min_by_key(center)
                    }
                // We don't handle outputs, as we will never move from one `Root` to another.
                // For other container types, we can just select the first or last.
                } else if target.backward {
                    t.nodes.last()
                } else {
                    t.nodes.first()
                }
            }
            _ => t.focus_local(),
        };
        // Keep selecting children until we reach a leaf
        if let Some(new_t) = new_t {
            t = new_t;
        } else {
            break;
        }
    }
    t
}
