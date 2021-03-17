use actix_web::{
    web::{Data, Query},
    HttpResponse,
};
use polygon::ws::types::PolygonAction;
use serde::Deserialize;
use std::sync::mpsc::Sender;

#[derive(Deserialize)]
pub struct Unsubscribe {
    stream: String,
    ticker: String,
}

pub async fn unsubscribe(
    message: Query<Unsubscribe>,
    tx: Data<Sender<PolygonAction>>,
) -> HttpResponse {
    let response = tx.send(PolygonAction {
        action: "unsubscribe".into(),
        params: format!("{}.{}", message.stream, message.ticker),
    });
    match response {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
