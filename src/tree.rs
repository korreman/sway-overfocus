use serde::Deserialize;
use std::mem;

// Command types
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

// Tree types
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum PLayout {
    SplitH,
    SplitV,
    Stacked,
    Tabbed,
    Output,
    #[serde(other)]
    Other,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(from = "PLayout")]
pub enum Layout {
    Root,
    Output,
    Floats,
    Split {
        vertical: bool,
    },
    Group {
        vertical: bool,
    },
    #[serde(other)]
    Other,
}

impl From<PLayout> for Layout {
    fn from(l: PLayout) -> Self {
        match l {
            PLayout::SplitH => Layout::Split { vertical: false },
            PLayout::SplitV => Layout::Split { vertical: true },
            PLayout::Tabbed => Layout::Group { vertical: false },
            PLayout::Stacked => Layout::Group { vertical: true },
            PLayout::Output => Layout::Output,
            PLayout::Other => Layout::Other,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum Type {
    Root,
    Output,
    Con,
    FloatingCon,
    Workspace,
    Dockarea,
}

#[derive(Deserialize)]
struct PRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(from = "PRect")]
pub struct Rect {
    pub pos: Vec2,
    pub dim: Vec2,
}

impl From<PRect> for Rect {
    fn from(r: PRect) -> Rect {
        Rect {
            pos: Vec2 { x: r.x, y: r.y },
            dim: Vec2 {
                x: r.width,
                y: r.height,
            },
        }
    }
}

impl Rect {
    fn closest_point(&self, p: Vec2) -> Vec2 {
        Vec2 {
            x: i32::clamp(p.x, self.pos.x, self.pos.x + self.dim.x),
            y: i32::clamp(p.y, self.pos.y, self.pos.y + self.dim.y),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Vec2 {
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Tree {
    pub id: u32,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub ctype: Type,
    pub layout: Layout,
    pub rect: Rect,
    pub focused: bool,
    pub focus: Box<[u32]>,
    pub nodes: Box<[Tree]>,
    pub floating_nodes: Box<[Tree]>,
}

impl Tree {
    pub fn reform(&mut self) {
        self.layout = Layout::Root;
        for output in self.nodes.iter_mut() {
            for workspace in output.nodes.iter_mut() {
                let focus = mem::take(&mut workspace.focus);
                let nodes = mem::take(&mut workspace.nodes);
                let floats = mem::take(&mut workspace.floating_nodes);

                let (focus_nodes, focus_floats): (Vec<u32>, Vec<u32>) = focus
                    .iter()
                    .partition(|id| nodes.iter().any(|n| n.id == **id));

                workspace.focus =
                    Box::new(if focus.first() == focus_nodes.first() { [0, 1] } else { [1, 0] });

                let mut nodes_node = workspace.clone();
                nodes_node.id = 0;
                nodes_node.nodes = nodes;
                nodes_node.focus = focus_nodes.into_boxed_slice();

                let mut floats_node = workspace.clone();
                floats_node.id = 1;
                floats_node.nodes = floats;
                floats_node.focus = focus_floats.into_boxed_slice();
                floats_node.layout = Layout::Floats;

                workspace.nodes = Box::new([nodes_node, floats_node]);
                workspace.layout = Layout::Other;
            }
        }
    }

    pub fn focus_command(&self) -> Option<String> {
        let name = self.name.clone()?;
        let id = self.id;
        let cmd = match self.ctype {
            Type::Root => None,
            Type::Output => Some(format!("focus output {name}")),
            Type::Workspace => Some(format!("workspace {name}")),
            _ => Some(format!("[con_id={id}] focus")),
        }?;
        Some(cmd.to_string())
    }

    fn focus_idx(&self) -> Option<usize> {
        self.nodes.iter().enumerate().find_map(|(idx, n)| {
            if n.id == *self.focus.first()? {
                Some(idx)
            } else {
                None
            }
        })
    }

    fn focus_local(&self) -> Option<&Tree> {
        self.nodes.get(self.focus_idx()?)
    }

    fn select_leaf(&self, targets: &[Target]) -> &Tree {
        let mut t = self;
        loop {
            let target = t.match_targets(targets);
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
                    } else {
                        if target.backward {
                            t.nodes.last()
                        } else {
                            t.nodes.first()
                        }
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

    pub fn neighbor(&self, targets: &[Target]) -> Option<&Tree> {
        let mut t = self;
        let mut matching_parents = Vec::new();
        while !t.focused {
            if let Some(target) = t.match_targets(targets) {
                matching_parents.push((target, t));
            }
            if let Some(new_t) = t.focus_local() {
                t = new_t;
            } else {
                break;
            }
        }
        let neighbor = matching_parents
            .iter()
            .rev()
            .find_map(|(t, p)| p.neighbor_local(&t));
        Some(neighbor?.select_leaf(&targets))
    }

    fn match_targets(&self, targets: &[Target]) -> Option<Target> {
        let res = *targets
            .iter()
            .find(|target| match (target.kind, self.layout) {
                (Kind::Float, Layout::Floats) | (Kind::Output, Layout::Root) => true,
                (Kind::Split, Layout::Split { vertical })
                | (Kind::Group, Layout::Group { vertical }) => vertical == target.vertical,
                _ => false,
            })?;
        Some(res)
    }

    // Attempts to get a neighbor of focused child,
    // based on a list of targets.
    fn neighbor_local(&self, target: &Target) -> Option<&Tree> {
        let focus_idx = self.focus_idx()?;

        let res = if target.kind == Kind::Float || target.kind == Kind::Output {
            let component = |v: Vec2| if target.vertical { v.y } else { v.x };
            let middle = |r: Rect| component(r.pos) + component(r.dim) / 2;
            let focused = self.nodes[focus_idx].rect;

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

            let mut nodes: Vec<&Tree> = self.nodes.iter().collect();
            nodes.remove(focus_idx);

            let mut res = nodes
                .iter()
                .filter(|n| pred(focused, n.rect))
                .min_by_key(|n| dist(n.rect));
            if target.edge_mode == EdgeMode::Wrap {
                res = res.or(nodes
                    .iter()
                    .filter(|n| pred(n.rect, focused))
                    .max_by_key(|n| dist(n.rect)));
            }
            res.map(|&n| n)
        } else {
            let len = self.nodes.len();
            let idx = focus_idx + len;
            let idx = if target.backward { idx - 1 } else { idx + 1 };
            let idx = if target.edge_mode == EdgeMode::Wrap {
                Some(idx % len)
            } else {
                if len <= idx && idx < len * 2 {
                    Some(idx - len)
                } else {
                    None
                }
            };
            idx.map(|idx| &self.nodes[idx])
        };
        res.or(if target.edge_mode == EdgeMode::Stop { self.focus_local() } else { None })
    }
}
