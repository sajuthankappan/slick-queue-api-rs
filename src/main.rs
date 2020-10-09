use config;
use env_logger;
use lapin::{
    options::BasicPublishOptions, BasicProperties, Channel, Connection, ConnectionProperties,
};
use log::info;
use serde::{Deserialize, Serialize};
use slick_models::{PageScoreParameters, ScoreParameters, SiteScoreParameters};
use std::convert::Infallible;
use std::env;
use std::net::SocketAddr;
use warp::Filter;

#[derive(Deserialize, Serialize, Debug)]
struct ApiConfig {
    amqp_uri: String,
    score_queue_name: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct QueueResponse {
    code: i16,
    message: String,
}

#[tokio::main]
async fn main() {
    env_logger::init();
    info!("Starting Slick Queue API..");

    let mut raw_config = config::Config::default();
    raw_config
        .merge(config::Environment::with_prefix("SLICK"))
        .unwrap();
    let api_config = raw_config.try_into::<ApiConfig>().unwrap();

    let amqp_addr = api_config.amqp_uri;
    let conn = Connection::connect(
        &amqp_addr,
        ConnectionProperties::default().with_default_executor(8),
    )
    .await
    .unwrap();
    let channel = conn.create_channel().await.unwrap();

    let ping = warp::path("ping").map(|| format!("pong"));

    let queue_page = warp::post()
        .and(warp::path("queue-page"))
        .and(with_amqp(channel.clone()))
        .and(warp::body::json())
        .and_then(handle_queue_page);

    let queue_site = warp::post()
        .and(warp::path("queue-site"))
        .and(with_amqp(channel.clone()))
        .and(warp::body::json())
        .and_then(handle_queue_site);

    let port = env::var("PORT").unwrap_or("8080".into());
    let server_port = format!("0.0.0.0:{}", port);
    let addr = server_port.parse::<SocketAddr>().unwrap();

    let routes = ping.or(queue_page).or(queue_site);

    println!("Listening on {}", &addr);

    warp::serve(routes).run(addr).await;
}

async fn handle_queue_page(
    channel: Channel,
    page_score_parameters: PageScoreParameters,
) -> Result<impl warp::Reply, warp::Rejection> {
    let parameters = ScoreParameters {
        page: Some(page_score_parameters),
        site: None,
    };

    send_score_request_to_queue(&channel, &parameters).await;

    let resp = QueueResponse {
        code: 200,
        message: String::from("Queued"),
    };

    Ok(warp::reply::json(&resp))
}

async fn handle_queue_site(
    channel: Channel,
    site_score_parameters: SiteScoreParameters,
) -> Result<impl warp::Reply, warp::Rejection> {
    let parameters = ScoreParameters {
        page: None,
        site: Some(site_score_parameters),
    };

    send_score_request_to_queue(&channel, &parameters).await;

    let resp = QueueResponse {
        code: 200,
        message: String::from("Queued"),
    };

    Ok(warp::reply::json(&resp))
}

async fn send_score_request_to_queue(channel: &Channel, parameters: &ScoreParameters) {
    let payload = serde_json::to_string(&parameters).unwrap();

    channel
        .basic_publish(
            "",
            "score-requests",
            BasicPublishOptions::default(),
            payload.into_bytes(),
            BasicProperties::default(),
        )
        .await
        .unwrap()
        .await
        .unwrap();
}

fn with_amqp(channel: Channel) -> impl Filter<Extract = (Channel,), Error = Infallible> + Clone {
    warp::any().map(move || channel.clone())
}
