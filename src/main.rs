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
    Parse,
    Command,
    Message,
}

fn main() {
    match task() {
        Err(e) => {
            match e {
                FocusError::Args => eprint!("{}", include_str!("../usage.md")),
                FocusError::Retrieve => eprintln!("error: failed to acquire container tree"),
                FocusError::Parse => eprintln!("error: failed to parse container tree"),
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
    let tree: Tree = serde_json::from_slice(input.stdout.as_slice())
        .ok()
        .ok_or(FocusError::Parse)?;

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
    let (i3, args) = if args.len() > 2 && args[1] == "--i3" {
        Some((true, &args[2..]))
    } else if args.len() > 1 {
        Some((false, &args[1..]))
    } else {
        None
    }?;

    let targets = args.iter().map(|arg| {
        let split = arg.split_once('-')?;
        let kind = match split.0 {
            "split" => Some(Kind::Split),
            "group" => Some(Kind::Group),
            "float" => Some(Kind::Float),
            "workspace" => Some(Kind::Workspace),
            "output" => Some(Kind::Output),
            _ => None,
        }?;
        if let [dir, wrap] = split.1.as_bytes() {
            let (backward, vertical) = match dir {
                0x75 => Some((true, true)),
                0x64 => Some((false, true)),
                0x6c => Some((true, false)),
                0x72 => Some((false, false)),
                _ => None,
            }?;
            let edge_mode = match wrap {
                0x73 => Some(EdgeMode::Stop),
                0x77 => Some(EdgeMode::Wrap),
                0x74 => Some(EdgeMode::Traverse),
                0x69 => Some(EdgeMode::Inactive),
                _ => None,
            }?;
            Some(Target {
                kind,
                backward,
                vertical,
                edge_mode,
            })
        } else {
            None
        }
    });
    let targets: Option<Box<[Target]>> = targets.collect();
    Some((i3, targets?))
}
