pub fn uppercase_first_letter(s: String) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first_letter) => first_letter.to_uppercase().chain(chars).collect(),
    }
}
