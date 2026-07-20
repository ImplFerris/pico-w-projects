use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};

use reqwless::client::HttpClient;
// Uncomment these for TLS requests:
// use reqwless::client::{HttpClient, TlsConfig, TlsVerify};
use reqwless::request::Method;
use serde::Deserialize;
use serde_json_core::from_slice;

use defmt::{Debug2Format, error, info};

#[derive(defmt::Format)]
pub enum FetchError {
    Request,
    Send,
    ReadBody,
}

pub async fn fetch_json(stack: embassy_net::Stack<'static>) -> Result<(), FetchError> {
    let client_state = TcpClientState::<1, 4096, 4096>::new();
    let tcp_client = TcpClient::new(stack, &client_state);
    let dns_client = DnsSocket::new(stack);

    // Using non-TLS HTTP for this example
    let mut http_client = HttpClient::new(&tcp_client, &dns_client);

    let url = "http://httpbin.org/json";

    // Uncomment these for TLS requests:
    // let mut tls_read_buffer = [0; 16640];
    // let mut tls_write_buffer = [0; 16640];

    // Uncomment these for TLS requests:
    // let tls_config = TlsConfig::new(seed, &mut tls_read_buffer, &mut tls_write_buffer, TlsVerify::None);
    // let mut http_client = HttpClient::new_with_tls(&tcp_client, &dns_client, tls_config);
    // let url = "https://httpbin.org/json";

    info!("connecting to {}", &url);

    let mut http_request = match http_client.request(Method::GET, url).await {
        Ok(req) => req,
        Err(e) => {
            error!("Failed to make HTTP request: {:?}", e);
            return Err(FetchError::Request);
        }
    };

    let mut rx_buffer = [0; 4096];

    let response = match http_request.send(&mut rx_buffer).await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Failed to send HTTP request: {:?}", e);
            return Err(FetchError::Send);
        }
    };

    info!("Response status: {}", response.status.0);

    let body_bytes = match response.body().read_to_end().await {
        Ok(b) => b,
        Err(_e) => {
            error!("Failed to read response body");
            return Err(FetchError::ReadBody);
        }
    };

    // Parse the JSON response from httpbin.org/json
    #[derive(Deserialize)]
    struct HttpBinResponse<'a> {
        #[serde(borrow)]
        slideshow: SlideShow<'a>,
    }

    #[derive(Deserialize)]
    struct SlideShow<'a> {
        author: &'a str,
        title: &'a str,
    }

    match from_slice::<HttpBinResponse>(body_bytes) {
        Ok((output, _used)) => {
            info!("Successfully parsed JSON response!");
            info!("Slideshow title: {:?}", output.slideshow.title);
            info!("Slideshow author: {:?}", output.slideshow.author);
        }
        Err(e) => {
            error!("Failed to parse JSON response: {}", Debug2Format(&e));
            // Log preview of response for debugging
            let preview = if body_bytes.len() > 200 {
                &body_bytes[..200]
            } else {
                body_bytes
            };
            match core::str::from_utf8(preview) {
                Ok(text) => info!("Response preview: {:?}", text),
                Err(_) => info!("Response preview contains non-UTF-8 data"),
            }
        }
    }

    Ok(())
}
