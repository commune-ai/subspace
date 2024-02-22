use sp_std::vec::Vec;

#[allow(dead_code)]
fn string2vec(s: &str) -> Vec<u8> {
	let mut v: Vec<u8> = Vec::new();
	for c in s.chars() {
		v.push(c as u8);
	}
	return v;
}
#[allow(dead_code)]
pub fn is_string_equal(s1: &str, s2: &str) -> bool {
	let v1: Vec<u8> = string2vec(s1);
	let v2: Vec<u8> = string2vec(s2);
	return v1 == v2;
}
#[allow(dead_code)]
pub fn is_string_vec(s1: &str, v2: Vec<u8>) -> bool {
	let v1: Vec<u8> = string2vec(s1);
	return v1 == v2.clone();
}
#[allow(dead_code)]
pub fn is_vec_str(v1: Vec<u8>, s2: &str) -> bool {
	let v2: Vec<u8> = string2vec(s2);
	return v1 == v2.clone();
}
