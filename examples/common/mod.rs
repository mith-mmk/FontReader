#![allow(dead_code)]

use std::env;
use std::path::PathBuf;

pub fn default_font_path() -> String {
    #[cfg(target_os = "windows")]
    {
        let windir = env::var("windir").unwrap();
        format!("{}\\fonts\\YuGothM.ttc", windir)
    }

    #[cfg(target_os = "macos")]
    {
        let home = env::var("HOME").unwrap();
        format!("{}/Library/Fonts/ヒラギノ角ゴシック W4.ttc", home)
    }

    #[cfg(target_os = "linux")]
    {
        "/usr/share/fonts".to_string()
    }
}

pub fn default_font_folder() -> String {
    #[cfg(target_os = "windows")]
    {
        let windir = env::var("windir").unwrap();
        format!("{}\\fonts\\", windir)
    }

    #[cfg(target_os = "macos")]
    {
        "/System/Library/Fonts/".to_string()
    }

    #[cfg(target_os = "linux")]
    {
        "/usr/share/fonts".to_string()
    }
}

pub fn value_for(args: &[String], names: &[&str]) -> Option<String> {
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        for name in names {
            if arg == name {
                return iter.next().cloned();
            }
            if let Some(rest) = arg.strip_prefix(name) {
                if let Some(value) = rest.strip_prefix('=') {
                    return Some(value.to_string());
                }
            }
        }
    }
    None
}

pub fn positional_value(args: &[String]) -> Option<String> {
    args.iter()
        .skip(1)
        .find(|arg| !arg.starts_with('-'))
        .cloned()
}

pub fn font_path(args: &[String]) -> PathBuf {
    value_for(args, &["-f", "--font"])
        .or_else(|| positional_value(args))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default_font_path()))
}

pub fn font_folder(args: &[String]) -> PathBuf {
    value_for(args, &["-d", "--dir", "--folder"])
        .or_else(|| positional_value(args))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default_font_folder()))
}

pub fn output_path(args: &[String], default: &str) -> PathBuf {
    value_for(args, &["-o", "--output"])
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default))
}

pub fn font_index(args: &[String], default: usize) -> usize {
    value_for(args, &["-i", "--index"])
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
}

pub fn text_content(args: &[String], default_file: &str) -> Result<String, std::io::Error> {
    if let Some(text) = value_for(args, &["-s", "--string"]) {
        return Ok(text);
    }

    let text_file = value_for(args, &["-t", "--text-file"])
        .unwrap_or_else(|| default_file.to_string());
    std::fs::read_to_string(text_file)
}
