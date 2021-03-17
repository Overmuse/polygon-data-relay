use crate::server::Command;
use actix_web::{
    web::{Data, Query},
    HttpResponse,
};
use polygon::ws::types::PolygonAction;
use serde::Deserialize;
use std::sync::mpsc::Sender;

#[derive(Deserialize)]
pub struct Subscribe {
    stream: String,
    ticker: String,
}

pub async fn subscribe(message: Query<Subscribe>, tx: Data<Sender<Command>>) -> HttpResponse {
    let response = tx.send(Command::Polygon(PolygonAction {
        action: "subscribe".into(),
        params: format!("{}.{}", message.stream, message.ticker).into(),
    }));

    match response {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(_) => HttpResponse::InternalServerError().finish(),
    }
}
