use anyhow::Result;
use serde::Deserialize;
use std::{
	collections::HashMap,
	fmt::Debug,
	fs,
	hash::{Hash, Hasher},
	path::Path,
};

/// A primitive element (not necessarily simple)
#[derive(Debug, Deserialize, Clone)]
pub struct Element {
	pub molar_mass: f64,
	pub name: String,
	pub insignificant: Option<bool>,
	pub aliases: Option<Vec<String>>,
}

impl Element {
	pub fn is_insignificant(&self) -> bool {
		self.insignificant.unwrap_or(false)
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
