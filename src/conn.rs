use log::{debug};

use http::{Response};
use serde::Deserialize;

use hyper;
use hyper::Client;
use hyper::body::Body;

use hyper_alpn::AlpnConnector;

use rand::prelude::*;

use std::mem;

use crate::error::Error;
use crate::url::encode_unsafe_url_chars;

pub type AlpnClient = Client<AlpnConnector>;

pub mod client
{
    use super::*;

    pub fn new() -> AlpnClient
    {
        let mut builder = Client::builder();
        builder.http2_only(true);

        builder.build(AlpnConnector::new())
    }
}

pub mod api
{
    use super::*;
    use wikiquery::{responses, requests};

    pub const ADDRESS: &'static str = "https://en.wikipedia.org/wiki/";

    type QueryResponse = Result<Response<responses::Query>, Error>;

    pub mod helpers
    {
        use super::*;
        
        pub async fn send_query_and_deserialize<'a>(client: &AlpnClient, query: &mut requests::Query<'a>) -> QueryResponse
        {
            let req = query.uri().unwrap();
            debug!("Req: {:?}", req);
            let resp = client.get(req).await?;
            debug!("Resp: {:?}", resp);
            deserialize_json::<responses::Query>(resp).await
        }
    }

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
}

async fn body_chunks_to_string(mut body: Body) -> String
{
    let mut data = String::new();

    while let Some(chunk) = body.next().await
    {
        data.push_str(std::str::from_utf8(&*chunk.unwrap()).unwrap())
    }

    data
}

async fn deserialize_json<D>(resp: Response<Body>) -> Result<Response<D>, Error>
    where for<'de> D: Deserialize<'de>
{
    let (parts, body) = resp.into_parts();
    let data = body_chunks_to_string(body).await;
    debug!("deserializing data string: \n{:?}", data);

    let error = serde_json::from_str(&data);

    if let Ok(e) = error
    {
        return Err(Error::Wiki(e));
    }

    let body = serde_json::from_str(&data)?;
    Ok(Response::from_parts(parts, body))
}

#[cfg(test)]
mod test
{
    use super::*;
    use api::helpers::send_query_and_deserialize;
    use tokio::runtime::Runtime;
    use futures_util::future;
    use hyper::Chunk;
    use serde_json::{json};

    use wikiquery::{requests};

