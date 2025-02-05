use chrono::NaiveDateTime;
use chrono::TimeZone;
use chrono::Utc;
use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::config;
use crate::renderer::html::{
    create_html_from_content_template, create_html_from_standalone_template,
};
use crate::renderer::pandoc::render_file;

#[derive(Debug, Clone)]
pub struct Article {
    /// relative to $input
    pub src_file_name: PathBuf,
    /// relative to $input or flattened to single filename
    pub dst_file_name: Option<PathBuf>,
    pub article_mdwn_source: Option<String>,
    pub draft: Option<bool>,

    /// override for the title (derived from filename by default)
    pub title: Option<String>,
    pub modification_date: Option<std::time::SystemTime>,
    pub summary: Option<String>,

    pub tags: Option<Vec<String>>,
    pub series: Option<String>,

    pub special_page: Option<bool>,
    pub timeline: Option<bool>,

    pub anchorjs: Option<bool>,
    pub tocify: Option<bool>,
    pub show_source_link: Option<bool>,
    /// register for live updates (default true)
    pub live_updates: Option<bool>,
}

pub fn scan_articles() -> HashMap<PathBuf, Article> {
    let mut articles: HashMap<PathBuf, Article> = HashMap::new();
    let cfg = config::Config::get();
    let input_path: PathBuf = cfg.input.clone();

    let start_time = std::time::Instant::now();

    fn traverse_and_collect_articles(dir: &PathBuf, articles: &mut HashMap<PathBuf, Article>) {
        if dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_dir() {
                            traverse_and_collect_articles(&path, articles);
                        } else if let Some(ext) = path.extension() {
                            if ext == "mdwn" {
                                match parse_article(&path) {
                                    Ok(article) => {
                                        articles.insert(path, article);
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

    traverse_and_collect_articles(&input_path, &mut articles);

    for (_, article) in &articles {
        println!("Writing article {} to disk", article.clone().dst_file_name.unwrap().display());
        write_article_to_disk(article);
    }

    let duration = start_time.elapsed();
    println!("Time taken to execute: {:?}", duration);

    articles
}

fn write_article_to_disk(article: &Article) {
    let cfg = config::Config::get();
    let output_path: PathBuf = cfg.output.clone();

    let article_mdwn_source = article
        .article_mdwn_source
        .clone()
        .unwrap();

    match render_file(article_mdwn_source.clone()) {
        Ok(mdwn_html) => {
            let content: String =
                create_html_from_content_template(article.clone(), mdwn_html).unwrap();
            let html: String =
                create_html_from_standalone_template(article.clone(), content).unwrap();

            if let Some(dst_file_name) = &article.dst_file_name {
                let mut output_file = output_path.clone();
                output_file.push(dst_file_name);
                std::fs::write(output_file, html).expect("Unable to write HTML file");
            }
        }
        Err(e) => {
            println!("Error: path: {} - {}", article_mdwn_source, e);
        }
    }
}

fn parse_article(article_path: &PathBuf) -> Result<Article, Box<dyn Error>> {
    println!("Parsing article {} from disk", article_path.clone().display());

    let mut article: Article = Article {
        src_file_name: article_path.clone(),
        dst_file_name: None,
        article_mdwn_source: None,
        title: None,
        modification_date: None,
        summary: None,
        tags: None,
        series: None,

        draft: None,
        special_page: None,
        timeline: None,

        anchorjs: None,
        tocify: None,
        show_source_link: None,
        live_updates: None,
    };

    let article_mdwn_raw_string = std::fs::read_to_string(article_path).unwrap();
    match process_plugins(&article_mdwn_raw_string, &mut article) {
        Ok(article_mdwn_refined_source) => {
            article.article_mdwn_source = Some(article_mdwn_refined_source)
        }
        Err(err) => {
            todo!()
        }
    }

    let dst_file_name: PathBuf = article_path
        .with_extension("html")
        .file_name()
        .unwrap()
        .into();
    article.dst_file_name = Some(dst_file_name);

    Ok(article)
}


#[cfg(test)]
mod tests {
    use super::*;  // Importing everything from the parent module
    
    #[test]
    fn test_process_plugins() {

        // let config = config::Config::new(
        //     PathBuf::new(),
        //     PathBuf::new(),
        //     PathBuf::new(),
        //     PathBuf::new(),
        //     23,
        // );
        // config::Config::initialize(config).expect("Failed to initialize config");
        
        let raw_mdwn = "[[!title Test Title]]\n[[!summary This is a summary.]]\n[[!series My Series]]\n[[!tag rust programming]]".to_string();
        let mut article = Article {
            src_file_name: PathBuf::from("example.mdwn"),
            dst_file_name: None,
            article_mdwn_source: None,
            title: None,
            modification_date: None,
            summary: None,
            tags: None,
            series: None,
            draft: None,
            special_page: None,
            timeline: None,
            anchorjs: None,
            tocify: None,
            show_source_link: None,
            live_updates: None,
        };

        let result = process_plugins(&raw_mdwn, &mut article);

        assert!(result.is_ok());
        assert_eq!(article.title, Some("Test Title".to_string()));
        assert_eq!(article.summary, Some("This is a summary.".to_string()));
        assert_eq!(article.series, Some("My Series".to_string()));
        assert_eq!(article.tags, Some(vec!["rust".to_string(), "programming".to_string()]));
    }
}

fn process_plugins(
    article_mdwn_raw_string: &String,
    article: &mut Article,
) -> Result<String, Box<dyn Error>> {
    let mut processed_article = String::new();
    let re = Regex::new(r"\[\[!(.*?)\]\]").unwrap();
    let mut prev_pos = 0;
    let mut found_plugins = Vec::new();

    for mat in re.find_iter(article_mdwn_raw_string) {
        if prev_pos != mat.start() {
            processed_article.push_str(&article_mdwn_raw_string[prev_pos..mat.start()]);
        }
        match call_plugin(&article_mdwn_raw_string[mat.start()..mat.end()], article) {
            Ok(res) => {
                let PluginResult {
                    name: plugin_name,
                    output: plugin_output,
                } = res;
                found_plugins.push(plugin_name);
                processed_article.push_str(&plugin_output);
                prev_pos = mat.end();
            }
            Err(e) => {
                //todo!()
            }
        }
        processed_article.push_str(&article_mdwn_raw_string[prev_pos..]);
    }

    //println!("Plugins: {:?}", found_plugins);
    Ok(processed_article)
}

struct PluginResult {
    name: String,
    output: String,
}

fn call_plugin(input: &str, article: &mut Article) -> Result<PluginResult, Box<dyn Error>> {
    let pattern = r#"\[\[!([\w]+)(?:\s+(.*))?\]\]"#;
    let re = Regex::new(pattern).unwrap();

    if let Some(captures) = re.captures(input) {
        let name = captures.get(1).unwrap().as_str();
        let argument = captures.get(2).map_or("", |m| m.as_str()).trim();

        match name {
            "specialpage" => {
                article.special_page = Some(true);
                Ok(PluginResult {
                    name: "specialpage".to_string(),
                    output: "".to_string(),
                })
            }
            "draft" => {
                article.special_page = Some(true);
                Ok(PluginResult {
                    name: "draft".to_string(),
                    output: "".to_string(),
                })
            }
            "meta" => {
                let re = Regex::new(r"\d{4}-\d{2}-\d{2} \d{2}:\d{2}").unwrap();
                if let Some(mat) = re.find(&argument) {
                    if let Ok(parsed_time) =
                        NaiveDateTime::parse_from_str(mat.as_str(), "%Y-%m-%d %H:%M")
                    {
                        let system_time: SystemTime = Utc.from_utc_datetime(&parsed_time).into();
                        article.modification_date = Some(system_time);
                        return Ok(PluginResult {
                            name: "meta".to_string(),
                            output: "".to_string(),
                        });
                    } else {
                        Err("Argument contains invalid characters (newlines or tabs)".into())
                    }
                } else {
                    Err("Argument contains invalid characters (newlines or tabs)".into())
                }
            }
            "series" => {
                if argument.contains('\n') || argument.contains('\t') {
                    Err("Argument contains invalid characters (newlines or tabs)".into())
                } else {
                    let series = argument.to_string();
                    article.series = Some(series);
                    Ok(PluginResult {
                        name: "series".to_string(),
                        output: "".to_string(),
                    })
                }
            }
            "tag" => {
                if argument.contains('\n') || argument.contains('\t') {
                    Err("Argument contains invalid characters (newlines or tabs)".into())
                } else {
                    let tag = argument.to_string();
                    article.tags = Some(tag.split_whitespace().map(|s| s.to_string()).collect());
                    Ok(PluginResult {
                        name: "tag".to_string(),
                        output: "".to_string(),
                    })
                }
            }
            "img" => {
                if argument.contains('\n') || argument.contains('\t') {
                    Err("Argument contains invalid characters (newlines or tabs)".into())
                } else {
                    let mut parts = argument.split_whitespace();
                    let img = parts.next().unwrap_or("").to_string();
                    let subarg = parts.collect::<Vec<&str>>().join(" ");
                    Ok(PluginResult {
                        name: "img".to_string(),
                        output: format!("<a href=\"{}\"><img src=\"{}\"></a>", img, subarg),
                    })
                }
            }
            "summary" => {
                if argument.contains('\n') || argument.contains('\t') {
                    Err("Argument contains invalid characters (newlines or tabs)".into())
                } else {
                    let summary = argument.to_string();
                    article.summary = Some(summary);
                    Ok(PluginResult {
                        name: "summary".to_string(),
                        output: "".to_string(),
                    })
                }
            }
            "title" => {
                if argument.contains('\n') || argument.contains('\t') {
                    Err("Argument contains invalid characters (newlines or tabs)".into())
                } else {
                    let title = argument.to_string();
                    article.title = Some(title);
                    Ok(PluginResult {
                        name: "title".to_string(),
                        output: "".to_string(),
                    })
                }
            }
            _ => Err("Plugin '{name}' is not supported".into()),
        }
    } else {
        Err("Plugin couldn't be decoded".into())
    }
}
