use std::sync::{Arc, Mutex};

use anyhow::Result;

use fert_calc::{concentration::*, elements::KnownElements, fertilizers_db::FertilizersDb, tank::Tank, traits::DiluteMethod, Fertilizer};

slint::include_modules!();

fn load_databases() -> Result<(Arc<Mutex<KnownElements>>, Arc<Mutex<FertilizersDb>>)> {
	let known_elements_toml = include_str!("../../elements.toml");
	let known_elements = KnownElements::new_with_string(known_elements_toml)?;
	let mut fertilizers_db: FertilizersDb = Default::default();
	let known_fertilizers_toml = include_str!("../../fertilizers.toml");
	fertilizers_db.load_db(known_fertilizers_toml, &known_elements)?;
	Ok((Arc::new(Mutex::new(known_elements)), Arc::new(Mutex::new(fertilizers_db))))
}

fn compute_dose(
	fertilizer_name: &str,
	tank_volume_l: f64,
	dosing_kind: &str,
	calc_type: &str,
	amount: f64,
	container_ml: f64,
	portion_ml: f64,
	target_element: Option<&str>,
	absolute: bool,
) -> Result<DiluteResult> {
	let (known_elts, db) = load_databases()?;
	let known_elts = known_elts.lock().unwrap();
	let db = db.lock().unwrap();
	let fert = if let Some(k) = db.known_fertilizers.get(fertilizer_name) {
		dyn_clone::clone(k)
	} else {
		Box::new(fert_calc::compound::Compound::new(fertilizer_name, &known_elts)?)
	};
	let tank = Tank::new_from_toml(format!("volume = {}\nabsolute = {}\n", tank_volume_l, absolute).as_str())?;
	let what = match calc_type {
		"Target" => DiluteCalcType::TargetDose,
		_ => DiluteCalcType::ResultOfDose,
	};
	match dosing_kind {
		"Solution" => {
			let dosing = SolutionDosing {
				container_volume: container_ml,
				portion_volume: portion_ml,
				solution_input: amount,
				what,
				target_element: target_element.map(|s| s.to_string()),
			};
			dosing.dilute(&*fert, &known_elts, &tank)
		},
		_ => {
			let dosing = DryDosing { dilute_input: amount, what, target_element: target_element.map(|s| s.to_string()) };
			dosing.dilute(&*fert, &known_elts, &tank)
		},
	}
}

fn main() -> Result<()> {
	let app = AppWindow::new()?;

	if let Ok((_, db)) = load_databases() {
		let db = db.lock().unwrap();
		let mut list: Vec<slint::SharedString> = db
			.known_fertilizers
			.keys()
			.map(|k| slint::SharedString::from(k.as_str()))
			.collect();
		list.sort();
		let model = slint::ModelRc::new(slint::VecModel::from(list));
		app.set_known_fertilizers(model.clone());
		app.set_filtered_fertilizers(model.clone());
		// keep dropdown selection in sync with input
		let _app_sync = app.as_weak();
		// no explicit input change listener needed; bound in UI
	}

	let app_weak = app.as_weak();
	app.on_compute(move |_, tank_l, dosing_kind, calc_type, amount, container_ml, portion_ml, target, absolute| {
		let app = app_weak.unwrap();
		let fertilizer = app.get_fertilizer_input();
		let target_opt = if target.is_empty() { None } else { Some(target.as_str()) };
		match compute_dose(fertilizer.as_str(), tank_l as f64, &dosing_kind.to_string(), &calc_type.to_string(), amount as f64, container_ml as f64, portion_ml as f64, target_opt, absolute) {
			Ok(result) => {
				let summary = format!("Dose: {:.3} g", result.compound_dose);
				let mut details = String::new();
				for d in result.elements_dose.iter() {
					let (dose, unit) = if d.dose <= 0.01 { (d.dose * 1000.0, "ug/l") } else { (d.dose, "mg/l") };
					details.push_str(&format!("{}: {:.3} {}\n", d.element.name, dose, unit));
				}
				app.set_result_summary(summary.into());
				app.set_result_details(details.into());
			},
			Err(err) => {
				app.set_result_summary(format!("Error: {}", err).into());
				app.set_result_details("".into());
			}
		}
	});

	let app_weak = app.as_weak();
	app.on_show_info(move |fertilizer| {
		let app = app_weak.unwrap();
		match load_databases() {
			Ok((known_elts, db)) => {
				let known_elts = known_elts.lock().unwrap();
				let db = db.lock().unwrap();
				let mut details = String::new();
				if let Some(f) = db.known_fertilizers.get(fertilizer.as_str()) {
					for c in f.components_percentage(&known_elts).iter() {
						details.push_str(&format!("Element: {} = {:.2}%", c.element.name, c.concentration * 100.0));
						for a in c.aliases.iter() {
							details.push_str(&format!(" as {}: {:.2}%", a.element_alias, a.concentration * 100.0));
						}
						details.push('\n');
					}
					app.set_result_summary(format!("Fertilizer: {}", f.name()).into());
					app.set_result_details(details.into());
				} else if let Ok(compound) = fert_calc::compound::Compound::new(fertilizer.as_str(), &known_elts) {
					for c in compound.components_percentage(&known_elts).iter() {
						details.push_str(&format!("Element: {} = {:.2}%", c.element.name, c.concentration * 100.0));
						for a in c.aliases.iter() {
							details.push_str(&format!(" as {}: {:.2}%", a.element_alias, a.concentration * 100.0));
						}
						details.push('\n');
					}
					app.set_result_summary(format!("Compound: {}", compound.name()).into());
					app.set_result_details(details.into());
				} else {
					app.set_result_summary("Unknown fertilizer/compound".into());
					app.set_result_details("".into());
				}
			},
			Err(e) => {
				app.set_result_summary(format!("Error: {}", e).into());
				app.set_result_details("".into());
			}
		}
	});

	// no explicit filter handler needed; auto-apply below

	// auto-apply filter on typing via periodic check
	{
		let weak_for_filter = app.as_weak();
		let last = std::rc::Rc::new(std::cell::RefCell::new(String::new()));
		let last_cl = last.clone();
		let timer = slint::Timer::default();
		timer.start(
			slint::TimerMode::Repeated,
			std::time::Duration::from_millis(300),
			move || {
				if let Some(app) = weak_for_filter.upgrade() {
					let cur = app.get_filter().to_string();
					let mut last_val = last_cl.borrow_mut();
					if *last_val != cur {
						*last_val = cur.clone();
						if let Ok((_, db)) = load_databases() {
							let db = db.lock().unwrap();
							let needle = cur.to_lowercase();
							let mut list: Vec<slint::SharedString> = db
								.known_fertilizers
								.keys()
								.filter(|k| k.to_lowercase().contains(&needle))
								.map(|k| slint::SharedString::from(k.as_str()))
								.collect();
							list.sort();
							app.set_filtered_fertilizers(slint::ModelRc::new(slint::VecModel::from(list)));
						}
					}
				}
			},
		);
		// keep timer alive until the end of main
		std::mem::forget(timer);
	}

	app.run()?;
	Ok(())
}


