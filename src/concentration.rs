use crate::{compound::Compound, elements::*, tank::Tank, traits::DiluteMethod, Fertilizer};
use anyhow::{anyhow, Result};
use crossterm::style::Stylize;
use itertools::Itertools;
use rustyline::{Editor, Helper};
use serde::{Deserialize, Serialize};
use std::{
	cmp::Ordering,
	fmt::{Debug, Formatter},
};
use strum::EnumString;

/// How do we calculate dilution
#[derive(Deserialize, Clone, Copy, Debug, EnumString)]
pub enum DiluteCalcType {
	ResultOfDose,
	TargetDose,
}

impl Default for DiluteCalcType {
	fn default() -> Self {
		DiluteCalcType::ResultOfDose
	}
}

/// Element name and it's concentration
#[derive(Serialize, Clone)]
pub struct ElementConcentrationAlias {
	pub element_alias: String,
	pub concentration: f64,
}
/// Concentration of the all elements in a fertilizer with all elements' aliases
#[derive(Serialize, Clone)]
pub struct ElementsConcentrationsWithAliases {
	pub element: Element,
	pub concentration: f64,
	pub aliases: Vec<ElementConcentrationAlias>,
}

impl Debug for ElementsConcentrationsWithAliases {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "Element: {} = {:.2}%", self.element.name.clone().bold(), self.concentration * 100.0)?;

		for alias in self.aliases.iter() {
			write!(f, " as {}: {:.2}%", alias.element_alias.clone().bold(), alias.concentration * 100.0)?;
		}

		Ok(())
	}
}

/// Element name and it's dose
#[derive(Serialize, Clone)]
pub struct ElementAliasDose {
	pub element_alias: String,
	pub dose: f64,
}
/// Dosing of the all elements in a fertilizer with all elements' aliases
#[derive(Serialize, Clone)]
pub struct ElementsDosesWithAliases {
	pub element: Element,
	pub dose: f64,
	pub aliases: Vec<ElementAliasDose>,
}

// Helpers to output and sort structures
impl Debug for ElementsDosesWithAliases {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		let adjust_units = |dose: f64| if dose <= 0.01 { (dose * 1000.0, "ug") } else { (dose, "mg") };
		let (dose, units) = adjust_units(self.dose);
		write!(f, "Element: {} = {:.3} {}/l", self.element.name.clone().bold(), dose, units)?;

		for alias in self.aliases.iter() {
			let (dose, units) = adjust_units(alias.dose);
			write!(f, " as {}: {:.3} {}/l", alias.element_alias.clone().bold(), dose, units)?;
		}

		Ok(())
	}
}

impl PartialOrd for ElementsConcentrationsWithAliases {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.element.cmp(&other.element))
	}
}

impl Ord for ElementsConcentrationsWithAliases {
	fn cmp(&self, other: &Self) -> Ordering {
		self.element.cmp(&other.element)
	}
}

impl PartialEq for ElementsConcentrationsWithAliases {
	fn eq(&self, other: &Self) -> bool {
		self.element == other.element
	}
}

impl Eq for ElementsConcentrationsWithAliases {}

impl PartialOrd for ElementsDosesWithAliases {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.element.cmp(&other.element))
	}
}

impl Ord for ElementsDosesWithAliases {
	fn cmp(&self, other: &Self) -> Ordering {
		self.element.cmp(&other.element)
	}
}

impl PartialEq for ElementsDosesWithAliases {
	fn eq(&self, other: &Self) -> bool {
		self.element == other.element
	}
}

impl Eq for ElementsDosesWithAliases {}

#[derive(Serialize, Clone)]
pub struct DiluteResult {
	pub compound_dose: f64,
	pub elements_dose: Vec<ElementsDosesWithAliases>,
}

fn get_element_dose_target<T: Helper>(known_elements: &KnownElements, editor: &mut Editor<T>) -> Result<(String, f64)> {
	let input: String = editor.readline("Input target element or compound (e.g. NO3 or N): ")?;
	let compound = Compound::new(input.as_str(), known_elements)?;
	let concentrations = compound.components_percentage(known_elements);
	let top_elt = concentrations[0].element.name.clone();
	let input: String = editor.readline("Input target element concentration (mg/l): ")?;
	let target = input.parse::<f64>()?;
	Ok((top_elt, target * concentrations[0].concentration))
}

