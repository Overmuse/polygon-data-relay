use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use polygon::ws::types::PolygonAction;
use routes::{health_check, subscribe, unsubscribe};
use std::net::TcpListener;
use std::sync::mpsc::Sender;

mod routes;

pub fn launch_server(tx: Sender<PolygonAction>) -> Result<Server, std::io::Error> {
    let address = TcpListener::bind("0.0.0.0:8888")?;
    let server = HttpServer::new(move || {
        App::new()
            .data(tx.clone())
            .route("/health_check", web::get().to(health_check))
            .route("/subscribe", web::get().to(subscribe))
            .route("/unsubscribe", web::get().to(unsubscribe))
    })
    .listen(address)?
    .run();
    Ok(server)
}
