use crate::{
	concentration::{ElementConcentrationAlias, ElementsConcentrationsWithAliases},
	elements::*,
	traits::Fertilizer,
};
use accurate::{sum::Sum2, traits::*};
use anyhow::{anyhow, Result};
use rustyline::Editor;

use itertools::Itertools;
use std::{
	collections::HashMap,
	fmt::{Debug, Display, Formatter},
};

/// A structure that represents a molecule of some compound
#[derive(Debug, Default, Clone)]
pub struct Compound {
	/// Elements in the compound and their quantity (in atoms)
	pub elements: HashMap<Element, u32>,
	/// Name of the compound (e.g. a trivial formula)
	pub name: String,
}

impl Display for Compound {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "{}", &self.name)
	}
}

impl PartialEq for Compound {
	fn eq(&self, other: &Self) -> bool {
		self.name == other.name
	}
}

impl Eq for Compound {}

impl Compound {
	fn process_acc<'a>(&mut self, acc: &str, cnt: u32, known_elts: &'a KnownElements) -> Result<&'a Element> {
		let maybe_elt = known_elts.elements.get(acc);

		if let Some(elt) = maybe_elt {
			*self.elements.entry(elt.clone()).or_default() += cnt;
			return Ok(elt)
		}

		Err(anyhow!("Unknown element: {}", acc))
	}

	fn process_trail(
		&mut self,
		last_cnt: Option<u32>,
		last_element: Option<&Element>,
		last_subcompound: &Option<Compound>,
	) -> Result<bool> {
		if let Some(cnt) = last_cnt {
			return if let Some(subcompound) = last_subcompound {
				subcompound.elements.iter().for_each(|(elt, sub_cnt)| {
					*self.elements.entry(elt.clone()).or_default() += sub_cnt * cnt;
				});
				Ok(true)
			} else if let Some(last_elt) = last_element {
				*self.elements.entry(last_elt.clone()).or_default() += cnt - 1;
				Ok(true)
			} else {
				Err(anyhow!("digit without element found"))
			}
		} else if let Some(subcompound) = last_subcompound {
			subcompound.elements.iter().for_each(|(elt, sub_cnt)| {
				*self.elements.entry(elt.clone()).or_default() += sub_cnt;
			});
			return Ok(true)
		}

		Ok(false)
	}

	fn new_hydrate(formula: &str, known_elts: &KnownElements) -> Result<Self> {

		if formula.len() < 2 {
			return Err(anyhow!("Invalid hydrate formula: {}", formula))
		}

		let first_c = formula.chars().next().expect("checked len above; qed.");
		let mult = if first_c.is_ascii_digit() {
			first_c.to_digit(10).expect("checked above; qed.")
		}
		else {
			1
		};

		let remain = if first_c.is_ascii_digit() {
			&formula[1..]
		}
		else {
			formula
		};

		let mut compound = Compound::new(remain, known_elts)?;
		compound.elements.values_mut().for_each(|v| *v *= mult);
		Ok(compound)
	}

	/// Parses formula from a trivial string knowing some elements
	pub fn new(formula: &str, known_elts: &KnownElements) -> Result<Self> {
		let mut acc = String::new();
		let mut new_compound: Self = Default::default();
		let mut last_element: Option<&Element> = Default::default();
		let mut last_subcompound: Option<Compound> = Default::default();
		let mut last_cnt: Option<u32> = Default::default();
		let mut obraces = 0;
		let mut ebraces = 0;
		new_compound.name = formula.to_owned();

		for (pos,chr) in formula.chars().enumerate() {
			if obraces > 0 {
				if chr == ')' {
					ebraces += 1;
				} else if chr == '(' {
					obraces += 1;
				}

				if ebraces != obraces {
					acc.push(chr);
				} else {
					// Here, acc has the whole matching sub-compound
					last_subcompound = Some(Compound::new(acc.as_str(), known_elts)?);
					obraces = 0;
					ebraces = 0;
					acc.clear();
				}
			} else if chr.is_ascii_uppercase() {
				if new_compound.process_trail(last_cnt, last_element, &last_subcompound)? {
					last_element = None;
					last_cnt = None;
					last_subcompound = None;
				}

				// Previous element
				if !acc.is_empty() {
					let elt = new_compound.process_acc(acc.as_str(), 1, known_elts)?;
					last_element = Some(elt);
					acc.clear();
				}

				acc.push(chr);
			} else if chr.is_lowercase() {
				// Lowercase is always end of the element name
				acc.push(chr);
			} else if chr.is_ascii_digit() {
				if last_subcompound.is_none() && !acc.is_empty() {
					// Process leftover
					let elt = new_compound.process_acc(acc.as_str(), 1, known_elts)?;
					last_element = Some(elt);
					acc.clear();
				}
				let cnt = chr.to_digit(10).unwrap();

				last_cnt = match last_cnt {
					Some(x) => Some(x * 10 + cnt),
					_ => Some(cnt),
				};
			} else if chr == '(' {
				if !new_compound.process_trail(last_cnt, last_element, &last_subcompound)? && !acc.is_empty() {
					new_compound.process_acc(acc.as_str(), 1, known_elts)?;
				}
				acc.clear();
				last_element = None;
				last_cnt = None;
				last_subcompound = None;
				obraces += 1;
			} else if chr == '*' {
				// Hydrate addition
				let hydrate = Compound::new_hydrate(&formula[pos+1..], known_elts)?;
				// Add hydrate definition to the original formula, as we need that
				// to calculate molecular mass
				hydrate.elements.iter().for_each(|(elt, cnt)| {
					*new_compound.elements.entry(elt.clone()).or_default() += cnt;
				});
			} else {
				// Ignore garbage stuff
			}
		}

		// Process trail
		if !new_compound.process_trail(last_cnt, last_element, &last_subcompound)? && !acc.is_empty() {
			new_compound.process_acc(acc.as_str(), 1, known_elts)?;
		}

		if new_compound.elements.is_empty() {
			return Err(anyhow!("Empty compound"))
		}

		Ok(new_compound)
	}

	/// Returns a compound from stdin
	pub fn new_from_stdin<T: rustyline::Helper>(known_elts: &KnownElements, editor: &mut Editor<T>) -> Result<Self> {
		let input_compound: String = editor.readline("Input compound (e.g. KNO3): ")?;
		Compound::new(input_compound.as_str(), known_elts)
	}

	/// Returns a molar mass for the compound
	pub fn molar_mass(&self) -> f64 {
		self.elements
			.iter()
			.fold(Sum2::zero(), |acc, (elt, cnt)| acc + elt.molar_mass * (*cnt as f64))
			.sum()
	}

	/// Returns percentage for a specific element
	pub fn element_fraction(&self, element: &Element) -> Option<f64> {
		let molar_mass = self.molar_mass();

		self.elements
			.get(element)
			.map(|elt_cnt| element.molar_mass * (*elt_cnt as f64) / molar_mass)
	}
}

