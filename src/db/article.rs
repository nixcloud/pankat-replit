use crate::articles::ArticleWithTags;
use crate::articles::NewArticle;

use crate::db::schema;

use crate::db::schema::tags::dsl as tags_objects;
use crate::db::schema::tags::dsl::tags as tags_table;

use crate::db::schema::articles::dsl as articles_objects;
use crate::db::schema::articles::dsl::articles as articles_table;

use crate::db::schema::article_tags::dsl as article_tags_objects;
use crate::db::schema::article_tags::dsl::article_tags as article_tags_table;

use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::Nullable;

use chrono::NaiveDateTime;

#[derive(Queryable, Insertable, Identifiable, Selectable, Debug, Clone, PartialEq)]
#[diesel(table_name = schema::articles)]
pub struct Article {
    pub id: i32,
    pub src_file_name: String,
    pub dst_file_name: String,
    pub title: Option<String>,
    pub modification_date: Option<NaiveDateTime>,
    pub summary: Option<String>,
    pub series: Option<String>,
    pub draft: Option<bool>,
    pub special_page: Option<bool>,
    pub timeline: Option<bool>,
    pub anchorjs: Option<bool>,
    pub tocify: Option<bool>,
    pub live_updates: Option<bool>,
}

impl From<Article> for ArticleWithTags {
    fn from(article: Article) -> Self {
        ArticleWithTags {
            id: Some(article.id),
            src_file_name: article.src_file_name,
            dst_file_name: article.dst_file_name,
            title: article.title,
            modification_date: article.modification_date,
            summary: article.summary,
            series: article.series,
            draft: article.draft,
            special_page: article.special_page,
            timeline: article.timeline,
            anchorjs: article.anchorjs,
            tocify: article.tocify,
            live_updates: article.live_updates,
            tags: None,
        }
    }
}
#[derive(Queryable, Selectable, Identifiable, Debug, PartialEq)]
#[diesel(belongs_to(Article, foreign_key = article_id))]
#[diesel(table_name = schema::tags)]
pub struct Tag {
    pub id: i32,
    pub name: String,
}

#[derive(Identifiable, Insertable, Selectable, Queryable, Associations, Debug)]
#[diesel(belongs_to(Article))]
#[diesel(belongs_to(Tag))]
#[diesel(table_name = schema::article_tags)]
#[diesel(primary_key(article_id, tag_id))]
pub struct ArticleTag {
    pub article_id: i32,
    pub tag_id: i32,
}

fn get_tags_for_article(conn: &mut SqliteConnection, article_id: i32) -> Option<Vec<String>> {
    let tag_ids_result: QueryResult<Vec<i32>> = article_tags_table
        .filter(article_tags_objects::article_id.eq(article_id))
        .select(article_tags_objects::tag_id)
        .load::<i32>(conn);
    match tag_ids_result {
        Ok(tag_ids) => {
            let mut tag_names = Vec::new();
            for tag_id in tag_ids {
                let name_result: QueryResult<String> = tags_table
                    .filter(tags_objects::id.eq(tag_id))
                    .select(tags_objects::name)
                    .first(conn);
                if let Ok(t_name) = name_result {
                    tag_names.push(t_name);
                }
            }
            Some(tag_names)
        }
        Err(_) => None,
    }
}

pub fn get_article_with_tags_by_id(
    conn: &mut SqliteConnection,
    article_id: i32,
) -> Option<ArticleWithTags> {
    let res = articles_table
        .filter(articles_objects::id.eq(article_id))
        .first::<Article>(conn);
    match res {
        Ok(article) => {
            let mut article_with_tags: ArticleWithTags = article.clone().into();
            article_with_tags.tags = get_tags_for_article(conn, article_id);
            Some(article_with_tags)
        }
        Err(e) => None,
    }
}

// func (a *ArticlesDb) MostRecentArticle() (Article, error) {
pub fn get_most_recent_article(conn: &mut SqliteConnection) -> Option<ArticleWithTags> {
    let res = articles_table
        .filter(
            articles_objects::draft
                .eq(false)
                .or(articles_objects::draft.is_null()),
        )
        .filter(
            articles_objects::special_page
                .eq(false)
                .or(articles_objects::special_page.is_null()),
        )
        .order((
            sql::<Nullable<diesel::sql_types::Timestamp>>("modification_date IS NULL"),
            articles_objects::modification_date.desc(),
        ))
        .first::<Article>(conn);
    match res {
        Ok(article) => {
            let mut article_with_tags: ArticleWithTags = article.clone().into();
            article_with_tags.tags = get_tags_for_article(conn, article.id);
            Some(article_with_tags)
        }
        Err(e) => None,
    }
}

