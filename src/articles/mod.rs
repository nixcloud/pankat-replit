use crate::db::article::{
    get_prev_and_next_article, get_prev_and_next_article_for_series, ArticleNeighbours,
};
use crate::db::cache::{compute_hash, get_cache, set_cache};
use crate::db::DbPool;
use colored::Colorize;
use notify::EventKind;
use scopeguard::defer;

use regex::Regex;
use serde_json::json;
use std::error::Error;
use std::path::PathBuf;

mod plugins;
mod tests;
pub mod timeline;
mod utils;

use crate::config;
use crate::renderer::html::{
    create_html_from_content_template, create_html_from_standalone_template_by_article,
    create_index_from_most_recent_article_template,
};
use crate::renderer::pandoc::pandoc_mdwn_2_html;

use self::plugins::{draft, img, meta, series, specialpage, summary, tag, title};
use diesel::prelude::*;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ArticleWithTags {
    pub id: Option<i32>,
    pub src_file_name: String,
    pub dst_file_name: String,
    pub title: Option<String>,
    pub modification_date: Option<chrono::NaiveDateTime>,
    pub summary: Option<String>,
    pub tags: Option<Vec<String>>,
    pub series: Option<String>,
    pub draft: Option<bool>,
    pub special_page: Option<bool>,
    pub anchorjs: Option<bool>,
    pub tocify: Option<bool>,
    pub live_updates: Option<bool>,
}

#[derive(Debug, Clone, Eq, PartialEq, Insertable, AsChangeset)]
#[diesel(table_name = crate::db::schema::articles)]
pub struct NewArticle {
    pub src_file_name: String,
    pub dst_file_name: String,
    pub title: Option<String>,
    pub modification_date: Option<chrono::NaiveDateTime>,
    pub summary: Option<String>,
    pub series: Option<String>,
    pub draft: Option<bool>,
    pub special_page: Option<bool>,
    pub anchorjs: Option<bool>,
    pub tocify: Option<bool>,
    pub live_updates: Option<bool>,
}

impl From<ArticleWithTags> for NewArticle {
    fn from(article: ArticleWithTags) -> Self {
        NewArticle {
            src_file_name: article.src_file_name,
            dst_file_name: article.dst_file_name,
            title: article.title,
            modification_date: article.modification_date,
            summary: article.summary,
            series: article.series,
            draft: article.draft,
            special_page: article.special_page,
            anchorjs: article.anchorjs,
            tocify: article.tocify,
            live_updates: article.live_updates,
        }
    }
}

static PANKAT_FILE: &str = ".pankat_maintained_output_folder";

pub fn output_folder_check(output_folder: &PathBuf) -> Result<(), Box<dyn Error>> {
    let output_path_check_file = output_folder.join(PANKAT_FILE);

    if output_path_check_file.exists() {
        return Ok(());
    }

    if std::fs::read_dir(output_folder)?.next().is_none() {
        std::fs::File::create(output_path_check_file)?;
        Ok(())
    } else {
        Err(format!(
            "Output directory contains unexpected files or subdirectories but no '{}' file!",
            PANKAT_FILE
        )
        .into())
    }
}

/// returns a command: json string which is sent to via WS to the client or an error
pub fn file_monitor_articles_change(
    conn: &mut SqliteConnection,
    event: &crate::file_monitor::PankatFileMonitorEvent,
) -> Result<String, String> {
    println!("-----------> file_monitor_articles_change begin");
    defer! {
        println!("-----------< file_monitor_articles_change end");
    }

    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            println!(
                "📝 created / ✏️ modified called on {}",
                event.path.display()
            );
            match parse_article(conn, &event.path) {
                Ok(article) => {
                    //println!("Parsed article: {:#?}", article);
                    let reply = crate::db::article::set(conn, &article);
                    match reply {
                        Ok(db_reply) => {
                            write_article_to_disk(conn, &db_reply.article);
                            if db_reply.most_recent_article_change.is_some() {
                                update_most_recent_article(conn);
                            }
                            // FIXME: timeline? change on: nav, summary, date, title, tags, series
                            //let _ = crate::articles::timeline::update_timeline(&articles);

                            if Some(true) == db_reply.article.draft {
                                let cfg = config::Config::get();
                                let output_path: PathBuf = cfg.output.clone();
                                let mut output_filename = output_path.clone();
                                output_filename.push(article.dst_file_name.clone());
                                match std::fs::remove_file(output_filename.clone()) {
                                    Ok(_) => {
                                        let target: String =
                                            format!("/draft?{}", output_filename.display());
                                        return Ok(json!({ "redirect": target, }).to_string());
                                    }
                                    Err(e) => {
                                        println!("Error removing file: {}", e);
                                    }
                                }
                            };

                            match crate::db::cache::get_cache(conn, article.src_file_name.clone()) {
                                Some(cache_entry) => {
                                    let html: String = create_nav_content_template(
                                        conn,
                                        &db_reply.article,
                                        cache_entry.html,
                                    );
                                    Ok(json!({ "update": html, }).to_string())
                                }
                                None => Err("Error loading cache for Article".to_string()),
                            }
                        }
                        Err(e) => {
                            let error_message = format!("Error: {:?}", e);
                            println!("{}", error_message);
                            Err(error_message)
                        }
                    }
                }
                Err(e) => {
                    println!("Error: {:?}", e);
                    Err(format!("Error: {:?}", e))
                }
            }
        }
        EventKind::Remove(_) => {
            println!("🗑️ removed called on {}", event.path.display());
            let res =
                crate::db::article::del_by_src_file_name(conn, event.path.display().to_string());
            match res {
                // FIXME implement this
                Ok(_) => Err("FIXME: implement this".to_string()),
                Err(_) => Err("FIXME".to_string()),
            }
        }
        _ => Err("file_monitor_articles_change: Unknown event type".to_string()),
    }
}