impl Fertilizer for Compound {
	/// Returns elements percentage for all elements except unimportant
	fn components_percentage(&self, known_elts: &KnownElements) -> Vec<ElementsConcentrationsWithAliases> {
		let molar_mass = self.molar_mass();

		self.elements
			.iter()
			.filter(|(element, _)| !element.is_insignificant())
			.map(|(element, cnt)| {
				let percentage = element.molar_mass * (*cnt as f64) / molar_mass;

				let aliases: Vec<ElementConcentrationAlias> = element.aliases.as_ref().map_or(Vec::new(), |aliases| {
					aliases
						.iter()
						.map(|alias| ElementConcentrationAlias {
							element_alias: alias.clone(),
							concentration: percentage *
								element.element_to_alias_rate(alias.as_str(), known_elts).unwrap(),
						})
						.collect::<Vec<_>>()
				});

				ElementsConcentrationsWithAliases { element: element.clone(), concentration: percentage, aliases }
			})
			.sorted()
			.collect::<Vec<_>>()
	}

	fn name(&self) -> &str {
		self.name.as_str()
	}
	fn description(&self) -> String {
		format!("Compound: {}", self.name)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{assert_delta_eq, test_utils::*};

	#[test]
	fn parse_simple() {
		let known_elements = load_known_elements();
		let kno3 = Compound::new("KNO3", &known_elements);
		assert_delta_eq!(kno3.as_ref().unwrap().molar_mass(), 101.1032, MOLAR_MASS_EPSILON);
		let kh2po4 = Compound::new("KH2PO4", &known_elements);
		assert_delta_eq!(kh2po4.as_ref().unwrap().molar_mass(), 136.084, MOLAR_MASS_EPSILON);
		let k2h100 = Compound::new("K2H100", &known_elements);
		assert_delta_eq!(k2h100.as_ref().unwrap().molar_mass(), 178.977, MOLAR_MASS_EPSILON);
		let k = Compound::new("K", &known_elements);
		assert_delta_eq!(k.as_ref().unwrap().molar_mass(), 39.098, MOLAR_MASS_EPSILON);
	}

	#[test]
	fn parse_invalid() {
		let known_elements = load_known_elements();
		let invalid = Compound::new("2KO", &known_elements);
		assert!(invalid.is_err());
		let invalid = Compound::new("Ololo", &known_elements);
		assert!(invalid.is_err());
		let invalid = Compound::new("(((Ca(((", &known_elements);
		assert!(invalid.is_err());
	}

	#[test]
	fn parse_braced() {
		let known_elements = load_known_elements();
		let cano3 = Compound::new("Ca(NO3)2", &known_elements);
		assert_delta_eq!(cano3.as_ref().unwrap().molar_mass(), 164.086, MOLAR_MASS_EPSILON);
		let cano3 = Compound::new("(Ca)(NO3)2", &known_elements);
		assert_delta_eq!(cano3.as_ref().unwrap().molar_mass(), 164.086, MOLAR_MASS_EPSILON);
		let cano3 = Compound::new("(Ca)1(NO3)2", &known_elements);
		assert_delta_eq!(cano3.as_ref().unwrap().molar_mass(), 164.086, MOLAR_MASS_EPSILON);
		let braces = Compound::new("(((Ca)))", &known_elements);
		assert_delta_eq!(braces.as_ref().unwrap().molar_mass(), 40.078, MOLAR_MASS_EPSILON);
		let braces = Compound::new("(((Ca)))2", &known_elements);
		assert_delta_eq!(braces.as_ref().unwrap().molar_mass(), 40.078 * 2.0, MOLAR_MASS_EPSILON);
	}
}
