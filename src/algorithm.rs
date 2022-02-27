//! Neighbor-finding algorithm.
use crate::tree::{closest_point, focus_idx, focus_local, Vec2};
use log::{debug, trace};
use swayipc::{Node, NodeLayout, NodeType, Rect};

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
pub fn neighbor<'a>(mut t: &'a Node, targets: &[Target]) -> Option<&'a Node> {
    // Go down the focus path and collect matching parents.
    debug!("Finding focus path");
    let mut path = Vec::new();
    while !t.focused {
        debug!("Node {}, {:?}", t.id, t.name);
        path.push(t);
        if let Some(new_t) = focus_local(t) {
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
    let neighbor = path.iter().rev().find_map(|parent| {
        debug!("Parent {}", parent.id);
        let target = match_targets(parent, targets)?;
        trace!("Matched {target:?}");
        let n = neighbor_local(parent, &target);
        if target.edge_mode == EdgeMode::Stop {
            debug!("Target is stopping, forcing return");
            Some(n)
        } else {
            n.map(Some)
        }
    })??;
    debug!("Found neighbor: {}", neighbor.id);
    debug!("Selecting a leaf descendant of neighbor");
    Some(select_leaf(neighbor, targets))
}

/// Finds a parent that contains direct children matching one of the `targets`.
fn match_targets(node: &Node, targets: &[Target]) -> Option<Target> {
    let focus = *node.focus.first()?;
    let float_focused = node.floating_nodes.iter().any(|c| c.id == focus);
    let res = *targets.iter().find(|target| match target.kind {
        Kind::Output => node.node_type == NodeType::Root,
        Kind::Workspace => node.node_type == NodeType::Output,
        Kind::Split => {
            !float_focused
                && (!target.vertical && node.layout == NodeLayout::SplitH
                    || target.vertical && node.layout == NodeLayout::SplitV)
        }
        Kind::Group => {
            !float_focused
                && (!target.vertical && node.layout == NodeLayout::Tabbed
                    || target.vertical && node.layout == NodeLayout::Stacked)
        }
        Kind::Float => float_focused,
    })?;
    Some(res)
}

/// Tries to find a neighbor of the focused child of the top node in `tree`,
/// according to the given target.
fn neighbor_local<'a>(tree: &'a Node, target: &Target) -> Option<&'a Node> {
    let (focus_idx, children) = focus_idx(tree)?;

    if target.kind == Kind::Float || target.kind == Kind::Output {
        let focus_id = *tree.focus.first()?;
        let focused = &children[focus_idx];
        trace!("Focused {:?}", focused.rect);

        // Selects x or y component of a rect, based on whether the target is horizontal/vertical
        let component = |r: &Rect| if target.vertical { (r.y, r.height) } else { (r.x, r.width) };

        // Computes a distance to the focused node.
        // Handles directions and filters out irrelevant neighbors.
        let dist = |t: &Node, flip: bool| -> Option<(i32, i64)> {
            trace!("Computing distance to {}", t.id);
            if t.id == focus_id { return None; }
            let (a, b) = if flip { (&t.rect, &focused.rect) } else { (&focused.rect, &t.rect) };
            let ((a_pos, a_dim), (b_pos, b_dim)) = (component(a), component(b));
            let (a_mid, b_mid) = (a_pos + a_dim / 2, b_pos + b_dim / 2);
            let a_edge = a_pos + a_dim;
            trace!("A component: ({a_pos}, {a_dim}), B component: ({b_pos}, {b_dim})");
            trace!("A middle: {a_mid}, B middle: {b_mid}");
            trace!("A edge: {a_edge}");

            let dist = match target.kind {
                // Floats are compared by distance of centers on one axis
                Kind::Float if a_mid < b_mid || (a_mid == b_mid && flip == (t.id > focus_id)) => {
                    Some((b_mid - a_mid).saturating_abs())
                }
                // Outputs are compared by euclidean distance to center of focused node
                Kind::Output if a_edge <= b_pos => {
                    let c = Vec2 {
                        x: focused.rect.x + focused.rect.width / 2,
                        y: focused.rect.y + focused.rect.height / 2,
                    };
                    let p = closest_point(&t.rect, &c);
                    Some((c.x - p.x) * (c.x - p.x) + (c.y - p.y) * (c.y - p.y))
                }
                _ => None,
            }?;
            trace!("Distance: {dist}");
            Some((dist, if flip { t.id } else { -t.id }))
        };
        // Select the closest neighbor to focused child,
        // or furthest in the opposite direction if wrapping.
        let mut res = children
            .iter()
            .filter_map(|n| Some((dist(n, target.backward)?, n)))
            .min_by_key(|(d, _)| *d)
            .map(|(_, node)| node);
        if res.is_none() && target.edge_mode == EdgeMode::Wrap {
            trace!("No neighbor, searching for wraparound target");
            let wrap_target = children
                .iter()
                .filter_map(|n| Some((dist(n, !target.backward)?, n)))
                .max_by_key(|(d, _)| *d)
                .map(|(_, node)| node);
            // Also includes focused container as a last resort.
            // Because this allows traversing wraparound for multiple outputs.
            res = wrap_target.or(Some(focused));
        }
        res
    } else {
        trace!("Selecting neighbor by index");
        let len = children.len();
        trace!("Focused subnode index: {focus_idx} out of {}", len - 1);
        // The remaining targets can be chosen by index, disregarding verticality
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
        idx.map(|idx| &children[idx])
    }
}

/// Find a leaf in a (presumed) neighboring container, respecting target edge-modes
fn select_leaf<'a>(mut t: &'a Node, targets: &[Target]) -> &'a Node {
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
                    let key = |n: &&Node| {
                        let center = if target.vertical {
                            n.rect.y + n.rect.height / 2
                        } else {
                            n.rect.x + n.rect.width / 2
                        };
                        (center, -n.id)
                    };
                    if target.backward {
                        t.floating_nodes.iter().max_by_key(key)
                    } else {
                        t.floating_nodes.iter().min_by_key(key)
                    }
                // We don't handle outputs, as we will never move from one `Root` to another.
                // For other container types, we can just select the first or last.
                } else if target.backward {
                    t.nodes.last()
                } else {
                    t.nodes.first()
                }
            }
            _ => focus_local(t),
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