// func (a *ArticlesDb) QueryAll() ([]Article, error) {
pub fn get_all_articles(conn: &mut SqliteConnection) -> Vec<ArticleWithTags> {
    let res: QueryResult<Vec<Article>> = articles_table
        .order((
            sql::<Nullable<diesel::sql_types::Timestamp>>("modification_date IS NULL"),
            articles_objects::modification_date.desc(),
        ))
        .load(conn);
    match res {
        Ok(articles) => {
            let mut results = Vec::new();
            for article in articles.iter() {
                let mut article_with_tags: ArticleWithTags = article.clone().into();
                article_with_tags.tags = get_tags_for_article(conn, article.id);
                results.push(article_with_tags)
            }
            results
        }
        Err(e) => {
            vec![]
        }
    }
}

//func (a *ArticlesDb) Articles() ([]Article, error) { -> all articles, except drafts / special pages
pub fn get_visible_articles(
    conn: &mut SqliteConnection,
) -> Result<Vec<ArticleWithTags>, diesel::result::Error> {
    // FIXME rewrite most functions to this return type
    let articles_query = articles_table
        .filter(
            articles_objects::draft
                .eq(false)
                .or(articles_objects::draft.is_null()),
        )
        .filter(
            articles_objects::special_page
                .eq(false)
                .or(articles_objects::special_page.is_null()),
        )
        .order((
            sql::<Nullable<diesel::sql_types::Timestamp>>("modification_date IS NULL"),
            articles_objects::modification_date.desc(),
        ))
        .load::<Article>(conn);
    match articles_query {
        Ok(articles) => {
            let mut articles_out: Vec<ArticleWithTags> = Vec::new();
            for article_in in articles {
                let mut article_with_tags: ArticleWithTags = article_in.clone().into();
                article_with_tags.tags = get_tags_for_article(conn, article_in.id);
                articles_out.push(article_with_tags);
            }
            Ok(articles_out)
        }
        Err(e) => Err(e),
    }
}

// func (a *ArticlesDb) ArticlesBySeries(series string) ([]Article, error) {
pub fn get_visible_articles_by_series(
    conn: &mut SqliteConnection,
    series: &str,
) -> Vec<ArticleWithTags> {
    let res = articles_table
        .filter(articles_objects::series.eq(series))
        .filter(
            articles_objects::draft
                .eq(false)
                .or(articles_objects::draft.is_null()),
        )
        .filter(
            articles_objects::special_page
                .eq(false)
                .or(articles_objects::special_page.is_null()),
        )
        .order((
            sql::<Nullable<diesel::sql_types::Timestamp>>("modification_date IS NULL"),
            articles_objects::modification_date.desc(),
        ))
        .load::<Article>(conn);
    match res {
        Ok(articles) => {
            let mut articles_out: Vec<ArticleWithTags> = Vec::new();
            for article_in in articles {
                let mut article_with_tags: ArticleWithTags = article_in.clone().into();
                article_with_tags.tags = get_tags_for_article(conn, article_in.id);
                articles_out.push(article_with_tags);
            }
            articles_out
        }
        Err(e) => {
            vec![]
        }
    }
}

// func (a *ArticlesDb) ArticlesByTag(tagName string) ([]Article, error) {
pub fn get_visible_articles_by_tag(
    conn: &mut SqliteConnection,
    tag: String,
) -> Vec<ArticleWithTags> {
    let res = articles_table
        .inner_join(
            article_tags_table.on(articles_objects::id.eq(article_tags_objects::article_id)),
        )
        .inner_join(tags_table.on(article_tags_objects::tag_id.eq(tags_objects::id)))
        .filter(tags_objects::name.eq(tag))
        .select(articles_table::all_columns())
        .load::<Article>(conn);
    match res {
        Ok(articles) => {
            let mut articles_out: Vec<ArticleWithTags> = Vec::new();
            for article_in in articles {
                let mut article_with_tags: ArticleWithTags = article_in.clone().into();
                article_with_tags.tags = get_tags_for_article(conn, article_in.id);
                articles_out.push(article_with_tags);
            }
            articles_out
        }
        Err(e) => {
            vec![]
        }
    }
}

// func (a *ArticlesDb) Drafts() ([]Article, error) {
pub fn get_drafts(conn: &mut SqliteConnection) -> Vec<ArticleWithTags> {
    let res = articles_table
        .filter(articles_objects::draft.eq(true))
        .order((
            sql::<Nullable<diesel::sql_types::Timestamp>>("modification_date IS NULL"),
            articles_objects::modification_date.desc(),
        ))
        .load::<Article>(conn);
    match res {
        Ok(articles) => {
            let mut articles_out: Vec<ArticleWithTags> = Vec::new();
            for article_in in articles {
                let mut article_with_tags: ArticleWithTags = article_in.clone().into();
                article_with_tags.tags = get_tags_for_article(conn, article_in.id);
                articles_out.push(article_with_tags);
            }
            articles_out
        }
        Err(e) => {
            vec![]
        }
    }
}

