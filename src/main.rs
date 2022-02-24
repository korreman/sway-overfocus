use log::info;
use std::env;
use swayipc::Connection;

mod algorithm;
use algorithm::{EdgeMode, Kind, Target};
mod tree;

#[derive(Debug)]
enum FocusError {
    Args,
    Command,
    SwayIPC(swayipc::Error),
}

fn main() {
    match task() {
        Err(e) => {
            match e {
                FocusError::Args => eprint!("{}", include_str!("../usage.md")),
                FocusError::Command => eprintln!("error: no valid focus command"),
                FocusError::SwayIPC(e) => eprintln!("swayipc error: {e}"),
            };
            std::process::exit(1);
        }
        Ok(()) => (),
    }
}

fn task() -> Result<(), FocusError> {
    env_logger::init();

    info!("Parsing arguments");
    let args: Box<[String]> = env::args().collect();
    let targets = parse_args(&args).ok_or(FocusError::Args)?;

    info!("Starting connection");
    let mut c = Connection::new().map_err(FocusError::SwayIPC)?;

    info!("Retrieving tree");
    let tree = c.get_tree().map_err(FocusError::SwayIPC)?;

    info!("Pre-processing tree");
    let tree = tree::preprocess(tree);

    info!("Searching for neighbor");
    let neighbor = algorithm::neighbor(&tree, &targets);

    if let Some(neighbor) = neighbor {
        let focus_cmd = tree::focus_command(neighbor).ok_or(FocusError::Command)?;
        info!("Running focus command: '{focus_cmd}'");
        c.run_command(focus_cmd).map_err(FocusError::SwayIPC)?;
    } else {
        info!("No neighbor found");
    }
    Ok(())
}

fn parse_args(args: &[String]) -> Option<Box<[Target]>> {
    if args.len() < 2 {
        return None;
    }

    args[1..]
        .iter()
        .map(|arg| {
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
        })
        .collect()
}
