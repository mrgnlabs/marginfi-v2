use crate::utils::protos::geyser::geyser_client::GeyserClient;
use anyhow::Result;
use tonic::{
    codegen::InterceptedService,
    service::Interceptor,
    transport::{Channel, ClientTlsConfig, Endpoint},
    Request, Status,
};

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
