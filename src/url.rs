use percent_encoding;

pub const URL_UNSAFE_ASCII: percent_encoding::AsciiSet = percent_encoding::CONTROLS
    .add(b' ')
    .add(b'<')
    .add(b'>')
    .add(b'[')
    .add(b']')
    .add(b'{')
    .add(b'}')
    .add(b'|')
    .add(b'\\')
    .add(b'^');

pub fn encode_unsafe_url_chars(url: &str) -> String
{
    percent_encoding::utf8_percent_encode(url, &URL_UNSAFE_ASCII).to_string()
}


#[cfg(test)]
mod test
{
    use super::*;
    
    #[test]
    fn test_no_change_safe_url() {
        let url = "https://wikicat.com";
        let encoded = encode_unsafe_url_chars(&url);
        assert_eq!(encoded, String::from(url));
    }

    #[test]
    fn test_encoded_url_space() {
        let url = " ";
        let encoded = encode_unsafe_url_chars(url);
        assert_eq!(encoded, String::from("%20"));

    }
    
    #[test]
    fn test_encoded_url_other_chars() {
        let url = "êëè";
        let encoded = encode_unsafe_url_chars(url);
        assert_eq!(encoded, String::from("%C3%AA%C3%AB%C3%A8"));
    }
}