use rocket::http::Status;

#[get("/health")]
pub fn health() -> Status {
    Status::Ok
}

pub fn launch_server() {
    rocket::ignite().mount("/", routes![health]).launch();
}
