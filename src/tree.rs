//! Basic tree functions and pre-processing
use log::{debug, trace};
use std::mem;
use swayipc::{Node, NodeLayout, NodeType, Rect};

/// Closest point to `p` within `rect`.
pub fn closest_point(rect: &Rect, p: &Vec2) -> Vec2 {
    Vec2 {
        x: i32::clamp(p.x, rect.x, rect.x + rect.width - 1),
        y: i32::clamp(p.y, rect.y, rect.y + rect.height - 1),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vec2 {
    pub x: i32,
    pub y: i32,
}

/// Generate a command that will focus `node`.
pub fn focus_command(node: &Node) -> Option<String> {
    let name = node.name.clone();
    match node.node_type {
        NodeType::Root => None,
        NodeType::Output => Some(format!("focus output {}", name?)),
        NodeType::Workspace => Some(format!("workspace {}", name?)),
        _ => Some(format!("[con_id={}] focus", node.id)),
    }
}

/// Return the focused child, if any.
pub fn focus_local(node: &Node) -> Option<&Node> {
    let focus = *node.focus.first()?;
    node.nodes
        .iter()
        .chain(node.floating_nodes.iter())
        .find(|child| child.id == focus)
}

/// Compute the index (_not_ identifier) of the focused node in child array, if any.
/// Also returns the vector of children to index into (either regular nodes or floats).
pub fn focus_idx(node: &Node) -> Option<(usize, &Vec<Node>)> {
    let focus = *node.focus.first()?;
    for children in [&node.nodes, &node.floating_nodes] {
        for (index, child) in children.iter().enumerate() {
            if child.id == focus {
                return Some((index, children));
            }
        }
    }
    None
}

/// Reform the tree to prepare for neighbor searching
/// This mainly consists of collapsing i3 outputs with `content` subnodes
/// and workspaces with fullscreen descendants
pub fn preprocess(mut node: Node) -> Node {
    node.layout = NodeLayout::None;
    // Remove scratchpad and potential similar output nodes
    node.nodes
        .retain(|node| node.name.as_ref().map(|name| name.starts_with("__i3")) != Some(true));

    for output in node.nodes.iter_mut() {
        debug!(
            "Output '{}', ID {}",
            output.name.as_ref().unwrap_or(&"".to_string()),
            output.id,
        );

        // On i3, outputs contain a `content` subnode containing workspaces.
        // If this is the case, replace the children of the output with those of the `content` node.
        if let Some(content) = output
            .nodes
            .iter_mut()
            .find(|node| node.name.as_ref() == Some(&"content".to_string()))
        {
            trace!("Found 'content' subnode, collapsing");
            output.focus = mem::take(&mut content.focus);
            output.nodes = mem::take(&mut content.nodes);
        }

        // Reform workspaces
        for workspace in output.nodes.iter_mut() {
            debug!(
                "Workspace '{}', ID {}",
                workspace.name.as_ref().unwrap_or(&"".to_string()),
                workspace.id,
            );
            // Collapse nodes with fullscreen descendants
            if let Some(fullscreen_node) = extract_fullscreen_child(workspace) {
                debug!(
                    "Node {} has fullscreen mode {}",
                    fullscreen_node.id,
                    fullscreen_node.fullscreen_mode.unwrap()
                );
                // If the node is global fullscreen, it replaces the entire tree
                if fullscreen_node.fullscreen_mode == Some(2) {
                    trace!("Replacing entire tree");
                    return fullscreen_node;
                }
                // Otherwise, it replaces the workspace
                if output.focus.first() == Some(&workspace.id) {
                    // We may potentially have to change parent focus
                    output.focus = vec![fullscreen_node.id];
                }
                *workspace = fullscreen_node;
            }
        }
    }
    node
}

/// Search the tree for a fullscreen descendant.
/// If found, the descendant is detached and returned.
/// Neighbors of the descendant are detached and dropped as collateral.
pub fn extract_fullscreen_child(node: &mut Node) -> Option<Node> {
    let mut children = node.nodes.iter_mut().chain(node.floating_nodes.iter_mut());
    let pred = |child: &Node| child.fullscreen_mode == Some(1) || child.fullscreen_mode == Some(2);
    if children.any(|c| pred(c)) {
        let nodes = mem::take(&mut node.nodes);
        let floating_nodes = mem::take(&mut node.floating_nodes);
        let mut children = nodes.into_iter().chain(floating_nodes.into_iter());
        children.find(pred)
    } else {
        node.nodes.iter_mut().find_map(extract_fullscreen_child)
    }
}
