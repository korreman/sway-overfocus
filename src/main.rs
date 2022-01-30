use serde::Deserialize;
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
    fn focus_idx(&self) -> Option<usize> {
        self.nodes.iter().enumerate().find_map(|(idx, child)| {
            if child.id == self.focus[0] { Some(idx) } else { None }
        })
    }

    /// Finds the next or previous node relative to the focused container,
    /// but only considering the [parent_layout] container type.
    fn find_neighbor(&self, task: &Task) -> Option<u32> {
        let mut t = self;
        let mut target_child = None;
        while let Some(focus_idx) = t.focus_idx() {
            if t.focused { break; }

            let num_children = t.nodes.len();
            if t.layout == task.layout {
                let branch_idx = focus_idx + num_children;
                let branch_idx = if task.backward { branch_idx - 1 } else { branch_idx + 1 };
                let branch_idx = if task.wrap {
                    branch_idx % num_children
                } else {
                    branch_idx.max(num_children).min(num_children * 2 - 1) - num_children
                };

                target_child = Some(t.nodes.get(branch_idx).unwrap());
            }
            t = &t.nodes[focus_idx];
        }
        target_child.map(|mut child| {
            while let Some(focus_idx) = child.focus_idx() {
                child = &child.nodes[focus_idx];
            }
            child.id
        })
    }
}

fn main() {
    let input = io::stdin();
    let input = input.lock();
    let tree: Tree = serde_json::from_reader(input).unwrap();
    if let Some(task) = parse_args() {
        if let Some(neighbor) = tree.find_neighbor(&task) {
            let mut cmd = Command::new("swaymsg");
            cmd.arg(format!("[con_id={neighbor}] focus"));
            cmd.spawn().unwrap().wait().unwrap();
        }
    } else {
        println!("usage: swaytab (splith|splitv|tabbed|stacked) (forward|backward) (wrap|nowrap)");
    }
}

struct Task {
    layout: Layout,
    backward: bool,
    wrap: bool,
}

fn parse_args() -> Option<Task> {
    let args: Vec<String> = env::args().collect();
    match args.len() {
        4 => {
            let layout = match args[1].as_str() {
                "splith" => Some(Layout::SplitH),
                "splitv" => Some(Layout::SplitV),
                "tabbed" => Some(Layout::Tabbed),
                "stacked" => Some(Layout::Stacked),
                _ => None,
            }?;
            let backward  = match args[2].as_str() {
                "backward" => Some(true),
                "forward" => Some(false),
                _ => None,
            }?;
            let wrap = match args[3].as_str() {
                "wrap" => Some(true),
                "nowrap" => Some(false),
                _ => None,
            }?;
            Some(Task {
                layout,
                backward,
                wrap,
            })
        }
        _ => None,
    }
}
