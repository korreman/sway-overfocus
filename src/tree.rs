//! Tree types, parsing, and pre-processing.
use log::{debug, trace};
use std::mem;
use swayipc::{Node, NodeLayout, NodeType, Rect};

/// Closest point to `p` within the rectangle.
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

/// Generate a command that will focus the top node.
pub fn focus_command(node: &Node) -> Option<String> {
    let name = node.name.clone()?;
    let id = node.id;
    match node.node_type {
        NodeType::Root => None,
        NodeType::Output => Some(format!("focus output {name}")),
        NodeType::Workspace => Some(format!("workspace {name}")),
        _ => Some(format!("[con_id={id}] focus")),
    }
}

/// Reform the tree to prepare for neighbor searching
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
        // If this is the case, replace the children of the output those of the content node.
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
            // For any workspace with a fullscreen child, replace it with said child
            if let Some(mut fullscreen_node) = extract_fullscreen_child(workspace) {
                // If the node is global fullscreen, it replaces the entire tree
                if fullscreen_node.fullscreen_mode == Some(2) {
                    debug!(
                        "Node {} is global fullscreen, replaces entire tree",
                        fullscreen_node.id
                    );
                    return fullscreen_node;
                }
                // Preserve workspace ID, type, and name when replacing.
                // If the fullscreen node is a focus target,
                // it will be focused indirectly through the workspace name.
                trace!(
                    "Node {} is fullscreen, replaces workspace",
                    fullscreen_node.id
                );
                fullscreen_node.id = workspace.id;
                fullscreen_node.node_type = NodeType::Workspace;
                fullscreen_node.name = mem::take(&mut workspace.name);
                *workspace = fullscreen_node;
            }
        }
    }
    node
}

// TODO: Handle floats in functions below

/// Search the tree for a child that is fullscreen.
/// If found, the child is detached and returned.
/// Neighbors of the child are detached and dropped as collateral.
pub fn extract_fullscreen_child(node: &mut Node) -> Option<Node> {
    if node
        .nodes
        .iter()
        .any(|node| node.fullscreen_mode == Some(1) || node.fullscreen_mode == Some(2))
    {
        let nodes = mem::take(&mut node.nodes);
        let node: Node = nodes
            .into_iter()
            .find(|node| node.fullscreen_mode == Some(1) || node.fullscreen_mode == Some(2))
            .unwrap();
        Some(node)
    } else {
        node.nodes.iter_mut().find_map(extract_fullscreen_child)
    }
}

/// Compute the index (_not_ identifier) of the focused node in child array,
/// if any.
pub fn focus_idx(node: &Node) -> Option<usize> {
    node.nodes.iter().enumerate().find_map(|(idx, n)| {
        if n.id == *node.focus.first()? {
            Some(idx)
        } else {
            None
        }
    })
}

/// Return the focused child, if any.
pub fn focus_local(node: &Node) -> Option<&Node> {
    node.nodes.get(focus_idx(node)?)
}
