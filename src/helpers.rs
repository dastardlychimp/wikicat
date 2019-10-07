use http::Response;
use hyper::body::Body;
use log::debug;
use serde::Deserialize;
use serde_json;
use wikiquery::{requests, responses};

use crate::api::QueryResponse;
use crate::error::Error;
use crate::client::AlpnClient;

pub async fn send_query_and_deserialize<'a>(client: &AlpnClient, query: &mut requests::Query<'a>) -> QueryResponse
{
    let req = query.uri().unwrap();
    debug!("Req: {:?}", req);
    let resp = client.get(req).await?;
    debug!("Resp: {:?}", resp);
    deserialize_json::<responses::Query>(resp).await
}

async fn body_chunks_to_string(mut body: Body) -> String
{
    let mut data = Vec::new();

    while let Some(chunk) = body.next().await
    {
        data.extend(&*chunk.unwrap());
    }

    String::from_utf8(data).unwrap()
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
    use hyper::Chunk;
    use serde_json::json;
    use tokio::runtime::Runtime;

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
    fn test_body_to_string_with_char_split_between_chunks() {
        let rt = Runtime::new().unwrap();
        
        let (mut sender, body) = Body::channel();

        let string = "this is a complex character: ൠ".to_string();
        let mut bytes = string.clone().into_bytes();
        let last = vec![bytes.pop().unwrap()];

        let chunk1 = Chunk::from(bytes);
        let chunk2 = Chunk::from(last);

        let send_fut = async move {
            sender.send_data(chunk1).await.unwrap();
            sender.send_data(chunk2).await.unwrap();
        };

        rt.spawn(send_fut);

        let fut = body_chunks_to_string(body);

        let result = rt.block_on(fut);

        assert_eq!(result, string);
    }
}