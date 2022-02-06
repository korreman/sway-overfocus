use serde::Deserialize;
use std::env;
use std::process::Command;

fn main() {
    let args: Box<[String]> = env::args().collect();
    if let Some(targets) = parse_args(&args) {
        let mut get_tree = Command::new("swaymsg");
        get_tree.arg("-t").arg("get_tree");
        let input = get_tree
            .output()
            .expect("failed to retrieve container tree");
        let tree: PTree = serde_json::from_slice(input.stdout.as_slice())
            .expect("failed to parse container tree");
        let tree = tree.process().unwrap();
        if let Some(neighbor) = tree.neighbor(&targets) {
            let mut cmd = Command::new("swaymsg");
            let focus_cmd = neighbor.focus_command().expect("no valid focus command");
            println!("{focus_cmd}");
            cmd.arg(focus_cmd);
            cmd.spawn()
                .and_then(|mut p| p.wait())
                .expect("failed to send focus command");
        }
    } else {
        let _bin_name = &args[0];
        println!("usage message");
    }
}

fn parse_args(args: &[String]) -> Option<Box<[Target]>> {
    if args.len() < 2 {
        return None;
    }
    let targets = args[1..].iter().map(|arg| {
        let split = arg.split_once('-')?;
        let kind = match split.0 {
            "split" => Some(Kind::Split),
            "group" => Some(Kind::Group),
            "float" => Some(Kind::Float),
            "output" => Some(Kind::Output),
            _ => None,
        }?;
        if let [dir, wrap] = split.1.as_bytes() {
            let (backward, vertical) = match dir {
                0x75 => Some((true, true)),
                0x64 => Some((false, true)),
                0x6c => Some((true, false)),
                0x72 => Some((false, false)),
                _ => None,
            }?;
            let wrap = match wrap {
                0x77 => Some(true),
                0x73 => Some(false),
                _ => None,
            }?;
            Some(Target {
                kind,
                backward,
                vertical,
                wrap,
            })
        } else {
            None
        }
    });
    targets.collect()
}

// Command types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Target {
    kind: Kind,
    backward: bool,
    vertical: bool,
    wrap: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Split,
    Group,
    Float,
    Output,
}

// Tree types
#[derive(Debug, Clone)]
struct Tree {
    id: u32,
    ctype: PType,
    name: Option<String>,
    layout: Layout,
    rect: Rect,
    is_focused: bool,
    focus: Option<usize>,
    nodes: Box<[Tree]>,
}

