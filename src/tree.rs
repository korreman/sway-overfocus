//! Tree types, parsing, and pre-processing.
use serde::Deserialize;
use std::mem;

/// Parsed layout, immediately converted to [Layout].
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

/// Re-interpreted layout.
#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(from = "PLayout")]
pub enum Layout {
    /// Holds outputs.
    Root,
    /// Holds workspaces.
    Output,
    /// Holds floating containers as regular nodes.
    Floats,
    /// `hsplith` and `vsplit` container.
    Split {
        vertical: bool,
    },
    /// Horizontal groups are tabbed, vertical groups are stacking.
    Group {
        vertical: bool,
    },
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
pub enum ContainerType {
    Root,
    Output,
    Con,
    FloatingCon,
    Workspace,
    Dockarea,
}

/// Parsed rectangles, immediately converted to [Rect].
#[derive(Deserialize)]
struct PRect {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

/// Bounding boxes of containers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(from = "PRect")]
pub struct Rect {
    pub pos: Vec2,
    pub dim: Vec2,
}

impl From<PRect> for Rect {
    /// Conversion only changes the data layout.
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
    /// Closest point to `p` within the rectangle.
    pub fn closest_point(&self, p: Vec2) -> Vec2 {
        Vec2 {
            x: i32::clamp(p.x, self.pos.x, self.pos.x + self.dim.x - 1),
            y: i32::clamp(p.y, self.pos.y, self.pos.y + self.dim.y - 1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct Vec2 {
    pub x: i32,
    pub y: i32,
}

pub type ID = u64;

/// Container tree, representing output from `-t get_tree`.
#[derive(Debug, Deserialize, Clone)]
pub struct Tree {
    pub id: ID,
    pub name: Option<String>,
    /// Needed for generating appropiate focus commands.
    #[serde(rename = "type")]
    pub con_type: ContainerType,
    pub layout: Layout,
    pub rect: Rect,
    pub focused: bool,
    pub focus: Box<[u64]>,
    pub nodes: Vec<Tree>,
    /// Only workspaces should contain these, before preprocessing.
    pub floating_nodes: Box<[Tree]>,
    pub fullscreen_mode: u8,
}

impl Tree {
    /// Generate a command that will focus the top node.
    pub fn focus_command(&self) -> Option<String> {
        let name = self.name.clone()?;
        let id = self.id;
        match self.con_type {
            ContainerType::Root => None,
            ContainerType::Output => Some(format!("focus output {name}")),
            ContainerType::Workspace => Some(format!("workspace {name}")),
            _ => Some(format!("[con_id={id}] focus")),
        }
    }

    /// Reform the tree to prepare for traversal.
    /// 1. Top node is marked as `Root`.
    /// 2. Workspace floats are moved to "regular" nodes in a separate subtree.
    /// 3. Fullscreen children replace their entire workspace
    ///    (global fullscreen replaces the entire tree).
    pub fn reform(mut self) -> Tree {
        self.layout = Layout::Root;
        for output in self.nodes.iter_mut() {
            for workspace in output.nodes.iter_mut() {
                // Split regular nodes and floating nodes into two subtrees with a new parent.
                let focus = mem::take(&mut workspace.focus);
                let nodes = mem::take(&mut workspace.nodes);
                let floats = mem::take(&mut workspace.floating_nodes);

                // Delegate focus history out to subtrees for regular nodes and floats.
                let (focus_nodes, focus_floats): (Vec<u64>, Vec<u64>) = focus
                    .iter()
                    .partition(|id| nodes.iter().any(|n| n.id == **id));

                // Set focus of parent based on which new subtree contains latest focused child.
                // Subtrees are given IDs 0 and 1.
                // This is fine as the parent is assigned the layout `Other`,
                // and the subtrees can therefore never be selected as focus targets.
                workspace.focus =
                    Box::new(if focus.first() == focus_nodes.first() { [0, 1] } else { [1, 0] });

                // Subtrees inherit most properties from the original.
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
                    // Preserve workspace ID, type, and name when replacing.
                    // If the fullscreen node is a focus target,
                    // it will be focused indirectly through the workspace name.
                    fullscreen_node.id = workspace.id;
                    fullscreen_node.con_type = ContainerType::Workspace;
                    fullscreen_node.name = mem::take(&mut workspace.name);
                    *workspace = fullscreen_node;
                }
            }
        }
        self
    }

    /// Search the tree for a child that is fullscreen.
    /// If found, the child is detached and returned.
    /// Neighbors of the child are detached and dropped as collateral.
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

    /// Compute the index (_not_ identifier) of the focused node in child array,
    /// if any.
    pub fn focus_idx(&self) -> Option<usize> {
        self.nodes.iter().enumerate().find_map(|(idx, n)| {
            if n.id == *self.focus.first()? {
                Some(idx)
            } else {
                None
            }
        })
    }

    /// Return the focused child, if any.
    pub fn focus_local(&self) -> Option<&Tree> {
        self.nodes.get(self.focus_idx()?)
    }
}
