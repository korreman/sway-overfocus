use serde::Deserialize;
use std::mem;

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

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
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
    pub fn closest_point(&self, p: Vec2) -> Vec2 {
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
    pub nodes: Vec<Tree>,
    pub floating_nodes: Box<[Tree]>,
    pub fullscreen_mode: u8,
}

impl Tree {
    pub fn reform(mut self) -> Tree {
        self.layout = Layout::Root;
        for output in self.nodes.iter_mut() {
            for workspace in output.nodes.iter_mut() {
                // Make into a parent that holds regular nodes and floating nodes in two subtrees.
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
                nodes_node.fullscreen_mode = 0;

                let mut floats_node = workspace.clone();
                floats_node.id = 1;
                floats_node.nodes = floats.to_vec();
                floats_node.focus = focus_floats.into_boxed_slice();
                floats_node.layout = Layout::Floats;
                floats_node.fullscreen_mode = 0;

                workspace.nodes = vec![nodes_node, floats_node];
                workspace.layout = Layout::Other;

                // For any workspace with a fullscreen child, replace it with said child.
                if let Some(mut fullscreen_node) = workspace.extract_fullscreen_child() {
                    // If the node is global fullscreen, it replaces the entire tree.
                    if fullscreen_node.fullscreen_mode == 2 {
                        return fullscreen_node;
                    }
                    // Preserve ID, type, and name when replacing.
                    // If the fullscreened node needs to be focused,
                    // it will be so indirectly through the workspace.
                    fullscreen_node.id = workspace.id;
                    fullscreen_node.ctype = Type::Workspace;
                    fullscreen_node.name = mem::take(&mut workspace.name);
                    *workspace = fullscreen_node;
                }
            }
        }
        self
    }

    // Search for a child that is fullscreen.
    // Also return whether the fullscreen mode is global.
    pub fn extract_fullscreen_child(&mut self) -> Option<Tree> {
        if self.nodes.iter().any(|node| node.fullscreen_mode != 0) {
            let nodes = mem::take(&mut self.nodes);
            let node: Tree = nodes
                .into_iter()
                .find(|node| node.fullscreen_mode != 0)
                .unwrap();
            Some(node)
        } else {
            self.nodes
                .iter_mut()
                .find_map(|n| n.extract_fullscreen_child())
        }
    }

    pub fn focus_command(&self) -> Option<String> {
        let name = self.name.clone()?;
        let id = self.id;
        match self.ctype {
            Type::Root => None,
            Type::Output => Some(format!("focus output {name}")),
            Type::Workspace => Some(format!("workspace {name}")),
            _ => Some(format!("[con_id={id}] focus")),
        }
    }

    pub fn focus_idx(&self) -> Option<usize> {
        self.nodes.iter().enumerate().find_map(|(idx, n)| {
            if n.id == *self.focus.first()? {
                Some(idx)
            } else {
                None
            }
        })
    }

    pub fn focus_local(&self) -> Option<&Tree> {
        self.nodes.get(self.focus_idx()?)
    }
}
