use super::tree::{Layout, Rect, Tree, Vec2};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub kind: Kind,
    pub backward: bool,
    pub vertical: bool,
    pub edge_mode: EdgeMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Split,
    Group,
    Float,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeMode {
    Stop,
    Wrap,
    Traverse,
    Inactive,
}

fn match_targets(tree: &Tree, targets: &[Target]) -> Option<Target> {
    let res = *targets
        .iter()
        .find(|target| match (target.kind, tree.layout) {
            (Kind::Float, Layout::Floats) | (Kind::Output, Layout::Root) => true,
            (Kind::Split, Layout::Split { vertical })
            | (Kind::Group, Layout::Group { vertical }) => vertical == target.vertical,
            _ => false,
        })?;
    Some(res)
}

fn select_leaf<'a>(mut t: &'a Tree, targets: &[Target]) -> &'a Tree {
    loop {
        let target = match_targets(t, targets);
        let new_t = match target {
            Some(target) if target.edge_mode == EdgeMode::Traverse => {
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
                } else if target.backward {
                    t.nodes.last()
                } else {
                    t.nodes.first()
                }
            }
            _ => t.focus_local(),
        };
        if let Some(new_t) = new_t {
            t = new_t;
        } else {
            break;
        }
    }
    t
}

pub fn neighbor<'a>(mut t: &'a Tree, targets: &[Target]) -> Option<&'a Tree> {
    // Traverse down the tree, following the focus path.
    let mut matching_parents = Vec::new();
    while !t.focused {
        if let Some(target) = match_targets(t, targets) {
            matching_parents.push((target, t));
        }
        if let Some(new_t) = t.focus_local() {
            t = new_t;
        } else {
            break;
        }
    }
    // Search backwards through the stack of parents with a matching neighbor.
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

// Attempts to get a neighbor of focused child,
// based on a list of targets.
fn neighbor_local<'a>(tree: &'a Tree, target: &Target) -> Option<&'a Tree> {
    let focus_idx = tree.focus_idx()?;

    if target.kind == Kind::Float || target.kind == Kind::Output {
        let component = |v: Vec2| if target.vertical { v.y } else { v.x };
        let middle = |r: Rect| component(r.pos) + component(r.dim) / 2;
        let focused = tree.nodes[focus_idx].rect;

        let pred = |a: Rect, b: Rect| {
            let (a, b) = if target.backward { (b, a) } else { (a, b) };
            match target.kind {
                // TODO: Handle perfectly aligned floats.
                Kind::Float => middle(a) <= middle(b),
                Kind::Output => component(a.pos) + component(a.dim) <= component(b.pos),
                _ => unreachable!(),
            }
        };

        let dist = |n: Rect| match target.kind {
            Kind::Float => (middle(n) - middle(focused)).saturating_abs(),
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

        let mut nodes: Vec<&Tree> = tree.nodes.iter().collect();
        nodes.remove(focus_idx);

        let mut res = nodes
            .iter()
            .filter(|n| pred(focused, n.rect))
            .min_by_key(|n| dist(n.rect));
        if target.edge_mode == EdgeMode::Wrap {
            let wrap_target = nodes
                .iter()
                .filter(|n| pred(n.rect, focused))
                .max_by_key(|n| dist(n.rect));
            res = res.or(wrap_target);
        }
        res.copied()
    } else {
        let len = tree.nodes.len();
        let idx = focus_idx + len;
        let idx = if target.backward { idx - 1 } else { idx + 1 };
        let idx = if target.edge_mode == EdgeMode::Wrap {
            Some(idx % len)
        } else if len <= idx && idx < len * 2 {
            Some(idx - len)
        } else {
            None
        };
        idx.map(|idx| &tree.nodes[idx])
    }
}
