use std::env;
use std::process::Command;

mod algorithm;
use algorithm::{EdgeMode, Kind, Target};
mod tree;
use tree::Tree;

#[derive(Debug)]
enum FocusError {
    Args,
    Retrieve,
    Parse(serde_json::Error),
    Command,
    Message,
}

fn main() {
    match task() {
        Err(e) => {
            match e {
                FocusError::Args => eprint!("{}", include_str!("../usage.md")),
                FocusError::Retrieve => eprintln!("error: failed to acquire container tree"),
                FocusError::Parse(e) => eprintln!("error: failed to parse container tree\n{e}"),
                FocusError::Command => eprintln!("error: no valid focus command"),
                FocusError::Message => eprintln!("error: failed to message WM"),
            };
            std::process::exit(1);
        }
        Ok(()) => (),
    }
}

fn task() -> Result<(), FocusError> {
    // Parse arguments into config and targets
    let args: Box<[String]> = env::args().collect();
    let (i3, targets) = parse_args(&args).ok_or(FocusError::Args)?;

    // Retrieve tree
    let mut get_tree = Command::new("swaymsg");
    get_tree.arg("-t").arg("get_tree");
    let input = get_tree.output().ok().ok_or(FocusError::Retrieve)?;

    // Parsing
    let tree: Tree = serde_json::from_slice(input.stdout.as_slice()).map_err(FocusError::Parse)?;

    // Pre-process
    let tree = tree.reform();

    // Look for neighbor using targets
    if let Some(neighbor) = algorithm::neighbor(&tree, &targets) {
        // Run focus command for found neighbor
        let mut cmd = Command::new(if i3 { "i3-msg" } else { "swaymsg" });
        let focus_cmd = neighbor.focus_command().ok_or(FocusError::Command)?;
        cmd.arg(focus_cmd);
        cmd.spawn()
            .and_then(|mut p| p.wait())
            .ok()
            .ok_or(FocusError::Message)?;
    } else {
        println!("no neighbor to focus");
    }
    Ok(())
}

fn parse_args(args: &[String]) -> Option<(bool, Box<[Target]>)> {
    // Check argument count and `--i3` flag
    let (i3, args) = if args.len() > 2 && args[1] == "--i3" {
        (true, &args[2..])
    } else if args.len() > 1 {
        (false, &args[1..])
    } else {
        return None;
    };

    // All subsequent arguments are layout targets,
    // so we can map parsing to the remaining slice.
    let targets: Option<Box<[Target]>> = args.iter().map(|arg| {
        let (target_name, mode_chars) = arg.split_once('-')?;
        let kind = match target_name {
            "split" => Some(Kind::Split),
            "group" => Some(Kind::Group),
            "float" => Some(Kind::Float),
            "workspace" => Some(Kind::Workspace),
            "output" => Some(Kind::Output),
            _ => None,
        }?;
        let mut mode_chars = mode_chars.chars();
        let (backward, vertical) = match mode_chars.next()? {
            'r' => Some((false, false)),
            'l' => Some((true, false)),
            'd' => Some((false, true)),
            'u' => Some((true, true)),
            _ => None,
        }?;
        let edge_mode = match mode_chars.next()? {
            's' => Some(EdgeMode::Stop),
            'w' => Some(EdgeMode::Wrap),
            't' => Some(EdgeMode::Traverse),
            'i' => Some(EdgeMode::Inactive),
            _ => None,
        }?;
        Some(Target {
            kind,
            backward,
            vertical,
            edge_mode,
        })
    }).collect();

    Some((i3, targets?))
}
