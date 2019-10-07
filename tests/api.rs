use tokio;
use tokio::runtime::Runtime;
use futures_util::future;
use wikiquery::{requests};

use wikicat::api;
use wikicat::client;
use wikicat::error::Error;
use wikicat::helpers::send_query_and_deserialize;

mod test
{
    use super::*;

    #[test]
    fn test_all_categories()
    {
        let rt = Runtime::new().unwrap();
        let client = client::new();
        let fut_resp = api::all_categories(&client, String::from("Lists_of_colors"));

        let resp = rt.block_on(fut_resp).unwrap();
        let body = resp.into_body();

        let categories = body.query.all_categories.unwrap();
        let first = &categories[0];
        
        assert_eq!(categories.len(), 1);
        assert_eq!(first.category, "Lists of colors".to_string());
        assert_eq!(first.size, Some(18));
        assert!(first.pages.is_some());
        assert!(first.files.is_some());
        assert!(first.subcats.is_some());
    }

    #[test]
    fn test_category_members()
    {
        let rt = Runtime::new().unwrap();
        let client = client::new();
        let fut_resp = api::category_members(&client, String::from("Category:Lists_of_colors"));

        let resp = rt.block_on(fut_resp).unwrap();
        let body = resp.into_body();

        let members = body.query.category_members.unwrap();
        let first = &members[0];

        assert_eq!(members.len(), 18);
        assert_eq!(first.title, Some("".to_string()))
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

        let resp = rt.block_on(future::join_all(fut));

        assert_eq!(resp.len(), 20);
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