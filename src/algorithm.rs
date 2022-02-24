//! Neighbor-finding algorithm.
use super::tree::{Layout, Rect, Tree, Vec2, ID};
use log::{debug, trace};

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
    debug!("Performing downward focus traversal");
    let mut matching_parents = Vec::new();
    while !t.focused {
        debug!("Node {}, {:?}", t.id, t.name);
        if let Some(target) = match_targets(t, targets) {
            trace!("Matched {target:?}");
            matching_parents.push((target, t));
        }
        if let Some(new_t) = t.focus_local() {
            t = new_t;
        } else {
            trace!("No focused child, stopping");
            break;
        }
    }
    debug!("Searching ancestors for neighbor");
    // Search backwards through the stack of parents for a valid neighbor.
    // Returns an `Option<Option<_>>`,
    // `Some(None)` is used to stop early when matching a target with `EdgeMode::Stop`.
    let neighbor = matching_parents.iter().rev().find_map(|(t, p)| {
        debug!("Parent {}", p.id);
        let n = neighbor_local(p, t);
        if let Some(n) = n {
            debug!("Found neighbor: {}", n.id);
        }
        if t.edge_mode == EdgeMode::Stop {
            debug!("Target is stopping, forcing return");
            Some(n)
        } else {
            n.map(Some)
        }
    })??;
    debug!("Selecting a leaf descendant of neighbor");
    Some(select_leaf(neighbor, targets))
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
    trace!("Finding neighbor for {target:?}");

    if target.kind == Kind::Float || target.kind == Kind::Output {
        let focus_id = *tree.focus.first()?;
        let focused = &tree.nodes[focus_idx];
        trace!("Focused {:?}", focused.rect);

        // Floats and outputs are chosen based on dimensions and position.
        // Both are first filtered by a criteria,
        // then the minimum/maximum container is chosen based on some distance measure.

        // Selects either the `x` or `y` component based on verticality
        let component = |v: Vec2| if target.vertical { v.y } else { v.x };
        // Computes the middle across the selected component
        let middle = |r: Rect| component(r.pos) + component(r.dim) / 2;

        // Filtering predicate
        let pred = |t: &Tree, flip: bool| {
            trace!("Testing node {}, {:?}", t.id, t.rect);
            let (a, b) = if flip { (t.rect, focused.rect) } else { (focused.rect, t.rect) };
            let p = t.id != focus_id // Discard currently focused node
                && match target.kind {
                    // For floats, the middle must be past the focused middle on the chosen axis
                    Kind::Float => {
                        // If perfectly aligned, IDs are used for bounds checks as well
                        let id_bound = flip == (focus_id < t.id);
                        middle(a) < middle(b) || (middle(a) == middle(b) && id_bound)
                    }
                    // For outputs, their rects must be strictly past the focused rect
                    Kind::Output => component(a.pos) + component(a.dim) <= component(b.pos),
                    _ => unreachable!(),
                };
            trace!("Passes filter: {p}");
            p
        };

        // Distance measure
        let dist_key = |t: &Tree, flip: bool| {
            trace!("Measuring node {}, {:?}", t.id, t.rect);
            let pos_dist = match target.kind {
                // Floats are chosen by component-wise distance
                Kind::Float => (middle(t.rect) - middle(focused.rect)).saturating_abs(),
                // Outputs are chosen by euclidean distance from focused center to closest point
                Kind::Output => {
                    let center = Vec2 {
                        x: focused.rect.pos.x + focused.rect.dim.x / 2,
                        y: focused.rect.pos.y + focused.rect.dim.y / 2,
                    };
                    let p = t.rect.closest_point(center);
                    (center.x - p.x) * (center.x - p.x) + (center.y - p.y) * (center.y - p.y)
                }
                _ => unreachable!(),
            };
            trace!("Distance {pos_dist}");
            // IDs are included to resolve ties
            let id_order = if flip { t.id } else { ID::MAX - t.id };
            (pos_dist, id_order)
        };

        // Filter by predicate and select closest node
        let mut res = tree
            .nodes
            .iter()
            .filter(|n| pred(n, target.backward))
            .min_by_key(|n| dist_key(n, target.backward));
        // If wrapping, filter by flipped (not negated) predicate and select furthest node
        if target.edge_mode == EdgeMode::Wrap {
            trace!("Finding potential wraparound target");
            let wrap_target = tree
                .nodes
                .iter()
                .filter(|n| pred(n, !target.backward))
                .max_by_key(|n| dist_key(n, !target.backward));
            res = res.or(wrap_target).or(Some(focused));
        }
        res
    } else {
        trace!("Selecting neighbor by index");
        trace!(
            "Focused subnode index: {focus_idx} out of {}",
            tree.nodes.len() - 1
        );
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
        trace!("Resulting index: {idx:?}");
        idx.map(|idx| &tree.nodes[idx])
    }
}

/// Find a leaf in a (presumed) neighboring container, respecting target edge-modes
fn select_leaf<'a>(mut t: &'a Tree, targets: &[Target]) -> &'a Tree {
    loop {
        debug!("Node {}, {:?}", t.id, t.name);
        // Match the current node with targets
        let target = match_targets(t, targets);
        let new_t = match target {
            // If the target has traversal mode,
            // choose the closest neighbor to focused node.
            // Fx. if moving right, the leftmost child is selected.
            Some(target) if target.edge_mode == EdgeMode::Traverse => {
                trace!("Matched traversing {:?}", target.kind);
                // For floats, this entails finding the left/right/top/bottom-most node
                if target.kind == Kind::Float {
                    trace!("Float container, selecting left/right/top/bottom-most child");
                    let key = |n: &&Tree| {
                        let center = if target.vertical {
                            n.rect.pos.y + n.rect.dim.y / 2
                        } else {
                            n.rect.pos.x + n.rect.dim.x / 2
                        };
                        (center, ID::MAX - n.id)
                    };
                    if target.backward {
                        t.nodes.iter().max_by_key(key)
                    } else {
                        t.nodes.iter().min_by_key(key)
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
    debug!("Selected leaf {}", t.id);
    t
}