pub fn collect_garbage(pool: &DbPool) {
    let cfg = config::Config::get();
    let input_path: PathBuf = cfg.input.clone();
    let output_path: PathBuf = cfg.output.clone();

    let mut conn = pool
        .get()
        .expect("Failed to get a connection from the pool");

    match crate::db::article::get_all_articles(&mut conn) {
        Ok(articles) => {
            println!("====== Running GC on 'articles table' ======");
            for article in articles.clone() {
                let path = input_path.join(article.src_file_name);
                if !path.exists() {
                    println!("Removing garbage 'article table' entry: {:?}", path);
                    let _ = crate::db::article::del_by_id(&mut conn, article.id.unwrap());
                }
            }
            println!("====== Running GC on 'output' directory ======");
            match output_folder_check(&output_path) {
                Ok(_) => {
                    let lookup_articles_set: std::collections::HashSet<String> = articles
                        .iter()
                        .map(|article| article.dst_file_name.clone())
                        .collect();
                    for entry in std::fs::read_dir(output_path.clone()).unwrap() {
                        let entry = entry.unwrap();
                        let relative_entry = entry
                            .path()
                            .strip_prefix(&output_path)
                            .unwrap()
                            .to_path_buf();
                        let relative_entry_string: String = relative_entry.display().to_string();
                        if relative_entry_string == PANKAT_FILE {
                            continue;
                        }
                        if entry.file_type().unwrap().is_dir() {
                            let w = "WARNING: ignoring path, needs to be implemented!".red();
                            println!("{w}");
                            continue;
                        }
                        if !lookup_articles_set.contains(relative_entry_string.as_str()) {
                            println!("Removing garbage 'output' entry: {:?}", relative_entry);
                            std::fs::remove_file(entry.path()).unwrap();
                        }
                    }
                }
                Err(e) => {
                    println!(
                        "Warning! Not doing any GC of output dir '{}': {}",
                        output_path.display(),
                        e
                    );
                }
            }
        }
        Err(_) => {}
    };

    match crate::db::cache::get_cache_src_file_names(&mut conn) {
        Ok(entries) => {
            println!("====== Running GC on 'cache table' ======");
            for (id, path) in entries {
                let path = input_path.join(path);
                if !path.exists() {
                    println!("Removing garbage 'cache table' entry: {:?}", path);
                    let _ = crate::db::cache::del_cache_by_id(&mut conn, id.unwrap());
                }
            }
        }
        Err(_) => {}
    };
}

