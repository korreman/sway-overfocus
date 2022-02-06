// Command types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub kind: Kind,
    pub backward: bool,
    pub vertical: bool,
    pub wrap: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Split,
    Group,
    Float,
    Output,
}

#[derive(Debug, Clone, Copy)]
pub enum CType {
    Root,
    Output,
    Con,
    FloatingCon,
    Workspace,
    Dockarea,
}

#[derive(Debug, Clone, Copy)]
pub enum Layout {
    Group { vertical: bool },
    Split { vertical: bool },
    Floats,
    Outputs,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub pos: Vec2,
    pub dim: Vec2,
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
pub struct Vec2 {
    pub x: i32,
    pub y: i32,
}

// Tree types
#[derive(Debug, Clone)]
pub struct Tree {
    pub id: u32,
    pub ctype: CType,
    pub name: Option<String>,
    pub layout: Layout,
    pub rect: Rect,
    pub is_focused: bool,
    pub focus: Option<usize>,
    pub nodes: Box<[Tree]>,
}

impl Tree {
    pub fn focus_command(&self) -> Option<String> {
        let name = self.name.clone()?;
        let id = self.id;
        let cmd = match self.ctype {
            CType::Root => None,
            CType::Output => Some(format!("focus output {name}")),
            CType::Workspace => Some(format!("workspace {name}")),
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

    pub fn neighbor(&self, targets: &[Target]) -> Option<&Tree> {
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
