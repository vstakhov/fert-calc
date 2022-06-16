//! A simple web interface

use crate::FertilizersDb;
use actix_web::{http::header::ContentType, web, App, HttpResponse, HttpServer, Responder};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct WebState {
	db: Arc<Mutex<FertilizersDb>>,
}

async fn list_db(data: web::Data<WebState>) -> impl Responder {
	let locked_db = data.db.lock().unwrap();
	let body = serde_json::to_string(&locked_db.known_fertilizers.keys().collect::<Vec<_>>()).unwrap();
	HttpResponse::Ok().content_type(ContentType::json()).body(body)
}

pub async fn run_server(db: Arc<Mutex<FertilizersDb>>) -> std::io::Result<()> {
	let state = WebState { db: db.clone() };

	HttpServer::new(move || {
		App::new()
			.app_data(web::Data::new(state.clone()))
			.route("/list_db", web::to(list_db))
	})
	.bind(("127.0.0.1", 8080))?
	.run()
	.await
}