    fn sample_response(body: &'static str) -> Response<Body>
    {
        let (mut sender, resp_body) = Body::channel();
        let resp = Response::new(resp_body);
        sender.try_send_data(Chunk::from(body)).unwrap();
        resp
    }

    #[test]
    fn test_deserialize_json_response()
    {
        let rt = Runtime::new().unwrap();
        let test_json_str = "{\"a\": 1, \"b\": [1, 2, 3]}";
        let test_json_val = json!({
            "a": 1,
            "b": [1, 2, 3]
        });

        let resp_serialized = sample_response(test_json_str);
        let resp_deserialized = rt.block_on(deserialize_json::<serde_json::Value>(resp_serialized)).unwrap();

        assert_eq!(test_json_val, *resp_deserialized.body());
    }

    #[test]
    fn test_all_categories()
    {
        let rt = Runtime::new().unwrap();
        let client = client::new();
        let fut_resp = api::all_categories(&client, String::from("Lists_of_colors"));

        let resp = rt.block_on(fut_resp).unwrap();

        debug!("Response is: {:?}", resp);
        debug!("Response body: {:?}", resp.body());

    }

    #[test]
    fn test_category_members()
    {
        let rt = Runtime::new().unwrap();
        let client = client::new();
        let fut_resp = api::category_members(&client, String::from("Category:Lists_of_colors"));

        let resp = rt.block_on(fut_resp).unwrap();

        debug!("Response is: {:?}", resp);
        debug!("Response body: {:?}", resp.body());
    }


    #[test]
    fn test_random_article()
    {
        let rt = Runtime::new().unwrap();
        let client = client::new();
        let fut_resp1 = api::random_article(&client, String::from("War"));
        let fut_resp2 = api::random_article(&client, String::from("War"));

        let resp1 = rt.block_on(fut_resp1).unwrap();
        let resp2 = rt.block_on(fut_resp2).unwrap();

        debug!("Response 1 is: {:?}", resp1);
        debug!("Response 2 is: {:?}", resp2);

        assert_ne!(resp1, resp2);
    }

    #[test]
    #[ignore]
    fn test_find_20_random_articles()
    {
        let rt = Runtime::new().unwrap();
        let client = client::new();

        let fut = (0..20)
            .into_iter()
            .map(|_| api::random_article(&client, "People".to_string()));
            // .collect();

        let fut_resp = rt.block_on(future::join_all(fut));

        debug!("20 random articles are: {:?}", fut_resp);
    }

    #[test]

    fn test_over_1000_subpages()
    {
        let rt = Runtime::new().unwrap();
        let client = client::new();

        let fut = api::random_category_members(&client, String::from("Category:20th-century births"));

        let members = rt.block_on(fut).unwrap();

        assert!(members.len() > 1000);
    }

    #[test]
    fn test_random_article_invalid_category()
    {
        let rt = Runtime::new().unwrap();
        let client = client::new();

        let invalid_category = String::from("aasgesgagaaex");

        let fut = api::random_article(&client, invalid_category);

        let result = rt.block_on(fut).unwrap_err();

        assert_eq!(Error::NoMembers, result);
    }

    #[test]
    fn test_all_categories_invalid_category()
    {
        let rt = Runtime::new().unwrap();
        let client = client::new();

        let invalid_category = String::from("anegageg");

        let fut = api::all_categories(&client, invalid_category);

        let result = rt.block_on(fut).unwrap();

        let (_head, body) = result.into_parts();

        debug!("body is: {:?}", &body);

        let categories = &body.query.all_categories.unwrap();

        assert!(categories.len() == 0);
    }

    #[test]
    fn test_invalid_parameter_value()
    {
        let rt = Runtime::new().unwrap();
        let client = client::new();

        let mut query = requests::Query::new();
        
        {
            query
                .all_categories()
                .ac_prop(String::from("I_am_bad_prop"));
        }


        let fut = send_query_and_deserialize(&client, &mut query);
        let result = rt.block_on(fut).unwrap();
        let (_head, body) = result.into_parts();

        let error = String::from("Unrecognized value for parameter \"acprop\": I_am_bad_prop.");
        let warnings = body.warnings.unwrap().all_categories.unwrap();

        assert_eq!(error, warnings.warnings);
    }

    #[test]
    fn test_invalid_format()
    {
        let rt = Runtime::new().unwrap();
        let client = client::new();

        let mut query = requests::Query::new();
        
        {
            query
                .all_categories()
                .ac_from(String::from("War"));

            query.format("I_am_a_bad_format");
        }


        let fut = send_query_and_deserialize(&client, &mut query);
        let result = rt.block_on(fut).unwrap_err();

        assert!(result.is_serde());
    }

    #[test]
    fn test_article_details() {
        let rt = Runtime::new().unwrap();
        let client = client::new();

        let fut = api::article_details(&client, "Death".to_string());
        let result = rt.block_on(fut).unwrap();

        let body = result.into_body();

        let page = &body.query.pages.unwrap()[0];

        let extract = "Death is the permanent cessation of all biological functions that sustain a living organism. Phenomena which commonly bring about death include aging, predation, malnutrition, disease, suicide, homicide, starvation, dehydration, and accidents or major trauma resulting in terminal injury. In most cases, bodies of living organisms begin to decompose shortly after death.Death – particularly the death of humans – has commonly been considered a sad or unpleasant occasion, due to the affection for the being that has died and the termination of social and familial bonds with the deceased. Other concerns include fear of death, necrophobia, anxiety, sorrow, grief, emotional pain, depression, sympathy, compassion, solitude, or saudade. Many cultures and religions have the idea of an afterlife, and also hold the idea of reward or judgement and punishment for past sin.\n\n\n== Senescence ==\n\nSenescence refers to a scenario when a living being is able to survive all calamities, but eventually dies due to...";

        assert_eq!(page.title, "Death");
        assert_eq!(page.display_title, Some("Death".to_string()));
        assert_eq!(page.full_url, Some("https://en.wikipedia.org/wiki/Death".to_string()));
        assert_eq!(page.extract, Some(extract.to_string()));
    }
}