use serde::Deserialize;
use serde_json::Result;
use std::env;
use std::io;
use std::process::Command;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Layout {
    SplitH,
    SplitV,
    Stacked,
    Tabbed,
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct Tree {
    id: u32,
    focused: bool,
    layout: Layout,
    focus: Vec<u32>,
    nodes: Vec<Tree>,
}

impl Tree {
    fn focus_idx(&self) -> usize {
        let (focus_idx, _) = self
            .nodes
            .iter()
            .enumerate()
            .find(|(_, child)| child.id == self.focus[0])
            .unwrap();
        focus_idx
    }
}

#[derive(Debug, Clone, Copy)]
enum Direction {
    Forward,
    Backward,
}

fn main() {
    let input = io::stdin();
    let input = input.lock();
    let tree: Tree = serde_json::from_reader(input).unwrap();
    println!("{tree:?}");
    if let Some((layout, direction)) = parse_args() {
        if let Some(neighbor) = find_neighbor(&tree, layout, direction) {
            let mut cmd = Command::new("swaymsg");
            cmd.arg(format!("[con_id={neighbor}] focus"));
            cmd.spawn().unwrap().wait().unwrap();
        }
    } else {
        println!("usage: swaytab (splith|splitv|tabbed|stacked) (forward|backward)");
    }
}

fn parse_args() -> Option<(Layout, Direction)> {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        3 => {
            let layout = match args[1].as_str() {
                "splith" => Some(Layout::SplitH),
                "splitv" => Some(Layout::SplitV),
                "tabbed" => Some(Layout::Tabbed),
                "stacked" => Some(Layout::Stacked),
                _ => None,
            }?;
            let direction = match args[2].as_str() {
                "forward" => Some(Direction::Forward),
                "backward" => Some(Direction::Backward),
                _ => None,
            }?;
            Some((layout, direction))
        }
        _ => None,
    }
}

/// Finds the next or previous node relative to the focused container,
/// but only considering the [parent_layout] container type.
fn find_neighbor(mut t: &Tree, parent_layout: Layout, dir: Direction) -> Option<u32> {
    let mut target_child = None;
    loop {
        let num_children = t.nodes.len();
        if num_children == 0 || t.focused {
            break;
        }
        let focus_idx = t.focus_idx();
        if t.layout == parent_layout {
            match dir {
                Direction::Forward => target_child = Some(&t.nodes[focus_idx + 1 % num_children]),
                Direction::Backward => target_child = Some(&t.nodes[focus_idx - 1 % num_children]),
            }
        }
        t = &t.nodes[focus_idx];
    }
    target_child.map(|mut child| {
        while !child.nodes.is_empty() {
            child = &child.nodes[child.focus_idx()];
        }
        child.id
    })
}
