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
        let mut tree: Tree = serde_json::from_slice(input.stdout.as_slice())
            .expect("failed to parse container tree");
        tree.layout = Layout::Other; // ignore the topmost container

        if let Some(neighbor) = tree.neighbor(&targets) {
            let mut cmd = Command::new("swaymsg");
            cmd.arg(format!("[con_id={neighbor}] focus"));
            cmd.spawn()
                .and_then(|mut p| p.wait())
                .expect("failed to send focus command");
        }
    } else {
        let bin_name = &args[0];
        println!(
            "usage: {bin_name} (splith|splitv|tabbed|stacked) (forward|backward) (cycle|nocycle)"
        );
    }
}

// Command types
#[derive(Clone, Copy, PartialEq, Eq)]
struct Target {
    kind: Kind,
    backward: bool,
    vertical: bool,
    wrap: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Split,
    Group,
    Float,
    Output,
}

// Tree types
struct Tree {
    id: u32,
    layout: Layout,
    rect: Rect,
    is_focused: bool,
    focus: Option<usize>,
    nodes: Box<[Tree]>,
}

#[derive(Clone, Copy)]
enum Layout {
    Group { vertical: bool },
    Split { vertical: bool },
    Floats,
    Outputs,
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq)]
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

#[derive(Clone, Copy, PartialEq, Eq)]
struct Vec2 {
    x: i32,
    y: i32,
}

impl Tree {
    fn neighbor(&self, targets: &[Target]) -> Option<u32> {
        let mut t = self;
        let mut deepest_neighbor = None;
        while !t.is_focused {
            deepest_neighbor = t.local_neighbor(targets).or(deepest_neighbor);
            if let Some(new_t) = t.focus_next() {
                t = new_t;
            } else {
                break;
            }
        }
        Some(deepest_neighbor?.focus_all().id)
    }

    fn focus_next(&self) -> Option<&Tree> {
        self.nodes.get(self.focus?)
    }

    fn focus_all(&self) -> &Tree {
        let mut t = self;
        while let Some(idx) = t.focus {
            if !t.is_focused {
                break;
            }
            t = t.nodes.get(idx).expect("Focused child doesn't exist");
        }
        t
    }

    // Attempts to get a neighbor of focused child,
    // based on a list of targets.
    fn local_neighbor(&self, targets: &[Target]) -> Option<&Tree> {
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
                        let (a, b) = rearrange(n.rect, focused);
                        component(a.pos) + component(a.dim) < component(b.pos)
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
                            let (a, b) = rearrange(focused, n.rect);
                            component(a.pos) + component(a.dim) < component(b.pos)
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

fn parse_args(args: &[String]) -> Option<Box<[Target]>> {
    None
}