fn dilute_fertilizer(
	concentrations: Vec<ElementsConcentrationsWithAliases>,
	mult: f64,
) -> Vec<ElementsDosesWithAliases> {
	concentrations
		.into_iter()
		.map(|elt_conc| {
			let aliases = elt_conc
				.aliases
				.iter()
				.map(|alias| ElementAliasDose {
					element_alias: alias.element_alias.clone(),
					dose: alias.concentration * mult,
				})
				.collect::<Vec<_>>();
			ElementsDosesWithAliases { element: elt_conc.element.clone(), dose: elt_conc.concentration * mult, aliases }
		})
		.sorted()
		.collect::<Vec<_>>()
}

/// A concrete implementation of the dosing with the value in grams
#[derive(Default, Debug, Deserialize, Clone)]
pub struct DryDosing {
	dilute_input: f64,
	what: DiluteCalcType,
	target_element: Option<String>,
}

impl DiluteMethod for DryDosing {
	fn new_from_stdin<T: Helper>(
		what: DiluteCalcType,
		known_elements: &KnownElements,
		editor: &mut Editor<T>,
	) -> Result<Self> {
		match what {
			DiluteCalcType::ResultOfDose => {
				let input: String = editor.readline("Dose size in grams (e.g. 2.5): ")?;
				let dose = input.parse::<f64>()?;
				Ok(Self { dilute_input: dose, what, ..Default::default() })
			},
			DiluteCalcType::TargetDose => {
				let (target_element, dilute_input) = get_element_dose_target(known_elements, editor)?;
				Ok(Self { dilute_input, what, target_element: Some(target_element) })
			},
		}
	}

	fn new_from_toml(toml: &str) -> Result<Self> {
		let res: Self = toml::from_str(toml)?;
		Ok(res)
	}
	fn new_from_json(json: &str) -> Result<Self> {
		let res: Self = serde_json::from_str(json)?;
		Ok(res)
	}

	fn dilute(&self, fertilizer: &dyn Fertilizer, known_elements: &KnownElements, tank: &Tank) -> Result<DiluteResult> {
		let concentrations = fertilizer.components_percentage(known_elements);
		let mult = match self.what {
			DiluteCalcType::ResultOfDose => self.dilute_input * 1000.0 / tank.effective_volume() as f64,
			DiluteCalcType::TargetDose => {
				// Get target element concentration
				let target_elt = self
					.target_element
					.as_ref()
					.ok_or_else(|| anyhow!("no target element defined"))?
					.as_str();
				let fert_elt = concentrations
					.get(
						concentrations
							.iter()
							.position(|elt| elt.element.name == target_elt)
							.ok_or_else(|| anyhow!("target element {} is not in the fertilizer", target_elt))?,
					)
					.unwrap();
				self.dilute_input / fert_elt.concentration
			},
		};
		// For dry dosing we simply dilute all components by a tank's effective volume
		let concentrations = dilute_fertilizer(concentrations, mult);
		Ok(DiluteResult {
			compound_dose: mult * tank.effective_volume() as f64 / 1000.0,
			elements_dose: concentrations,
		})
	}
}

/// A concrete implementation of the dosing by dissolving dry salt in a concentrated solution
#[derive(Default, Debug, Deserialize, Clone)]
pub struct SolutionDosing {
	container_volume: f64,
	portion_volume: f64,
	dose: f64,
	what: DiluteCalcType,
	target_element: Option<String>,
}

