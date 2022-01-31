use serde::Deserialize;
use std::env;
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

struct Task {
    layouts: Box<[Layout]>,
    backward: bool,
    wrap: bool,
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
            if child.id == self.focus[0] {
                Some(idx)
            } else {
                None
            }
        })
    }

    /// Finds the next or previous node relative to the focused container,
    /// but only considering the [parent_layout] container type.
    fn find_neighbor(&self, task: &Task) -> Option<u32> {
        let mut t = self;
        let mut target_child = None;
        while let Some(focus_idx) = t.focus_idx() {
            if t.focused {
                break;
            }

            let num_children = t.nodes.len();
            if task.layouts.contains(&t.layout) {
                let branch_idx = focus_idx + num_children;
                let branch_idx = if task.backward { branch_idx - 1 } else { branch_idx + 1 };
                let branch_idx = if task.wrap {
                    Some(branch_idx % num_children)
                } else if branch_idx >= num_children && branch_idx < num_children * 2 {
                    Some(branch_idx - num_children)
                } else {
                    None
                };
                if let Some(branch_idx) = branch_idx {
                    target_child = Some(t.nodes.get(branch_idx).unwrap());
                }
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

fn parse_args(args: &[String]) -> Option<Task> {
    if args.len() < 4 {
        return None;
    }
    let backward = match args[1].as_str() {
        "prev" => Some(true),
        "next" => Some(false),
        _ => None,
    }?;
    let wrap = match args[2].as_str() {
        "wrap" => Some(true),
        "nowrap" => Some(false),
        _ => None,
    }?;
    let layouts: Box<[Layout]> = args[3..]
        .iter()
        .flat_map(|arg| match arg.as_str() {
            "splith" => Some(Layout::SplitH),
            "splitv" => Some(Layout::SplitV),
            "tabbed" => Some(Layout::Tabbed),
            "stacked" => Some(Layout::Stacked),
            _ => None,
        })
        .collect();

    Some(Task {
        layouts,
        backward,
        wrap,
    })
}

fn main() {
    let args: Box<[String]> = env::args().collect();
    if let Some(task) = parse_args(&args) {
        let mut get_tree = Command::new("swaymsg");
        get_tree.arg("-t").arg("get_tree");
        let input = get_tree
            .output()
            .expect("failed to retrieve container tree");
        let mut tree: Tree = serde_json::from_slice(input.stdout.as_slice())
            .expect("failed to parse container tree");
        tree.layout = Layout::Other; // ignore the topmost container

        if let Some(neighbor) = tree.find_neighbor(&task) {
            let mut cmd = Command::new("swaymsg");
            cmd.arg(format!("[con_id={neighbor}] focus"));
            cmd.spawn()
                .and_then(|mut p| p.wait())
                .expect("failed to send focus command");
        }
    } else {
        let bin_name = &args[0];
        println!("usage: {bin_name} (prev|next) (wrap|nowrap) (splith|splitv|tabbed|stacked)+");
    }
}
