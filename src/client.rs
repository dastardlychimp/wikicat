use hyper::Client;
use hyper_alpn::AlpnConnector;

pub type AlpnClient = Client<AlpnConnector>;

pub fn new() -> AlpnClient
{
    let mut builder = Client::builder();
    builder.http2_only(true);

    builder.build(AlpnConnector::new())
}