impl DiluteMethod for SolutionDosing {
	fn new_from_stdin<T: Helper>(
		what: DiluteCalcType,
		known_elements: &KnownElements,
		editor: &mut Editor<T>,
	) -> Result<Self> {
		match what {
			DiluteCalcType::ResultOfDose => {
				let input: String = editor.readline("Container size in ml: ")?;
				let container_volume = input.parse::<f64>()?;
				let input: String = editor.readline("Portion size in ml: ")?;
				let portion_volume = input.parse::<f64>()?;
				let input: String = editor.readline("Dose size in grams (e.g. 2.5): ")?;
				let dose = input.parse::<f64>()?;
				Ok(Self { container_volume, portion_volume, dose, what, ..Default::default() })
			},
			DiluteCalcType::TargetDose => {
				let (target_element, dose) = get_element_dose_target(known_elements, editor)?;
				let input: String = editor.readline("Container size in ml: ")?;
				let container_volume = input.parse::<f64>()?;
				let input: String = editor.readline("Portion size in ml: ")?;
				let portion_volume = input.parse::<f64>()?;
				Ok(Self { container_volume, portion_volume, dose, what, target_element: Some(target_element) })
			},
		}
	}

	fn new_from_toml(toml: &str) -> Result<Self> {
		let res: Self = toml::from_str(toml)?;
		Ok(res)
	}

	fn new_from_json(toml: &str) -> Result<Self>
	where
		Self: Sized,
	{
		let res: Self = serde_json::from_str(toml)?;
		Ok(res)
	}

	fn dilute(&self, fertilizer: &dyn Fertilizer, known_elements: &KnownElements, tank: &Tank) -> Result<DiluteResult> {
		let concentrations = fertilizer.components_percentage(known_elements);
		let dose = match self.what {
			DiluteCalcType::ResultOfDose => self.dose,
			DiluteCalcType::TargetDose => {
				// Get target element concentration
				let target_elt = self
					.target_element
					.as_ref()
					.ok_or_else(|| anyhow!("no target element defined"))?
					.as_str();
				let fert_elt = concentrations
					.get(
						concentrations
							.iter()
							.position(|elt| elt.element.name == target_elt)
							.ok_or_else(|| anyhow!("target element {} is not in the fertilizer", target_elt))?,
					)
					.unwrap();
				self.dose * tank.effective_volume() as f64 / fert_elt.concentration * self.container_volume /
					self.portion_volume / 1000.0
			},
		};
		let mult = (dose * 1000.0 / self.container_volume * self.portion_volume) / tank.effective_volume() as f64;
		let concentrations = dilute_fertilizer(concentrations, mult);
		Ok(DiluteResult { compound_dose: dose, elements_dose: concentrations })
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{assert_delta_eq, compound::Compound, test_utils::*};

	#[test]
	fn test_kno3_dry() {
		let tank = sample_tank();
		let known_elts = load_known_elements();
		let compound: Box<dyn Fertilizer> = Box::new(Compound::new("KNO3", &known_elts).unwrap());
		let dosing =
			Box::new(DryDosing { dilute_input: 1.0, what: DiluteCalcType::ResultOfDose, ..Default::default() });
		let results = dosing.dilute(&*compound, &known_elts, &tank).unwrap();
		assert!(!results.elements_dose.is_empty());
		assert_eq!(results.elements_dose[0].element.name.as_str(), "N");
		assert_delta_eq!(results.elements_dose[0].dose, 0.815, MOLAR_MASS_EPSILON);
		assert_eq!(results.elements_dose[1].element.name.as_str(), "K");
		assert_delta_eq!(results.elements_dose[1].dose, 2.275, MOLAR_MASS_EPSILON);
	}

	#[test]
	fn test_kno3_solution() {
		let tank = sample_tank();
		let known_elts = load_known_elements();
		let compound: Box<dyn Fertilizer> = Box::new(Compound::new("KNO3", &known_elts).unwrap());
		let dosing = Box::new(SolutionDosing {
			dose: 10.0,
			container_volume: 1000.0,
			portion_volume: 100.0,
			what: DiluteCalcType::ResultOfDose,
			..Default::default()
		});
		let results = dosing.dilute(&*compound, &known_elts, &tank).unwrap();
		assert!(!results.elements_dose.is_empty());
		assert_eq!(results.elements_dose[0].element.name.as_str(), "N");
		assert_delta_eq!(results.elements_dose[0].dose, 0.815, MOLAR_MASS_EPSILON);
		assert_eq!(results.elements_dose[1].element.name.as_str(), "K");
		assert_delta_eq!(results.elements_dose[1].dose, 2.275, MOLAR_MASS_EPSILON);
	}
}
