#![allow(warnings)]
use axum::{response::Html, routing::get, Router};
use notify::RecursiveMode;
use notify::Watcher;
use notify_debouncer_mini::new_debouncer;
use notify_debouncer_mini::DebounceEventResult;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::fs;
use std::fs::remove_file;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;
use tower_http::services::ServeDir;
use tower_livereload::LiveReloadLayer;
use tower_livereload::Reloader;
use walkdir::{DirEntry, WalkDir};
// use tracing::{debug, error, info, span, warn, Level};

#[derive(Debug)]
pub struct Site {
    pages: BTreeMap<String, Page>,
    pages_v2: BTreeMap<PathBuf, Page>,
    input_dir: PathBuf,
    output_dir: PathBuf,
    // the valid extensions are to prevent tmp files
    // that end in e.g. `~`` from triggering
    valid_extension: Vec<String>,
    home_page: PathBuf,
}

#[derive(Debug)]
pub struct Page {
    site_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let site = Site {
        pages: BTreeMap::new(),
        // needs to be full paths for now. the paths that 
        // come back from the watch are full regardless of
        // if this is set to relative path and I'm not sure
        // how to parse that out
        input_dir: PathBuf::from("/Users/alan/workshop/rust-playground.alanwsmith.com/site/web_server_with_hot_reload/_content"),
        output_dir: PathBuf::from("/Users/alan/workshop/rust-playground.alanwsmith.com/site/web_server_with_hot_reload/_site"),
        valid_extension: vec!["html".to_string(), "md".to_string(), "neo".to_string()],
        home_page: PathBuf::from("/Users/alan/workshop/rust-playground.alanwsmith.com/site/web_server_with_hot_reload/_site/index.html"),
    };
    // tokio::spawn(async {
    //     let _ = watch_files(site);
    // });
    let _ = run_web_server(site).await;
    Ok(())
}

async fn run_web_server(site: Site) -> Result<(), Box<dyn std::error::Error>> {
    println!("- Starting web server");
    // TODO: read the site path for this
    let livereload = LiveReloadLayer::new();
    let reloader = livereload.reloader();
    let app = Router::new()
        .nest_service("/", ServeDir::new(Path::new(&site.output_dir)))
        .layer(livereload);
    // let mut watcher = notify::recommended_watcher(move |_| reloader.reload())?;
    //    watcher.watch(Path::new(site_dir), notify::RecursiveMode::Recursive)?;
    // TODO: put the bind address in the site config
    tokio::spawn(async {
        let _ = watch_files(site, reloader);
    });
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3443").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn watch_files(mut site: Site, reloader: Reloader) -> notify::Result<()> {
    println!("- Loading initial files");
    let walker = WalkDir::new(&site.input_dir).into_iter();
    for entry in walker.filter_entry(|e| {
        match e.path().extension() {
            Some(ext) => {
                if site
                    .valid_extension
                    .contains(&ext.to_string_lossy().to_string())
                {
                    let mut site_path = PathBuf::from("/");
                    site_path.push(
                        e.path()
                            .strip_prefix(&site.input_dir)
                            .unwrap()
                            .to_path_buf(),
                    );
                    site.pages
                        .insert(e.path().to_string_lossy().to_string(), Page { site_path });
                }
            }
            None => (),
        }
        true
    }) {}
    println!("- Buiding initial site");
    build_full_site(&site, &reloader);
    // println!("- Buiding initila home page");
    // build_home_page(&site);
    println!("- Starting file watcher");
    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(100), tx)?;
    debouncer
        .watcher()
        .watch(&site.input_dir, RecursiveMode::Recursive)?;
    for result in rx {
        let mut update_paths: BTreeSet<PathBuf> = BTreeSet::new();
        match result {
            Ok(events) => events.iter().for_each(|event| {
                let mut add_it = false;
                match event.path.extension() {
                    Some(p) => {
                        if site
                            .valid_extension
                            .contains(&p.to_string_lossy().to_string())
                        {
                            add_it = true;
                        }
                        ()
                    }
                    None => (),
                }
                if add_it {
                    update_paths.insert(event.path.clone());
                }
                ()
            }),
            Err(_) => {}
        }
        update_paths.iter().for_each(|path| {
            let output_rel_path = &path.strip_prefix(&site.input_dir).unwrap();
            let mut output_path = site.output_dir.clone();
            output_path.push(&output_rel_path);
            if file_exists(&path) {
                if &output_path != &site.home_page {
                    // do work here to apply templates, etc...
                    build_page(path, &output_path);
                    // add the page to the list of site pages
                    // then build the home page
                    let mut site_path = PathBuf::from("/");
                    site_path.push(&path.strip_prefix(&site.input_dir).unwrap().to_path_buf());
                    site.pages
                        .insert(path.display().to_string(), Page { site_path });
                    build_home_page(&site);
                } else {
                    // the home page changed, rebuild it
                    build_home_page(&site);
                }
            } else {
                site.pages.remove(&path.display().to_string());
                // remove the page from the output directory if
                // it exists
                if file_exists(&output_path) {
                    let _ = remove_file(output_path);
                }
                build_home_page(&site);
            }
        });
    }
    Ok(())
}

pub fn build_home_page(site: &Site) {
    println!("Building home page");
    let mut output_string = r#"<!DOCTYPE html><html><body>
<h1>Home Page</h1><ul>"#
        .to_string();
    &site.pages.iter().for_each(|page| {
        output_string.push_str(
            format!(
                r#"<li><a href="{}">{}</a></li>"#,
                &page.1.site_path.display().to_string(),
                &page.1.site_path.display().to_string()
            )
            .as_str(),
        );
        ()
    });
    output_string.push_str("</ul></body></html>");
    fs::write(&site.home_page, output_string);
}

fn file_exists(path: &PathBuf) -> bool {
    match path.try_exists() {
        Ok(exists) => {
            if exists == true {
                true
            } else {
                false
            }
        }
        Err(_) => false,
    }
}

fn build_page(input: &PathBuf, output: &PathBuf) {
    fs::copy(input, output);
}

fn build_full_site(site: &Site, reloader: &Reloader) {
    site.pages.iter().for_each(|page| {
        // dbg!(page);
        ()
    });
    reloader.reload();
}

fn build_queue(site: &Site, queue: Vec<PathBuf>) {}

// Improvements:
//
// Clear files that were removed when the
// process wasn't running on the first run
// or maybe look at the entire directory
// each time and remove files that got
// removed?
//
// build full site on startup
