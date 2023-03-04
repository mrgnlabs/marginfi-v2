use anchor_client::anchor_lang::prelude::thiserror::Error;
use crate::utils::protos::{SubscribeUpdate, geyser::geyser_client::GeyserClient, SubscribeRequest};
use anyhow::{Result};
use backoff::{ExponentialBackoff, future::retry};
use futures::stream::once;
use futures::StreamExt;
use tonic::{codegen::InterceptedService, service::Interceptor, transport::{Channel, ClientTlsConfig, Endpoint}, Request, Status, Streaming, Response};
use tonic::metadata::{Ascii, MetadataValue};
use tracing::{error, info};

pub struct RequestInterceptor {
    auth_token: String,
}

impl Interceptor for RequestInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        request
            .metadata_mut()
            .insert("x-token", self.auth_token.parse().unwrap());
        Ok(request)
    }
}

pub async fn get_geyser_client(
    url: String,
    auth_token: String,
) -> Result<GeyserClient<InterceptedService<Channel, RequestInterceptor>>> {
    let mut endpoint = Endpoint::from_shared(url.clone())?;

    if url.contains("https") {
        endpoint = endpoint.tls_config(ClientTlsConfig::new())?;
    }
    let channel = endpoint.connect().await.unwrap();

    Ok(GeyserClient::with_interceptor(
        channel,
        RequestInterceptor { auth_token },
    ))
}


#[derive(Debug, Error)]
pub enum Error {
    #[error("XToken: {0}")]
    XToken(String),

    #[error("Invalid URI {0}")]
    InvalidUri(String),

    #[error("RetrySubscribe")]
    RetrySubscribe(anyhow::Error),
}

#[derive(Debug)]
pub struct RetryChannel {
    x_token: Option<MetadataValue<Ascii>>,
    channel: Channel,
}

impl RetryChannel {
    /// Establish a channel to tonic endpoint
    /// The channel does not attempt to connect to the endpoint until first use
    pub fn new(endpoint_str: String, x_token_str: String) -> Result<Self, Error> {
        let endpoint: Endpoint;
        let x_token: Option<MetadataValue<Ascii>>;

        // the client should fail immediately if the x-token is invalid
        match x_token_str.parse::<MetadataValue<Ascii>>() {
            Ok(metadata) => x_token = Some(metadata),
            Err(_) => return Err(Error::XToken(x_token_str)),
        }

        let res = Channel::from_shared(endpoint_str.clone());
        match res {
            Err(e) => {
                error!("{}", e);
                return Err(Error::InvalidUri(endpoint_str));
            }
            Ok(_endpoint) => {
                if _endpoint.uri().scheme_str() == Some("https") {
                    match _endpoint.tls_config(ClientTlsConfig::new()) {
                        Err(e) => {
                            error!("{}", e);
                            return Err(Error::InvalidUri(endpoint_str));
                        }
                        Ok(e) => endpoint = e,
                    }
                } else {
                    endpoint = _endpoint;
                }
            }
        }
        let channel = endpoint.connect_lazy();

        Ok(Self { x_token, channel })
    }

    /// Create a new GeyserClient client with Auth interceptor
    /// Clients require `&mut self`, due to `Tonic::transport::Channel` limitations, however
    /// creating new clients is cheap and thus can be used as a work around for ease of use.
    pub fn client(&self) -> RetryClient<impl FnMut(Request<()>) -> InterceptedRequestResult + '_> {
        let client = GeyserClient::with_interceptor(
            self.channel.clone(),
            move |mut req: tonic::Request<()>| {
                if let Some(x_token) = self.x_token.clone() {
                    req.metadata_mut().insert("x-token", x_token);
                }
                Ok(req)
            },
        );
        RetryClient { client }
    }

    pub async fn subscribe_retry(
        &self,
        subscribe_request: &SubscribeRequest,
    ) -> Result<Streaming<SubscribeUpdate>, anyhow::Error> {
        // The default exponential backoff strategy intervals:
        // [500ms, 750ms, 1.125s, 1.6875s, 2.53125s, 3.796875s, 5.6953125s,
        // 8.5s, 12.8s, 19.2s, 28.8s, 43.2s, 64.8s, 97s, ... ]
        retry(ExponentialBackoff::default(), || async {
            info!("Retry to connect to the server");
            let mut client = self.client();
            Ok(client
                .subscribe(subscribe_request)
                .await?)
        },
        ).await
    }
}

type InterceptedRequestResult = std::result::Result<Request<()>, Status>;

pub struct RetryClient<F: FnMut(Request<()>) -> InterceptedRequestResult> {
    client: GeyserClient<InterceptedService<Channel, F>>,
}

impl<F: FnMut(Request<()>) -> InterceptedRequestResult> RetryClient<F> {
    pub async fn subscribe(
        &mut self,
        subscribe_request: &SubscribeRequest,
    ) -> Result<Streaming<SubscribeUpdate>, anyhow::Error> {
        let subscribe_request = subscribe_request.clone();
        let response: Response<Streaming<SubscribeUpdate>> =
            self.client.subscribe(once(async move { subscribe_request })).await?;
        let stream: Streaming<SubscribeUpdate> = response.into_inner();

        Ok(stream)
    }
}
