use crate::{elements::KnownElements, tank::Tank, FertilizersDb};
use std::{fs, path::Path};

#[macro_export]
macro_rules! assert_delta_eq {
	($x:expr, $y:expr, $d:expr) => {
		assert!(
			($x >= $y && $x - $y < $d) || ($x < $y && $y - $x < $d),
			"assert_delta_eq!({}, {}); {:?} != {:?}",
			stringify!($x),
			stringify!($y),
			$x,
			$y
		)
	};
}

/// Used to compare results with some epsilon
pub const MOLAR_MASS_EPSILON: f64 = 0.001;

/// Load pre-populated elements
pub fn load_known_elements() -> KnownElements {
	KnownElements::new_with_db(Path::new("./elements.toml")).unwrap()
}

/// Load known fertilizers for testing purposes
pub fn load_known_fertilizers(known_elements: &KnownElements) -> FertilizersDb {
	let mut fertilizers_db: FertilizersDb = Default::default();
	let data = fs::read_to_string(Path::new("./fertilizers.toml")).unwrap();
	fertilizers_db.load_db(data.as_str(), known_elements).unwrap();
	fertilizers_db
}

/// A sample tank for testing purposes
pub fn sample_tank() -> Tank {
	Tank::new_from_toml(
		r#"
		volume = 200
		absolute = false
		"#,
	)
	.unwrap()
}
