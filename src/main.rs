use std::env;
use std::fs;
use std::process::ExitCode;

use breevy_core::{parse_bvy, Expander};

#[cfg(feature = "gui")]
mod gui;

#[cfg(windows)]
mod hook;
#[cfg(target_os = "linux")]
mod hook_linux;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();

    #[cfg(feature = "gui")]
    if args.len() < 2 || args.get(1).map(String::as_str) == Some("--gui") {
        let path = args.get(2).cloned().unwrap_or_else(|| "abbreviations.bvy".to_string());
        return match gui::run(path) {
            Ok(_) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("gui error: {e}");
                ExitCode::FAILURE
            }
        };
    }

    if args.len() < 2 {
        eprintln!("usage: breevy-rs <abbreviations.bvy> [--list] [--match TEXT] [--trigger]");
        return ExitCode::from(2);
    }
    #[cfg(windows)]
    if args.iter().any(|a| a == "--hook") {
        hook::run(args.get(2).map(String::as_str).unwrap_or("abbreviations.bvy"));
    }
    #[cfg(target_os = "linux")]
    if args.iter().any(|a| a == "--hook") {
        hook_linux::run(args.get(2).map(String::as_str).unwrap_or("abbreviations.bvy"));
    }

    let path = &args[1];
    let input = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot read {path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    let snippets = parse_bvy(&input);
    let mut expander = Expander::new(snippets.clone());

    if let Some(pos) = args.iter().position(|a| a == "--export") {
        let Some(out) = args.get(pos + 1) else {
            eprintln!("--export needs a path");
            return ExitCode::from(2);
        };
        let xml = breevy_core::write_bvy(&snippets);
        if let Err(e) = fs::write(out, &xml) {
            eprintln!("cannot write {out}: {e}");
            return ExitCode::FAILURE;
        }
        println!("exported: {} -> {}", snippets.len(), out);
        return ExitCode::SUCCESS;
    }

    if args.iter().any(|a| a == "--list") {
        for s in &snippets {
            println!("{}\t{}\t{}", s.abbreviation, s.mode, s.label);
        }
        return ExitCode::SUCCESS;
    }

    if let Some(pos) = args.iter().position(|a| a == "--match") {
        let Some(text) = args.get(pos + 1) else {
            eprintln!("--match needs a value");
            return ExitCode::from(2);
        };
        for c in text.chars() {
            expander.feed(breevy_core::engine::Event::Character(c));
        }
        if args.iter().any(|a| a == "--trigger") {
            expander.feed(breevy_core::engine::Event::Trigger);
        }
        match expander.take_expansion() {
            Some(e) => {
                println!("match: {}\nremove: {}\nexpansion:\n{}", e.abbreviation, e.remove_chars, e.text);
                return ExitCode::SUCCESS;
            }
            None => {
                println!("no match");
                return ExitCode::FAILURE;
            }
        }
    }

    println!("snippets: {}", snippets.len());
    ExitCode::SUCCESS
}
