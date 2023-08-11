use anyhow::{anyhow, Result};
use std::{
	collections::HashMap,
	fmt::{Display, Formatter},
};

use itertools::Itertools;
use rustyline::Helper;

use crate::{
	compound::Compound,
	concentration::{ElementConcentrationAlias, ElementsConcentrationsWithAliases},
	elements::{Element, KnownElements},
	traits::Editor,
	Fertilizer,
};

/// Represents a pre-mixed set of elements
#[derive(Default, Clone)]
pub struct MixedFertilizer {
	/// Elements and their percentage in a mix
	pub elements_composition: HashMap<Element, f64>,
	/// Public name of the mix
	pub name: String,
	/// Description of the fertilizer
	pub description: String,
}

impl Display for MixedFertilizer {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", &self.name)
	}
}

impl PartialEq for MixedFertilizer {
	fn eq(&self, other: &Self) -> bool {
		self.name == other.name
	}
}

impl Eq for MixedFertilizer {}

/// Macro elements composition from the fertilizer declaration
#[derive(Default)]
pub struct MacroElements {
	pub nitrogen_percentage: f64,
	pub p2o5_percentage: f64,
	pub k2o_percentage: f64,
	pub mgo_percentage: f64,
}

// Check if all elements are sane for input of the mixed fertilizer
fn is_sane_elements(known_elements: &KnownElements) -> Result<()> {
	let _n = known_elements
		.elements
		.get("N")
		.ok_or_else(|| anyhow!("missing nitrogen in known elements"))?;
	let p = known_elements
		.elements
		.get("P")
		.ok_or_else(|| anyhow!("missing phosphorus in known elements"))?;
	p.aliases
		.as_ref()
		.ok_or_else(|| anyhow!("missing aliases for P"))?
		.iter()
		.position(|e| e == "P2O5")
		.ok_or_else(|| anyhow!("missing P2O5 alias in known elements"))?;
	let k = known_elements
		.elements
		.get("K")
		.ok_or_else(|| anyhow!("missing potassium in known elements"))?;
	k.aliases
		.as_ref()
		.ok_or_else(|| anyhow!("missing aliases for K"))?
		.iter()
		.position(|e| e == "K2O")
		.ok_or_else(|| anyhow!("missing K2O alias in known elements"))?;
	let mg = known_elements
		.elements
		.get("Mg")
		.ok_or_else(|| anyhow!("missing magnesium in known elements"))?;
	mg.aliases
		.as_ref()
		.ok_or_else(|| anyhow!("missing aliases for Mg"))?
		.iter()
		.position(|e| e == "MgO")
		.ok_or_else(|| anyhow!("missing MgO alias in known elements"))?;
	Ok(())
}

impl MacroElements {
	// Creates a trivial name from percentage
	pub fn name_from_npk(&self) -> String {
		if self.mgo_percentage > f64::EPSILON {
			format!(
				"NPK+Mg-{:0}:{:0}:{:0}+{:0}",
				self.nitrogen_percentage, self.p2o5_percentage, self.k2o_percentage, self.mgo_percentage
			)
		} else {
			format!("NPK-{:0}:{:0}:{:0}", self.nitrogen_percentage, self.p2o5_percentage, self.k2o_percentage)
		}
	}
}

impl MixedFertilizer {
	// Push concentrations from macro elements in fetilizer declaration
	fn push_macro_elements(&mut self, macros: &MacroElements, known_elements: &KnownElements) {
		if macros.nitrogen_percentage > f64::EPSILON {
			self.elements_composition
				.insert(known_elements.elements.get("N").unwrap().clone(), macros.nitrogen_percentage / 100.0);
		}
		if macros.p2o5_percentage > f64::EPSILON {
			let p = known_elements.elements.get("P").unwrap();
			self.elements_composition.insert(
				p.clone(),
				p.element_from_alias_rate("P2O5", known_elements).unwrap() * macros.p2o5_percentage / 100.0,
			);
		}
		if macros.k2o_percentage > f64::EPSILON {
			let k = known_elements.elements.get("K").unwrap();
			self.elements_composition.insert(
				k.clone(),
				k.element_from_alias_rate("K2O", known_elements).unwrap() * macros.k2o_percentage / 100.0,
			);
		}
		if macros.mgo_percentage > f64::EPSILON {
			let mg = known_elements.elements.get("Mg").unwrap();
			self.elements_composition.insert(
				mg.clone(),
				mg.element_from_alias_rate("MgO", known_elements).unwrap() * macros.mgo_percentage / 100.0,
			);
		}
	}
	/// Parses a mixed fertilizer from stdin
	pub fn new_from_stdin<T: Helper>(known_elements: &KnownElements, editor: &mut Editor<T>) -> Result<Self> {
		is_sane_elements(known_elements)?;

		let mut macros: MacroElements = Default::default();

		let input: String = editor.readline("Input total N in percents: ")?;
		macros.nitrogen_percentage = input.parse::<f64>()?;
		let input: String = editor.readline("Input total P2O5 in percents: ")?;
		macros.p2o5_percentage = input.parse::<f64>()?;
		let input: String = editor.readline("Input total K2O in percents: ")?;
		macros.k2o_percentage = input.parse::<f64>()?;
		let input: String = editor.readline("Input total MgO in percents: ")?;
		macros.mgo_percentage = input.parse::<f64>()?;

		let mut res = Self { name: macros.name_from_npk(), ..Default::default() };

		res.push_macro_elements(&macros, known_elements);

		Ok(res)
	}

	// Used for tests currently but might be used for something else
	#[allow(dead_code)]
	pub fn new_from_npk(macros: &MacroElements, known_elements: &KnownElements) -> Result<Self> {
		is_sane_elements(known_elements)?;

		let mut res = Self { name: macros.name_from_npk(), ..Default::default() };
		res.push_macro_elements(macros, known_elements);

		Ok(res)
	}

