use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::{
	cmp::Ordering,
	collections::HashMap,
	fmt::Debug,
	fs,
	hash::{Hash, Hasher},
	path::Path,
};

use crate::compound::Compound;

/// A primitive element (not necessarily simple)
#[derive(Debug, Deserialize, Clone)]
pub struct Element {
	pub molar_mass: f64,
	pub name: String,
	pub insignificant: Option<bool>,
	pub priority: Option<u32>,
	pub aliases: Option<Vec<String>>,
}

impl Element {
	pub fn is_insignificant(&self) -> bool {
		self.insignificant.unwrap_or(false)
	}
	pub fn priority(&self) -> u32 {
		self.priority.unwrap_or(0)
	}
}

impl Hash for Element {
	fn hash<H: Hasher>(&self, state: &mut H) {
		self.name.hash(state)
	}
}

impl PartialEq for Element {
	fn eq(&self, other: &Self) -> bool {
		self.name == other.name
	}
}

impl Eq for Element {}

impl PartialOrd for Element {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		Some(self.cmp(&other))
	}
}

impl Ord for Element {
	fn cmp(&self, other: &Self) -> Ordering {
		if self.priority() == other.priority() {
			self.name.cmp(&&other.name)
		} else {
			self.priority().cmp(&&other.priority()).reverse()
		}
	}
}

/// Defines static knowledge of all elements we are interested in
pub struct KnownElements {
	pub elements: HashMap<String, Element>,
}

impl KnownElements {
	/// Creates and fill all elements (presumably this should live in a separate TOML file
	/// but will be in the code for now (for simplicity purposes)
	pub fn new_with_db(database: &Path) -> Result<Self> {
		let data = fs::read_to_string(database)?;

		KnownElements::new_with_string(data.as_str())
	}

	pub fn new_with_string(input: &str) -> Result<Self> {
		let json: Vec<Element> = serde_json::from_str(&input)?;

		Ok(Self { elements: HashMap::from_iter(json.into_iter().map(|e| (e.name.clone(), e))) })
	}
}

impl Element {
	/// Returns element rate to alias rate where an alias parsed from a string
	pub fn from_alias_rate(&self, alias: &str, known_elts: &KnownElements) -> Result<f64> {
		let molecule = Compound::new(alias, known_elts)?;
		molecule.element_fraction(self).ok_or(anyhow!("invalid alias: {}", alias))
	}
	/// Returns alias rate to specific element rate where an alias is parsed from a string
	pub fn to_alias_rate(&self, alias: &str, known_elts: &KnownElements) -> Result<f64> {
		self.from_alias_rate(alias, known_elts).map(|rate| 1.0 / rate)
	}
}
