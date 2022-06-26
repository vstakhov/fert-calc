//! A simple web interface

use crate::{compound, concentration::*, elements::KnownElements, tank::Tank, DiluteMethod, Fertilizer, FertilizersDb};
use actix_web::{
	get,
	http::{header::ContentType, StatusCode},
	web, App, HttpResponse, HttpServer, Responder, Result,
};
use anyhow::anyhow;
use either::{Either, Left, Right};
use serde::{Deserialize, Serialize};
use std::{
	fmt,
	net::ToSocketAddrs,
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
	let body = serde_json::to_string(
		&locked_db
			.known_fertilizers
			.iter()
			.map(|(name, fert)| (name, fert.description()))
			.collect::<Vec<_>>(),
	)
	.unwrap();
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
#[derive(Debug, Clone, Deserialize, Serialize)]
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
	listen_addr: impl ToSocketAddrs,
) -> std::io::Result<()> {
	let state = WebState { db: db.clone(), known_elements: known_elements.clone() };

	HttpServer::new(move || {
		App::new()
			.app_data(web::Data::new(state.clone()))
			.service(list_db)
			.service(fertilizer_info)
	})
	.bind(listen_addr)?
	.run()
	.await
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::test_utils::{load_known_elements, load_known_fertilizers, sample_tank, MOLAR_MASS_EPSILON};
	use actix_web::{test, App};

	fn new_state() -> WebState {
		let known_elts = load_known_elements();
		let known_fertilizers = load_known_fertilizers(&known_elts);

		WebState { db: Arc::new(Mutex::new(known_fertilizers)), known_elements: Arc::new(Mutex::new(known_elts)) }
	}

	fn new_calc_data_dry() -> CalcData {
		CalcData {
			fertilizer: "KNO3".to_owned(),
			tank: sample_tank(),
			dosing_data: Left(DryDosing {
				dilute_input: 10.0,
				target_element: Some("NO3".to_owned()),
				what: DiluteCalcType::TargetDose,
			}),
		}
	}
	fn new_calc_data_solution() -> CalcData {
		CalcData {
			fertilizer: "KNO3".to_owned(),
			tank: sample_tank(),
			dosing_data: Right(SolutionDosing {
				portion_volume: 20.0,
				container_volume: 1000.0,
				dose: 10.0,
				target_element: Some("NO3".to_owned()),
				what: DiluteCalcType::TargetDose,
			}),
		}
	}

	#[actix_web::test]
	async fn test_list_db() {
		let app_state = new_state();
		let app = test::init_service(App::new().app_data(web::Data::new(app_state.clone())).service(list_db)).await;
		let req = test::TestRequest::get().uri("/list").to_request();
		let resp: Vec<(String, String)> = test::call_and_read_body_json(&app, req).await;
		assert!(!resp.is_empty());
		assert!(resp.iter().any(|f| f.0.as_str() == "Urea"));
	}

	#[actix_web::test]
	async fn test_info() {
		let app_state = new_state();
		let nitrogen = {
			let locked_elts = app_state.known_elements.lock().unwrap();
			locked_elts.elements.get("N").unwrap().clone()
		};
		let app =
			test::init_service(App::new().app_data(web::Data::new(app_state.clone())).service(fertilizer_info)).await;
		let req = test::TestRequest::get().uri("/info/KNO3").to_request();
		let resp: Vec<ElementsConcentrationsWithAliases> = test::call_and_read_body_json(&app, req).await;
		assert!(!resp.is_empty());
		assert_delta_eq!(
			resp.iter().find(|elt| elt.element == nitrogen).unwrap().concentration,
			0.1385,
			MOLAR_MASS_EPSILON
		);
	}

	#[actix_web::test]
	async fn test_calc() {
		let app_state = new_state();
		let app = test::init_service(App::new().app_data(web::Data::new(app_state.clone())).service(calc)).await;
		let dry_dose = new_calc_data_dry();
		let req = test::TestRequest::get().uri("/calc").set_json(&dry_dose).to_request();
		let resp: DiluteResult = test::call_and_read_body_json(&app, req).await;
		// Tank 170, target: 10ppm NO3
		assert_delta_eq!(resp.compound_dose, 2.772, MOLAR_MASS_EPSILON);

		let dry_dose = new_calc_data_solution();
		let req = test::TestRequest::get().uri("/calc").set_json(&dry_dose).to_request();
		let resp: DiluteResult = test::call_and_read_body_json(&app, req).await;
		// Tank 170, target: 10ppm NO3, container: 1L, dose: 20ml
		assert_delta_eq!(resp.compound_dose, 138.599, MOLAR_MASS_EPSILON);
	}
}
