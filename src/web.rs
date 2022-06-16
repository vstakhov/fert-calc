//! A simple web interface

use crate::{compound, elements::KnownElements, Fertilizer, FertilizersDb};
use actix_web::{
	get,
	http::{header::ContentType, StatusCode},
	web, App, HttpResponse, HttpServer, Responder,
};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
struct WebState {
	db: Arc<Mutex<FertilizersDb>>,
	known_elements: Arc<Mutex<KnownElements>>,
}

#[get("/list")]
async fn list_db(data: web::Data<WebState>) -> impl Responder {
	let locked_db = data.db.lock().unwrap();
	let body = serde_json::to_string(&locked_db.known_fertilizers.keys().collect::<Vec<_>>()).unwrap();
	HttpResponse::Ok().content_type(ContentType::json()).body(body)
}

#[get("/info/{name}")]
async fn fertilizer_info(name: web::Path<String>, data: web::Data<WebState>) -> impl Responder {
	let locked_db = data.db.lock().unwrap();
	let locked_elts = data.known_elements.lock().unwrap();
	if let Some(fertilizer_box) = locked_db.known_fertilizers.get(name.as_str()) {
		let components = fertilizer_box.components_percentage(&locked_elts);
		let body = serde_json::to_string(&components).unwrap();
		HttpResponse::Ok().content_type(ContentType::json()).body(body)
	} else {
		let maybe_compound =
			compound::Compound::new(name.as_str(), &locked_elts).map_err(|_| HttpResponse::new(StatusCode::NOT_FOUND));

		match maybe_compound {
			Ok(compound) => {
				let body = serde_json::to_string(&compound.components_percentage(&locked_elts)).unwrap();
				HttpResponse::Ok().content_type(ContentType::json()).body(body)
			},
			Err(resp) => resp,
		}
	}
}

pub async fn run_server(
	db: Arc<Mutex<FertilizersDb>>,
	known_elements: Arc<Mutex<KnownElements>>,
) -> std::io::Result<()> {
	let state = WebState { db: db.clone(), known_elements: known_elements.clone() };

	HttpServer::new(move || {
		App::new()
			.app_data(web::Data::new(state.clone()))
			.service(list_db)
			.service(fertilizer_info)
	})
	.bind(("127.0.0.1", 8080))?
	.run()
	.await
}