	/// Parse a mixed fertilizer from a toml object
	pub fn new_from_toml_object(
		name: &str,
		obj: &toml::Value,
		known_elements: &KnownElements,
		is_percents: bool,
	) -> Result<Self> {
		if !obj.is_table() {
			return Err(anyhow!("expect input to be a toml object"))
		}

		let compounds = obj
			.as_table()
			.unwrap()
			.get("compounds")
			.ok_or_else(|| anyhow!("no `compounds` object in a mix"))?
			.as_table()
			.ok_or_else(|| anyhow!("expect compounds to be an object"))?;

		let description = if let Some(descr_obj) = obj.as_table().unwrap().get("description") {
			descr_obj.as_str().unwrap_or("")
		} else {
			""
		};

		let mut res = Self { name: name.to_owned(), description: description.to_owned(), ..Default::default() };

		// Ineffective, but who cares
		if !compounds
			.iter()
			.all(|(k, v)| (v.is_integer() || v.is_float()) && Compound::new(k.as_str(), known_elements).is_ok())
		{
			return Err(anyhow!("incorrect compounds definition"))
		}

		let compounds_portions = compounds
			.iter()
			.filter(|(_, v)| (v.is_integer() || v.is_float()))
			.map(|(k, v)| {
				(
					Compound::new(k.as_str(), known_elements)
						.unwrap()
						.components_percentage(known_elements),
					if is_percents { extract_toml_number(v) / 100.0 } else { extract_toml_number(v) },
				)
			});

		compounds_portions.for_each(|(elements_percentages, portion)| {
			elements_percentages
				.iter()
				.filter(|e| !e.element.is_insignificant())
				.for_each(|elt_percentage| {
					*res.elements_composition.entry(elt_percentage.element.clone()).or_default() +=
						elt_percentage.concentration * portion;
				})
		});

		Ok(res)
	}
}

impl Fertilizer for MixedFertilizer {
	/// Returns elements percentage for all elements except unimportant
	fn components_percentage(&self, known_elts: &KnownElements) -> Vec<ElementsConcentrationsWithAliases> {
		self.elements_composition
			.iter()
			.filter(|(element, _)| !element.is_insignificant())
			.map(|(element, fraction)| {
				let aliases: Vec<ElementConcentrationAlias> = element.aliases.as_ref().map_or(Vec::new(), |aliases| {
					aliases
						.iter()
						.map(|alias| ElementConcentrationAlias {
							element_alias: alias.clone(),
							concentration: *fraction *
								element.element_to_alias_rate(alias.as_str(), known_elts).unwrap(),
						})
						.collect::<Vec<_>>()
				});

				ElementsConcentrationsWithAliases { element: element.clone(), concentration: *fraction, aliases }
			})
			.sorted()
			.collect::<Vec<_>>()
	}

	fn name(&self) -> &str {
		self.name.as_str()
	}

	fn description(&self) -> String {
		self.description.clone()
	}
}

fn extract_toml_number(val: &toml::Value) -> f64 {
	match *val {
		toml::Value::Float(f) => f,
		toml::Value::Integer(i) => i as f64,
		_ => panic!("must not be reached"),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{assert_delta_eq, test_utils::*};

	// Compare fertilizer declaration for miracle gro fertilizer
	#[test]
	fn miracle_gro() {
		let known_elements = load_known_elements();
		let fert = MixedFertilizer::new_from_npk(
			&MacroElements {
				nitrogen_percentage: 24.0,
				p2o5_percentage: 8.0,
				k2o_percentage: 16.0,
				..Default::default()
			},
			&known_elements,
		)
		.unwrap();
		assert_eq!(fert.name(), "NPK-24:8:16");
		let percentages = fert.components_percentage(&known_elements);
		assert_eq!(percentages[0].element.name, "N");
		assert_delta_eq!(percentages[0].concentration, 24.0 / 100.0, MOLAR_MASS_EPSILON);
		assert_eq!(percentages[1].element.name, "P");
		assert_delta_eq!(percentages[1].concentration, 3.5 / 100.0, MOLAR_MASS_EPSILON);
		assert_eq!(percentages[2].element.name, "K");
		assert_delta_eq!(percentages[2].concentration, 13.3 / 100.0, MOLAR_MASS_EPSILON);
	}

	// Compare fertilizer declaration for chempak tomato fertilizer
	#[test]
	fn chempak_tomato() {
		let known_elements = load_known_elements();
		let fert = MixedFertilizer::new_from_npk(
			&MacroElements {
				nitrogen_percentage: 11.0,
				p2o5_percentage: 9.0,
				k2o_percentage: 30.0,
				mgo_percentage: 2.5,
			},
			&known_elements,
		)
		.unwrap();
		assert_eq!(fert.name(), "NPK+Mg-11:9:30+2.5");
		let percentages = fert.components_percentage(&known_elements);
		assert_eq!(percentages[0].element.name, "N");
		assert_delta_eq!(percentages[0].concentration, 11.0 / 100.0, MOLAR_MASS_EPSILON);
		assert_eq!(percentages[1].element.name, "P");
		assert_delta_eq!(percentages[1].concentration, 3.9 / 100.0, MOLAR_MASS_EPSILON);
		assert_eq!(percentages[2].element.name, "K");
		assert_delta_eq!(percentages[2].concentration, 24.9 / 100.0, MOLAR_MASS_EPSILON);
		assert_eq!(percentages[3].element.name, "Mg");
		assert_delta_eq!(percentages[3].concentration, 1.5 / 100.0, MOLAR_MASS_EPSILON);
	}
}
