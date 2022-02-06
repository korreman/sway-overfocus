use super::tree::{CType, Layout, Rect, Tree, Vec2};
use serde::Deserialize;

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
pub enum PType {
    Root,
    Output,
    Con,
    FloatingCon,
    Workspace,
    Dockarea,
}

impl PType {
    fn process(&self) -> CType {
        match self {
            PType::Root => CType::Root,
            PType::Output => CType::Output,
            PType::Con => CType::Con,
            PType::FloatingCon => CType::FloatingCon,
            PType::Workspace => CType::Workspace,
            PType::Dockarea => CType::Dockarea,
        }
    }
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
pub struct PTree {
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
    pub fn process(&self) -> Option<Tree> {
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
            ctype: self.ctype.process(),
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
                ctype: self.ctype.process(),
                name: self.name.clone(),
                layout: Layout::Other,
                rect,
                is_focused: self.focused,
                focus,
                nodes: Box::new([
                    simple_tree,
                    Tree {
                        ctype: self.ctype.process(),
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