// func (a *ArticlesDb) SpecialPages() ([]Article, error) {
pub fn get_special_pages(conn: &mut SqliteConnection) -> Vec<ArticleWithTags> {
    let res = articles_table
        .filter(articles_objects::special_page.eq(true))
        .order((
            sql::<Nullable<diesel::sql_types::Timestamp>>("modification_date IS NULL"),
            articles_objects::modification_date.desc(),
        ))
        .load::<Article>(conn);
    match res {
        Ok(articles) => {
            let mut articles_out: Vec<ArticleWithTags> = Vec::new();
            for article_in in articles {
                let mut article_with_tags: ArticleWithTags = article_in.clone().into();
                article_with_tags.tags = get_tags_for_article(conn, article_in.id);
                articles_out.push(article_with_tags);
            }
            articles_out
        }
        Err(e) => {
            vec![]
        }
    }
}

// func (a *ArticlesDb) Set(article *Article) (*Article, []string, error) {
// returns affected neighbours like:
// * next/prev neighbours
// * next/prev tags neighbours
// * next/prev series neighbours
// * most recent article (if changed)
// * add/del on draft (so leptos can display an adapted list using /ws)
// * add/del on special_pages (so leptos can display an adapted list using /ws)

// 1. check if article is exists
// 1. a) it exists, update it but track old neigbours
//  2. update tags
//  3. update article_tags bindings
//  4. finish transaction
// 1. b) it doesn't exist, create it
// follow 2./3./4.
pub fn set(conn: &mut SqliteConnection, new_article_with_tags: &ArticleWithTags) {
    //let article: Article = new_article_with_tags.clone().into();
    let new_article: NewArticle = new_article_with_tags.clone().into();
    println!("asdf");

    let _ = conn.transaction(|mut conn| {
        let articles_result = diesel::insert_into(articles_table)
            .values(new_article)
            .get_results::<Article>(conn);
        //println!("asdf1 {:#?}", articles_result);

        match articles_result {
            Ok(ref articles_result) => {
                println!("asdf2");

                let article_id: i32 = articles_result[0].id; // FIXME error handling

                if let Some(tags) = new_article_with_tags.tags.clone() {
                    // add to tags table and reference it in article_tags table
                    for tag in tags.iter() {
                        let tag_result = diesel::insert_into(tags_table)
                            .values(tags_objects::name.eq(tag))
                            .on_conflict(tags_objects::name)
                            .do_nothing()
                            .get_result::<Tag>(conn);

                        let tag_id: i32 = match tag_result {
                            Ok(tag_result) => {
                                // If the insert was successful, query the inserted tag to get its ID
                                let inserted_tag = tags_table
                                    .filter(tags_objects::name.eq(tag))
                                    .select(tags_objects::id)
                                    .first::<i32>(conn);
                                inserted_tag.unwrap()
                            }
                            Err(_) => {
                                // If the insert failed due to a conflict, query the existing tag by name and get its ID
                                let existing_tag = tags_table
                                    .filter(tags_objects::name.eq(tag))
                                    .select(tags_objects::id)
                                    .first::<i32>(conn);
                                existing_tag.unwrap()
                            }
                        };

                        let article_tag: ArticleTag = ArticleTag { article_id, tag_id };
                        println!(" -> {} - {:?}", tag, article_tag);

                        let _ = diesel::insert_into(article_tags_table)
                            .values(article_tag)
                            .execute(conn);
                    }
                }
            }
            Err(ref e) => {
                println!("Article already exists id articles db: either implement update or remove db to re-generate article");
            }
        };

        articles_result
    });
}

// func (a *ArticlesDb) Del(SrcFileName string) ([]string, error) {
pub fn del_by_src_file_name(
    conn: &mut SqliteConnection,
    src_file_name: String,
) -> Result<(), String> {
    let num_deleted = diesel::delete(
        articles_table.filter(articles_objects::src_file_name.eq(src_file_name.clone())),
    )
    .execute(conn);

    match num_deleted {
        Ok(0) => Err(format!("Article not found: {}", src_file_name)),
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to delete article: {}", e)),
    }
}

pub fn del_by_id(conn: &mut SqliteConnection, id: i32) -> Result<(), String> {
    let num_deleted =
        diesel::delete(articles_table.filter(articles_objects::id.eq(id))).execute(conn);

    match num_deleted {
        Ok(0) => Err(format!("Article with id '{}' not found", id)),
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Failed to delete article: {}", e)),
    }
}

// func (a *ArticlesDb) AllTagsInDB() ([]string, error) {
pub fn get_all_tags(conn: &mut SqliteConnection) -> QueryResult<Vec<String>> {
    tags_table.select(tags_objects::name).load(conn)
}

