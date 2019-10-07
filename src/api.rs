use http::Response;
use log::{debug};
use rand::prelude::*;

use std::mem;

use crate::client::AlpnClient; 
use crate::error::Error;
use crate::url::encode_unsafe_url_chars;
use crate::helpers;

use wikiquery::{responses, requests};

pub type QueryResponse = Result<Response<responses::Query>, Error>;

pub mod queries
{
    use super::*;
    
    pub fn all_categories_query<'a>(category: String) -> requests::Query<'a>
    {
        let mut query = requests::Query::new();
        let encoded_category = encode_unsafe_url_chars(&category);
        
        {
            query
                .all_categories()
                .ac_min("1")
                .ac_limit("20")
                .ac_prop("size")
                .ac_prefix(encoded_category);
        }

        query
    }

    pub fn category_members_query<'a>(category: String) -> requests::Query<'a>
    {
        let mut query = requests::Query::new();
        let encoded_category = encode_unsafe_url_chars(&category);

        query
            .category_members()
            .cm_title(encoded_category)
            .cm_type("page")
            .cm_type("subcat")
            .cm_prop("ids")
            .cm_prop("title")
            .cm_prop("type")
            .cm_prop("timestamp")
            .cm_limit("500");

        query
    }

    pub fn details_query<'a>(article: String) -> requests::Query<'a>
    {
        let mut query = requests::Query::new();
        let encoded_article = encode_unsafe_url_chars(&article);

        query.pages()
            .titles(encoded_article)
            .info()
            .in_prop("url")
            .in_prop("displaytitle")
            .extracts()
            .ex_chars("1000")
            .ex_limit("1")
            .ex_plain_text();

        query
    }
}

pub async fn all_categories(client: &AlpnClient, category: String) -> QueryResponse
{
    let mut query = queries::all_categories_query(category);
    helpers::send_query_and_deserialize(client, &mut query).await
}

pub async fn category_members(client: &AlpnClient, category: String) -> QueryResponse
{
    let mut query = queries::category_members_query(category);
    helpers::send_query_and_deserialize(client, &mut query).await
}

pub async fn article_details(client: &AlpnClient, article: String) -> QueryResponse
{
    let mut query = queries::details_query(article);
    helpers::send_query_and_deserialize(client, &mut query).await
}

pub async fn random_category_members(client: &AlpnClient, category: String) -> Result<Vec<responses::category_members::Data>, Error>
{
    debug!("finding category members for: {}", &category);
    let mut query = queries::category_members_query(category);
    let mut members = Vec::new();

    loop
    {
        let resp = helpers::send_query_and_deserialize(client, &mut query).await?;
        let (_head, body) = resp.into_parts();

        members.append(&mut mem::replace(&mut body.query.category_members.unwrap(), Vec::new()));

        if body.continue_block.is_some()
        {
            query.continue_query(&body.continue_block);
        }
        else
        {
            break;
        }
    }

    debug!("members: {:?}", members.len());

    let mut rng = rand::thread_rng();
    members.shuffle(&mut rng);

    Ok(members)
}

pub async fn random_article(client: &AlpnClient, category: String) -> Result<String, Error>
{
    let mut members = random_category_members(client, format!("Category:{}", category)).await?;
    let mut result = Err(Error::NoMembers);

    while let Some(m) = members.pop()
    {
        match m.page_type.unwrap().as_ref() {
            "subcat" => {
                let mut next_members = random_category_members(client, m.title.unwrap()).await?;

                members.append(&mut next_members);
            },
            "page" => {
                result = Ok(m.title.unwrap());
                break;
            }
            unknown_page_type => {
                panic!("Unknown page_type: {}",unknown_page_type);
            }
        }
    }

    result
}
