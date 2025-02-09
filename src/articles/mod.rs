use crate::db::article::Article;
use crate::db::DbPool;

use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;

mod plugins;
mod tests;
mod utils;

use crate::config;
use crate::renderer::html::{
    create_html_from_content_template, create_html_from_standalone_template,
};
use crate::renderer::pandoc::pandoc_mdwn_2_html;

use self::plugins::{draft, img, meta, series, specialpage, summary, tag, title};
use diesel::prelude::*;

#[derive(diesel::Insertable, Debug, Clone, PartialEq, Eq)]
#[diesel(table_name = crate::db::schema::articles)]
pub struct NewArticle {
    /// relative to $input
    pub src_file_name: String,
    /// relative to $input or flattened to single filename
    pub dst_file_name: String,

    /// override for the title (derived from filename by default)
    pub title: Option<String>,
    pub modification_date: Option<chrono::NaiveDateTime>,
    pub summary: Option<String>,
    //pub tags: Option<Vec<String>>,
    pub series: Option<String>,
    pub draft: Option<bool>,
    pub special_page: Option<bool>,
    pub timeline: Option<bool>,
    pub anchorjs: Option<bool>,
    pub tocify: Option<bool>,
    pub live_updates: Option<bool>,
}

pub fn scan_articles(pool: DbPool) {
    let cfg = config::Config::get();
    let input_path: PathBuf = cfg.input.clone();
    let mut cache: HashMap<String, String> = HashMap::new();

    let mut conn = pool
        .get()
        .expect("Failed to get a connection from the pool");

    let start_time = std::time::Instant::now();

    fn traverse_and_collect_articles(
        conn: &mut SqliteConnection,
        dir: &PathBuf,
        cache: &mut HashMap<String, String>,
    ) {
        if dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_dir() {
                            traverse_and_collect_articles(conn, &path, cache);
                        } else if let Some(ext) = path.extension() {
                            if ext == "mdwn" {
                                match parse_article(&path, cache) {
                                    Ok(article) => {
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

    traverse_and_collect_articles(&mut conn, &input_path, &mut cache);

    match crate::db::article::get_visible_articles(&mut conn) {
        Ok(articles) => {
            for article in articles {
                println!("Writing article {} to disk", article.clone().dst_file_name);
                write_article_to_disk(&article, &mut cache);
            }
        }
        Err(_) => { /* Handle errors if necessary */ }
    }

    // for (key, value) in &cache {
    //     println!("{}: {}...", key, &value[0..40]);
    // }

    let duration = start_time.elapsed();
    println!("Time taken to execute: {:?}", duration);
}

fn write_article_to_disk(article: &Article, cache: &mut HashMap<String, String>) {
    let cfg = config::Config::get();
    let output_path: PathBuf = cfg.output.clone();

    //let relative_path: String = String::new(); // FIXME move code from html.rs here

    match cache.get(&article.src_file_name) {
        Some(html) => {
            let content: String =
                create_html_from_content_template(article.clone(), html.clone()).unwrap();

            let standalone_html: String =
                create_html_from_standalone_template(article.clone(), content)
                    .unwrap();

            let mut output_filename = output_path.clone();
            output_filename.push(article.dst_file_name.clone());
            std::fs::write(output_filename, standalone_html).expect("Unable to write HTML file");
        }
        None => {
            println!("Error: path: {}", &article.src_file_name);
        }
    }
}

fn parse_article(
    article_path: &PathBuf,
    cache: &mut HashMap<String, String>,
) -> Result<NewArticle, Box<dyn Error>> {
    println!(
        "Parsing article {} from disk",
        article_path.clone().display()
    );

    let src_file_name = article_path.display().to_string();

    let mut new_article: NewArticle = NewArticle {
        src_file_name: src_file_name.clone(),
        dst_file_name: utils::create_dst_file_name(article_path),
        title: None,
        modification_date: None,
        summary: None,
        //tags: None,
        series: None,

        draft: None,
        special_page: None,
        timeline: None,

        anchorjs: Some(true),
        tocify: Some(true),
        live_updates: Some(true),
    };

    let article_mdwn_raw_string = std::fs::read_to_string(article_path).unwrap();
    match eval_plugins(&article_mdwn_raw_string, &mut new_article) {
        Ok(article_mdwn_refined_source) => {
            match pandoc_mdwn_2_html(article_mdwn_refined_source.clone()) {
                Ok(html) => {
                    cache.insert(src_file_name, html);
                    if new_article.title == None {
                        let title = utils::article_src_file_name_to_title(&article_path);
                        new_article.title = Some(title);
                    }
                    Ok(new_article)
                }
                Err(e) => {
                    println!(
                        "Error: No entry in cache for path: {}: {}",
                        src_file_name, e,
                    );
                    Err(e)
                }
            }
        }
        Err(e) => {
            println!("Error: Evaluating plugins on: {}: {}", src_file_name, e,);
            Err(e)
        }
    }
}

fn eval_plugins(
    article_mdwn_raw_string: &String,
    article: &mut NewArticle,
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

pub fn exec_plugin(input: &str, article: &mut NewArticle) -> Result<String, Box<dyn Error>> {
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