// func (a *ArticlesDb) AllSeriesInDB() ([]string, error) {
pub fn get_all_series_from_visible_articles(conn: &mut SqliteConnection) -> Vec<String> {
    let res: QueryResult<Vec<Article>> = articles_table
        .filter(articles_objects::series.is_not_null())
        .order((
            sql::<Nullable<diesel::sql_types::Timestamp>>("modification_date IS NULL"),
            articles_objects::modification_date.desc(),
        ))
        .load(conn);
    match res {
        Ok(articles) => {
            let mut results: Vec<String> = Vec::new();
            for article in articles.iter() {
                match &article.series.clone() {
                    Some(series) => {
                        results.push(series.clone());
                    }
                    None => {}
                }
            }
            results
        }
        Err(e) => {
            vec![]
        }
    }
}

pub struct Neighbours {
    prev: Option<ArticleWithTags>,
    next: Option<ArticleWithTags>,
}

// func (a *ArticlesDb) NextArticle(article Article) (*Article, error) {
// func (a *ArticlesDb) PrevArticle(article Article) (*Article, error) {
pub fn get_prev_and_next_article(
    conn: &mut SqliteConnection,
    id: i32,
) -> Result<Neighbours, diesel::result::Error> {
    let articles_query: QueryResult<Vec<Article>> = articles_table
        .filter(
            articles_objects::draft
                .eq(false)
                .or(articles_objects::draft.is_null()),
        )
        .filter(
            articles_objects::special_page
                .eq(false)
                .or(articles_objects::special_page.is_null()),
        )
        .order((
            sql::<Nullable<diesel::sql_types::Timestamp>>("modification_date IS NULL"),
            articles_objects::modification_date.desc(),
        ))
        .load::<Article>(conn);
    match articles_query {
        Ok(articles) => {
            let mut prev_article: Option<ArticleWithTags> = None;
            let mut next_article: Option<ArticleWithTags> = None;
            if let Some(pos) = articles.iter().position(|article| article.id == id) {
                if pos > 0 {
                    let p: Article = articles[pos - 1].clone();
                    let mut prev: ArticleWithTags = p.clone().into();
                    prev.tags = get_tags_for_article(conn, p.id);
                    prev_article = Some(prev);
                }
                if pos < articles.len() - 1 {
                    let n: Article = articles[pos + 1].clone();
                    let mut next: ArticleWithTags = n.clone().into();
                    next.tags = get_tags_for_article(conn, n.id);
                    next_article = Some(next);
                }
            }

            let n = Neighbours {
                prev: prev_article,
                next: next_article,
            };
            Ok(n)
        }
        Err(e) => {
            println!("Error: {}", e);
            Err(e)
        }
    }
}

// func (a *ArticlesDb) NextArticleInSeries(article Article) (Article, error) {
// func (a *ArticlesDb) PrevArticleInSeries(article Article) (Article, error) {
pub fn get_prev_and_next_article_for_series(
    conn: &mut SqliteConnection,
    id: i32,
    series: String,
) -> Result<Neighbours, diesel::result::Error> {
    let articles_query: QueryResult<Vec<Article>> = articles_table
        .filter(articles_objects::series.eq(series))
        .filter(
            articles_objects::draft
                .eq(false)
                .or(articles_objects::draft.is_null()),
        )
        .filter(
            articles_objects::special_page
                .eq(false)
                .or(articles_objects::special_page.is_null()),
        )
        .order((
            sql::<Nullable<diesel::sql_types::Timestamp>>("modification_date IS NULL"),
            articles_objects::modification_date.desc(),
        ))
        .load::<Article>(conn);
    match articles_query {
        Ok(articles) => {
            let mut prev_article: Option<ArticleWithTags> = None;
            let mut next_article: Option<ArticleWithTags> = None;
            if let Some(pos) = articles.iter().position(|article| article.id == id) {
                if pos > 0 {
                    let p: Article = articles[pos - 1].clone();
                    let mut prev: ArticleWithTags = p.clone().into();
                    prev.tags = get_tags_for_article(conn, p.id);
                    prev_article = Some(prev);
                }
                if pos < articles.len() - 1 {
                    let n: Article = articles[pos + 1].clone();
                    let mut next: ArticleWithTags = n.clone().into();
                    next.tags = get_tags_for_article(conn, n.id);
                    next_article = Some(next);
                }
            }

            let n = Neighbours {
                prev: prev_article,
                next: next_article,
            };
            Ok(n)
        }
        Err(e) => {
            println!("Error: {}", e);
            Err(e)
        }
    }
}

// func (a *ArticlesDb) GetRelatedArticles(article Article) map[string]bool {
// func (a *ArticlesDb) QueryRawBySrcFileName(SrcFileName string) (*Article, error) {
