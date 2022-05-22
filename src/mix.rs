use anyhow::{anyhow, Result};
use std::{
	collections::HashMap,
	fmt::{Display, Formatter},
};

use dialoguer::Input;
use itertools::Itertools;

use crate::{
	compound::Compound,
	concentration::{ElementConcentrationAlias, ElementsConcentrationsWithAliases},
	elements::{Element, KnownElements},
	Fertilizer,
};

/// Represents a pre-mixed set of elements
#[derive(Default)]
pub struct MixedFertilizer {
	/// Elements and their percentage in a mix
	pub elements_composition: HashMap<Element, f64>,
	/// Public name of the mix
	pub name: String,
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
		.ok_or(anyhow!("missing nitrogen in known elements"))?;
	let p = known_elements
		.elements
		.get("P")
		.ok_or(anyhow!("missing phosphorus in known elements"))?;
	p.aliases
		.as_ref()
		.ok_or(anyhow!("missing aliases for P"))?
		.iter()
		.position(|e| e == "P2O5")
		.ok_or(anyhow!("missing P2O5 alias in known elements"))?;
	let k = known_elements
		.elements
		.get("K")
		.ok_or(anyhow!("missing potassium in known elements"))?;
	k.aliases
		.as_ref()
		.ok_or(anyhow!("missing aliases for K"))?
		.iter()
		.position(|e| e == "K2O")
		.ok_or(anyhow!("missing K2O alias in known elements"))?;
	let mg = known_elements
		.elements
		.get("Mg")
		.ok_or(anyhow!("missing magnesium in known elements"))?;
	mg.aliases
		.as_ref()
		.ok_or(anyhow!("missing aliases for Mg"))?
		.iter()
		.position(|e| e == "MgO")
		.ok_or(anyhow!("missing MgO alias in known elements"))?;
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
			self.elements_composition
				.insert(p.clone(), p.from_alias_rate("P2O5", known_elements).unwrap() * macros.p2o5_percentage / 100.0);
		}
		if macros.k2o_percentage > f64::EPSILON {
			let k = known_elements.elements.get("K").unwrap();
			self.elements_composition
				.insert(k.clone(), k.from_alias_rate("K2O", known_elements).unwrap() * macros.k2o_percentage / 100.0);
		}
		if macros.mgo_percentage > f64::EPSILON {
			let mg = known_elements.elements.get("Mg").unwrap();
			self.elements_composition
				.insert(mg.clone(), mg.from_alias_rate("MgO", known_elements).unwrap() * macros.mgo_percentage / 100.0);
		}
	}
	/// Parses a mixed fertilizer from stdin
	pub fn new_from_stdin(known_elements: &KnownElements) -> Result<Self> {
		is_sane_elements(known_elements)?;

		let mut macros: MacroElements = Default::default();

		let input: String = Input::new().with_prompt("Input total N in percents").interact_text()?;
		macros.nitrogen_percentage = input.parse::<f64>()?;
		let input: String = Input::new().with_prompt("Input total P2O5 in percents").interact_text()?;
		macros.p2o5_percentage = input.parse::<f64>()?;
		let input: String = Input::new().with_prompt("Input total K2O in percents").interact_text()?;
		macros.k2o_percentage = input.parse::<f64>()?;
		let input: String = Input::new().with_prompt("Input total MgO in percents").interact_text()?;
		macros.mgo_percentage = input.parse::<f64>()?;

		let mut res: Self = Default::default();

		res.name = macros.name_from_npk();
		res.push_macro_elements(&macros, known_elements);

		Ok(res)
	}

	// Used for tests currently but might be used for something else
	#[allow(dead_code)]
	pub fn new_from_npk(macros: &MacroElements, known_elements: &KnownElements) -> Result<Self> {
		is_sane_elements(known_elements)?;

		let mut res: Self = Default::default();
		res.name = macros.name_from_npk();
		res.push_macro_elements(&macros, known_elements);

		Ok(res)
	}

	/// Parse a mixed fertilizer from a json object
	pub fn new_from_json_object(name: &str, obj: &serde_json::Value, known_elements: &KnownElements) -> Result<Self> {
		if !obj.is_object() {
			return Err(anyhow!("expect input to be a json object"))
		}

		let compounds = obj
			.as_object()
			.unwrap()
			.get("compounds")
			.ok_or(anyhow!("no `compounds` object in a mix"))?
			.as_object()
			.ok_or(anyhow!("expect compounds to be an object"))?;

		let mut res: Self = Default::default();
		res.name = name.to_owned();

		// Ineffective, but who cares
		if !compounds
			.iter()
			.all(|(k, v)| v.is_number() && Compound::new(k.as_str(), known_elements).is_ok())
		{
			return Err(anyhow!("incorrect compounds definition"))
		}

		let compounds_portions = compounds.iter().filter(|(_, v)| v.is_number()).map(|(k, v)| {
			(
				Compound::new(k.as_str(), known_elements)
					.unwrap()
					.components_percentage(known_elements),
				v.as_f64().unwrap(),
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
			.filter(|(element, _)| return !element.is_insignificant())
			.map(|(element, fraction)| {
				let aliases: Vec<ElementConcentrationAlias> = element.aliases.as_ref().map_or(Vec::new(), |aliases| {
					aliases
						.iter()
						.map(|alias| ElementConcentrationAlias {
							element_alias: alias.clone(),
							concentration: *fraction * element.to_alias_rate(alias.as_str(), known_elts).unwrap(),
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
				..Default::default()
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