pub fn scan_articles(pool: &DbPool) {
    let cfg = config::Config::get();
    let input_path: PathBuf = cfg.input.clone();

    let mut conn = pool
        .get()
        .expect("Failed to get a connection from the pool");

    let start_time = std::time::Instant::now();

    println!("====== Parsing input for mdwn documents ======");

    fn traverse_and_collect_articles(
        conn: &mut SqliteConnection,
        dir: &PathBuf,
        input_path: &PathBuf,
    ) {
        if dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        //println!("{}", path.clone().display());
                        if path.is_dir() {
                            traverse_and_collect_articles(conn, &path, &input_path);
                        } else if let Some(ext) = path.extension() {
                            if ext == "mdwn" {
                                let article_path: PathBuf =
                                    path.strip_prefix(input_path).unwrap().to_path_buf();
                                match parse_article(conn, &article_path) {
                                    Ok(article) => {
                                        //println!("Parsed article: {:#?}", article);
                                        let _ = crate::db::article::set(conn, &article);
                                    }
                                    Err(_) => { /* Handle errors if necessary */ }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    traverse_and_collect_articles(&mut conn, &input_path, &input_path);

    let duration = start_time.elapsed();
    println!("Time to scan input for articles: {:?}", duration);
}

pub fn build_articles(pool: &DbPool) {
    let mut conn = pool
        .get()
        .expect("Failed to get a connection from the pool");

    let start_time = std::time::Instant::now();

    match crate::db::article::get_visible_articles(&mut conn) {
        Ok(articles) => {
            let _ = crate::articles::timeline::update_timeline(&articles);

            for article in articles {
                let article_id = article.id.unwrap();
                println!(
                    "Writing article {} (id: {}) to disk",
                    article.clone().dst_file_name,
                    article_id
                );
                write_article_to_disk(&mut conn, &article);
            }
        }
        Err(_) => { /* Handle errors if necessary */ }
    }

    update_special_pages(&mut conn);
    update_most_recent_article(&mut conn);

    let duration = start_time.elapsed();
    println!("Time to build articles: {:?}", duration);
}

pub fn update_special_pages(conn: &mut SqliteConnection) {
    match crate::db::article::get_special_pages(conn) {
        Ok(special_pages) => {
            for article in special_pages {
                let article_id = article.id.unwrap();
                println!(
                    "Writing special_page article {} (id: {}) to disk",
                    article.clone().dst_file_name,
                    article_id
                );
                write_article_to_disk(conn, &article);
            }
        }
        Err(_) => { /* Handle errors if necessary */ }
    }
}

pub fn update_most_recent_article(conn: &mut SqliteConnection) {
    match crate::db::article::get_most_recent_article(conn) {
        Ok(article_option) => match article_option {
            Some(article) => {
                match create_index_from_most_recent_article_template(article.dst_file_name) {
                    Ok(html) => {
                        let cfg = config::Config::get();
                        let output_path: PathBuf = cfg.output.clone();
                        let mut output_filename = output_path.clone();
                        output_filename.push("index.html");
                        write_to_disk(&html, &output_filename);
                    }
                    Err(e) => {
                        println!("Error: {}", e);
                    }
                }
            }
            None => {}
        },
        Err(_) => {}
    };
}

fn create_nav_content_template(
    conn: &mut SqliteConnection,
    article: &ArticleWithTags,
    html: String,
) -> String {
    let article_id = article.id.unwrap();

    let article_neighbours: ArticleNeighbours = match get_prev_and_next_article(conn, article_id) {
        Ok(neighbours) => neighbours,
        Err(_) => ArticleNeighbours::new(),
    };
    let article_series_neighbours: ArticleNeighbours =
        match get_prev_and_next_article_for_series(conn, article_id) {
            Ok(neighbours) => neighbours,
            Err(_) => ArticleNeighbours::new(),
        };

    let content: String = create_html_from_content_template(
        article.clone(),
        html,
        article_neighbours,
        article_series_neighbours,
    )
    .unwrap();
    content
}

fn write_article_to_disk(conn: &mut SqliteConnection, article: &ArticleWithTags) {
    let cfg = config::Config::get();
    let output_path: PathBuf = cfg.output.clone();

    match get_cache(conn, article.src_file_name.clone()) {
        Some(cache_entry) => {
            let content: String = create_nav_content_template(conn, article, cache_entry.html);
            let standalone_html: String =
                create_html_from_standalone_template_by_article(article.clone(), content).unwrap();

            let mut output_filename = output_path.clone();
            output_filename.push(article.dst_file_name.clone());
            write_to_disk(&standalone_html, &output_filename)
        }
        None => {
            println!(
                "Error retrieving cache for path: {}",
                &article.src_file_name
            );
        }
    }
}

pub fn write_to_disk(content: &String, filepath: &PathBuf) {
    if let Some(parent) = filepath.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            eprintln!("Failed to create directory {:?}: {}", parent, e);
        }
    }
    std::fs::write(filepath, content.as_str()).expect("Unable to write HTML file");
}

fn parse_article(
    conn: &mut SqliteConnection,
    article_path: &PathBuf,
) -> Result<ArticleWithTags, Box<dyn Error>> {
    let src_file_name_string = article_path.display().to_string();
    let cfg = config::Config::get();
    let input_path: PathBuf = cfg.input.clone();

    println!(
        "Parsing article {} from disk",
        article_path.clone().display()
    );

    let mut new_article: ArticleWithTags = ArticleWithTags {
        id: None,
        src_file_name: src_file_name_string.clone(),
        dst_file_name: utils::create_dst_file_name(&article_path, cfg.flat),
        title: None,
        modification_date: None,
        summary: None,
        tags: None,
        series: None,
        draft: None,
        special_page: None,
        anchorjs: Some(true),
        tocify: Some(true),
        live_updates: Some(true),
    };

    let file_path: PathBuf = input_path.join(article_path);
    let article_mdwn_raw_string = std::fs::read_to_string(file_path).unwrap();
    match eval_plugins(&article_mdwn_raw_string, &mut new_article) {
        Ok(article_mdwn_refined_source) => {
            if new_article.special_page == Some(true) {
                new_article.tocify = None;
            }
            let hash: String = compute_hash(article_mdwn_refined_source.clone());
            // println!(
            //     "src_file_name_string.clone(): {}",
            //     src_file_name_string.clone()
            // );
            let renew_cache: bool = match get_cache(conn, src_file_name_string.clone()) {
                Some(cache_entry) => {
                    //println!("Cache_entry.hash: {}, hash: {}", cache_entry.hash, hash);
                    if cache_entry.hash == hash {
                        false
                    } else {
                        true
                    }
                }
                None => true,
            };
            if renew_cache {
                //println!(" ... cache outdated, regenerating");
                match pandoc_mdwn_2_html(
                    article_mdwn_refined_source.clone(),
                    new_article.anchorjs.unwrap_or(false),
                ) {
                    Ok(html) => {
                        match set_cache(conn, src_file_name_string.clone(), html.clone(), hash) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Error udpating cache: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        println!(
                            "Error: No entry in cache for path: {}: {}",
                            src_file_name_string, e,
                        );
                        return Err(e);
                    }
                };
            } else {
                println!(" ... skipping call to pandoc, already in cache");
            };
            if new_article.title == None {
                let title = utils::article_src_file_name_to_title(&article_path);
                new_article.title = Some(title);
            }
            Ok(new_article)
        }
        Err(e) => {
            println!(
                "Error: Evaluating plugins on: {}: {}",
                src_file_name_string, e,
            );
            Err(e)
        }
    }
}

fn eval_plugins(
    article_mdwn_raw_string: &String,
    article: &mut ArticleWithTags,
) -> Result<String, Box<dyn Error>> {
    let re = Regex::new(r"\[\[\!(.*?)\]\]").unwrap();

    let mut last = 0;
    let mut res: String = String::new();
    for mat in re.find_iter(&article_mdwn_raw_string) {
        let start = mat.start();
        let end = mat.end();

        if start > last {
            res += &article_mdwn_raw_string[last..start];
            last = start;
        }

        match exec_plugin(&article_mdwn_raw_string[start..end], article) {
            Ok(result) => {
                res.push_str(&result);
            }
            Err(e) => {
                res += &article_mdwn_raw_string[start..end];
                match utils::position_to_line_and_col_number(&article_mdwn_raw_string, start) {
                    Ok((line, col)) => {
                        println!(
                            "Error: call_plugin (at {}:{}:{}) returned error: {e}",
                            article.src_file_name, line, col
                        );
                    }
                    Err(_) => {
                        println!(
                            "Error: call_plugin (at {}:unknown position) returned error: {e}",
                            article.src_file_name
                        )
                    }
                }
            }
        }
        if end <= article_mdwn_raw_string.len() {
            res += &article_mdwn_raw_string[last..start];
            last = end;
        } else {
            return Err(
                "Error: The specified length to extract is beyond the string's bounds.".into(),
            );
        }
    }
    if last <= article_mdwn_raw_string.len() {
        let t = &article_mdwn_raw_string[last..];
        res += t;
    }
    Ok(res)
}

pub fn exec_plugin(input: &str, article: &mut ArticleWithTags) -> Result<String, Box<dyn Error>> {
    let pattern = r#"\[\[!([\w]+)(?:\s+(.*))?\]\]"#;
    let re = Regex::new(pattern).unwrap();

    if let Some(captures) = re.captures(input) {
        let name: &str = captures.get(1).unwrap().as_str();
        let argument = captures.get(2).map_or("", |m| m.as_str()).trim();

        match name.to_lowercase().as_str() {
            "title" => title::title(argument, article),
            "specialpage" => specialpage::specialpage(argument, article),
            "draft" => draft::draft(argument, article),
            "meta" => meta::meta(argument, article),
            "series" => series::series(argument, article),
            "tag" => tag::tag(argument, article),
            "img" => img::img(argument, article),
            "summary" => summary::summary(argument, article),
            _ => Err(format!("Plugin '{}' is not supported", name).into()),
        }
    } else {
        Err("Plugin couldn't be decoded".into())
    }
}
