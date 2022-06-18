//! A simple web interface

use crate::{compound, concentration::*, elements::KnownElements, tank::Tank, DiluteMethod, Fertilizer, FertilizersDb};
use actix_web::{
	get,
	http::{header::ContentType, StatusCode},
	web, App, HttpResponse, HttpServer, Responder, Result,
};
use anyhow::anyhow;
use either::{Either, Left, Right};
use serde::Deserialize;
use std::{
	fmt,
	sync::{Arc, Mutex},
};
use strum::EnumString;

#[derive(Clone)]
struct WebState {
	db: Arc<Mutex<FertilizersDb>>,
	known_elements: Arc<Mutex<KnownElements>>,
}

#[get("/list")]
async fn list_db(state: web::Data<WebState>) -> impl Responder {
	let locked_db = state.db.lock().unwrap();
	let body = serde_json::to_string(&locked_db.known_fertilizers.keys().collect::<Vec<_>>()).unwrap();
	HttpResponse::Ok().content_type(ContentType::json()).body(body)
}

#[get("/info/{name}")]
async fn fertilizer_info(name: web::Path<String>, state: web::Data<WebState>) -> impl Responder {
	let locked_db = state.db.lock().unwrap();
	let locked_elts = state.known_elements.lock().unwrap();
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

#[derive(PartialEq, Eq, Debug, Clone, Copy, EnumString, Deserialize)]
enum DosingMethod {
	Dry,
	Solution,
}

#[derive(Debug)]
struct WebError {
	err: anyhow::Error,
}
impl fmt::Display for WebError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{:?}", self.err)
	}
}

impl actix_web::error::ResponseError for WebError {
	fn status_code(&self) -> StatusCode {
		StatusCode::BAD_REQUEST
	}
}
impl From<anyhow::Error> for WebError {
	fn from(err: anyhow::Error) -> WebError {
		WebError { err }
	}
}
impl From<serde_json::Error> for WebError {
	fn from(err: serde_json::Error) -> WebError {
		WebError { err: anyhow!("json serialization error: {:?}", err) }
	}
}

// Generic calculation request for a specific tank and compound/ready fertilizer
#[derive(Debug, Clone, Deserialize)]
struct CalcData {
	tank: Tank,
	fertilizer: String,
	#[serde(with = "either::serde_untagged")]
	dosing_data: Either<DryDosing, SolutionDosing>,
}

#[get("/calc")]
async fn calc(data: web::Json<CalcData>, state: web::Data<WebState>) -> Result<impl Responder> {
	let locked_db = state.db.lock().unwrap();
	let maybe_known_fertilizer = locked_db.known_fertilizers.get(data.fertilizer.as_str());
	let locked_elts = state.known_elements.lock().unwrap();

	let real_ferilizer = match maybe_known_fertilizer {
		Some(fertilizer_box) => dyn_clone::clone(fertilizer_box),
		None => {
			let compound = compound::Compound::new(data.fertilizer.as_str(), &locked_elts)
				.map_err(|e| -> WebError { e.into() })?;
			Box::new(compound)
		},
	};
	let tank = &data.tank;
	let dosages = match &data.dosing_data {
		Left(dry_dosing) => dry_dosing
			.dilute(&*real_ferilizer, &locked_elts, tank)
			.map_err(|e| -> WebError { e.into() })?,
		Right(solution_dosing) => solution_dosing
			.dilute(&*real_ferilizer, &locked_elts, tank)
			.map_err(|e| -> WebError { e.into() })?,
	};
	Ok(web::Json(dosages))
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
