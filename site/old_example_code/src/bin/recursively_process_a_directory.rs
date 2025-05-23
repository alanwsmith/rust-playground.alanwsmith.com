// use walkdir::{DirEntry, WalkDir};

// fn is_not_hidden(entry: &DirEntry) -> bool {
//     entry
//         .file_name()
//         .to_str()
//         .map(|s| entry.depth() == 0 || !s.starts_with("."))
//         .unwrap_or(false)
// }

// fn main() {
//     WalkDir::new(".")
//         .into_iter()
//         .filter_entry(|e| is_not_hidden(e))
//         .filter_map(|v| v.ok())
//         .for_each(|x| println!("{}", x.path().display()));
// }

// cargo add glob
// cargo add anyhow
use anyhow::Error;
use glob::glob;

fn main() -> Result<(), Error> {
    for entry in glob("**/*.rs")? {
        println!("{}", entry?.display());
    }

    Ok(())
}
