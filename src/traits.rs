use crate::{
	concentration::{DiluteCalcType, DiluteResult, ElementsConcentrationsWithAliases},
	elements::KnownElements,
	tank::Tank,
};
use anyhow::Result;
use dyn_clone::DynClone;
use rustyline::{Editor, Helper};

/// A generic representation of the fertilizer, must return components percentage for the fertilizer
pub trait Fertilizer: DynClone {
	fn components_percentage(&self, known_elts: &KnownElements) -> Vec<ElementsConcentrationsWithAliases>;
	fn name(&self) -> &str;
}

/// Represents a concentration after adding some fertilizer to the specific tank
pub trait DiluteMethod {
	/// Load dilute method from stdin
	fn new_from_stdin<T: Helper>(
		what: DiluteCalcType,
		known_elements: &KnownElements,
		editor: &mut Editor<T>,
	) -> Result<Self>
	where
		Self: Sized;
	/// Deserialize dilute method from TOML
	fn new_from_toml(toml: &str) -> Result<Self>
	where
		Self: Sized;
	/// Deserialize dilute method from JSON
	fn new_from_json(json: &str) -> Result<Self>
	where
		Self: Sized;
	/// Dilute fertilizer in a specific tank using known dilute method
	fn dilute(&self, fertilizer: &dyn Fertilizer, known_elements: &KnownElements, tank: &Tank) -> Result<DiluteResult>;
}

dyn_clone::clone_trait_object!(Fertilizer);