#[derive(Debug, Clone, Copy)]
enum Layout {
    Group { vertical: bool },
    Split { vertical: bool },
    Floats,
    Outputs,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rect {
    pos: Vec2,
    dim: Vec2,
}

impl Rect {
    fn closest_point(&self, p: Vec2) -> Vec2 {
        Vec2 {
            x: i32::clamp(p.x, self.pos.x, self.pos.x + self.dim.x),
            y: i32::clamp(p.y, self.pos.y, self.pos.y + self.dim.y),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Vec2 {
    x: i32,
    y: i32,
}

impl Tree {
    fn focus_command(&self) -> Option<String> {
        let name = self.name.clone()?;
        let id = self.id;
        let cmd = match self.ctype {
            PType::Root => None,
            PType::Output => Some(format!("focus output {name}")),
            PType::Workspace => Some(format!("workspace {name}")),
            _ => Some(format!("[con_id={id}] focus")),
        }?;
        Some(cmd.to_string())
    }
    fn focus_local(&self) -> Option<&Tree> {
        self.nodes.get(self.focus?)
    }

    fn focus(&self) -> &Tree {
        let mut t = self;
        while let Some(idx) = t.focus {
            if t.is_focused {
                break;
            }
            t = t.nodes.get(idx).expect("Focused child doesn't exist");
        }
        t
    }

    fn neighbor(&self, targets: &[Target]) -> Option<&Tree> {
        let mut t = self;
        let mut deepest_neighbor = None;
        while !t.is_focused {
            deepest_neighbor = t.neighbor_local(targets).or(deepest_neighbor);
            if let Some(new_t) = t.focus_local() {
                t = new_t;
            } else {
                break;
            }
        }
        Some(deepest_neighbor?.focus())
    }

    // Attempts to get a neighbor of focused child,
    // based on a list of targets.
    fn neighbor_local(&self, targets: &[Target]) -> Option<&Tree> {
        let target = *targets
            .iter()
            .find(|target| match (target.kind, self.layout) {
                (Kind::Float, Layout::Floats) | (Kind::Output, Layout::Outputs) => true,
                (Kind::Split, Layout::Split { vertical })
                | (Kind::Group, Layout::Group { vertical }) => vertical == target.vertical,
                _ => false,
            })?;

        match target {
            Target {
                kind: Kind::Float,
                vertical,
                backward,
                wrap,
            } => {
                let get = |v: Vec2| if !vertical { v.x } else { v.y };
                let center = |r: Rect| get(r.pos) + get(r.dim) / 2;

                let sign = if !backward { 1 } else { -1 };
                let focused = center(self.nodes[self.focus?].rect);

                let mut res = self
                    .nodes
                    .iter()
                    .map(|n| ((center(n.rect) - focused) * sign, n))
                    .filter(|&(d, _)| d > 0)
                    .min_by_key(|&(d, _)| d);

                if wrap {
                    res = res.or(self
                        .nodes
                        .iter()
                        .map(|n| (focused - (center(n.rect)) * sign, n))
                        .filter(|&(d, _)| d < 0)
                        .max_by_key(|&(d, _)| d))
                };

                Some(res?.1)
            }

            Target {
                kind: Kind::Output,
                backward,
                vertical,
                wrap,
            } => {
                let focused = self.nodes[self.focus?].rect;
                let center = Vec2 {
                    x: focused.pos.x + focused.dim.x / 2,
                    y: focused.pos.y + focused.dim.y / 2,
                };

                let rearrange = |a: Rect, b: Rect| if backward { (b, a) } else { (a, b) };
                let component = |r: Vec2| if vertical { r.y } else { r.x };

                let mut res = self
                    .nodes
                    .iter()
                    .filter(|n| {
                        let (a, b) = rearrange(focused, n.rect);
                        component(a.pos) + component(a.dim) <= component(b.pos)
                    })
                    .min_by_key(|n| {
                        let p = n.rect.closest_point(center);
                        (center.x - p.x) * (center.x - p.x) + (center.y - p.y) * (center.y - p.y)
                    });

                if wrap {
                    res = res.or(self
                        .nodes
                        .iter()
                        .filter(|n| {
                            let (a, b) = rearrange(n.rect, focused);
                            component(a.pos) + component(a.dim) <= component(b.pos)
                        })
                        .max_by_key(|n| {
                            let p = n.rect.closest_point(center);
                            (center.x - p.x) * (center.x - p.x)
                                + (center.y - p.y) * (center.y - p.y)
                        }));
                };
                res
            }

            // For groups and splits, simply go to previous or next child (and handle wrapping).
            Target { backward, wrap, .. } => {
                let len = self.nodes.len();
                let idx = self.focus? + len;
                let idx = if !backward { idx + 1 } else { idx - 1 };
                let idx = if wrap {
                    Some(idx % len)
                } else {
                    if len <= idx && idx < len * 2 {
                        Some(idx - len)
                    } else {
                        None
                    }
                }?;
                Some(&self.nodes[idx])
            }
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum PLayout {
    SplitH,
    SplitV,
    Stacked,
    Tabbed,
    Dockarea,
    Output,
    None,
}

impl PLayout {
    fn process(&self) -> Layout {
        match self {
            PLayout::SplitH => Layout::Split { vertical: false },
            PLayout::SplitV => Layout::Split { vertical: true },
            PLayout::Tabbed => Layout::Group { vertical: false },
            PLayout::Stacked => Layout::Group { vertical: true },
            _ => Layout::Other,
        }
    }
}

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum PType {
    Root,
    Output,
    Con,
    FloatingCon,
    Workspace,
    Dockarea,
}

#[derive(Debug, Deserialize)]
struct PRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl PRect {
    fn process(&self) -> Rect {
        Rect {
            pos: Vec2 {
                x: self.x,
                y: self.y,
            },
            dim: Vec2 {
                x: self.width,
                y: self.height,
            },
        }
    }
}

#[derive(Debug, Deserialize)]
struct PTree {
    id: u32,
    name: Option<String>,
    #[serde(rename = "type")]
    ctype: PType,
    layout: PLayout,
    rect: PRect,
    focused: bool,
    focus: Box<[u32]>,
    nodes: Box<[PTree]>,
    floating_nodes: Box<[PTree]>,
}

impl PTree {
    fn process(&self) -> Option<Tree> {
        if self.name.as_ref().map(|name| name.starts_with("__i3")) == Some(true) {
            return None;
        }

        let nodes: Box<[Tree]> = self.nodes.iter().flat_map(|n| n.process()).collect();
        let float_nodes: Box<[Tree]> = self
            .floating_nodes
            .iter()
            .flat_map(|n| n.process())
            .collect();

        let focus_id = self.focus.first();
        let simple_focus = if let Some(&id) = focus_id {
            nodes
                .iter()
                .enumerate()
                .find_map(|(idx, n)| if n.id == id { Some(idx) } else { None })
        } else {
            None
        };

        let float_focus = if let Some(&id) = focus_id {
            float_nodes.iter().enumerate().find_map(
                |(idx, n)| {
                    if n.id == id {
                        Some(idx)
                    } else {
                        None
                    }
                },
            )
        } else {
            None
        };

        let focus = Some(if simple_focus.is_some() { 0 } else { 1 });
        let rect = self.rect.process();

        let mut simple_tree = Tree {
            id: self.id,
            name: self.name.clone(),
            ctype: self.ctype,
            layout: self.layout.process(),
            rect,
            is_focused: self.focused,
            focus: simple_focus,
            nodes,
        };

        match self.ctype {
            PType::Root => {
                simple_tree.layout = Layout::Outputs;
                Some(simple_tree)
            }
            PType::Workspace => Some(Tree {
                id: self.id,
                ctype: self.ctype,
                name: self.name.clone(),
                layout: Layout::Other,
                rect,
                is_focused: self.focused,
                focus,
                nodes: Box::new([
                    simple_tree,
                    Tree {
                        ctype: self.ctype,
                        name: self.name.clone(),
                        id: self.id,
                        layout: Layout::Floats,
                        rect,
                        is_focused: false,
                        focus: float_focus,
                        nodes: float_nodes,
                    },
                ]),
            }),
            _ => Some(simple_tree),
        }
    }
}
