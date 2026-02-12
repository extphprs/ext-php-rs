//! JSON plugin - demonstrates auto-detected dependencies

use serde::{Deserialize, Serialize}; // 1.0, features = ["derive"]
use serde_json; // 1.0

#[derive(Serialize, Deserialize)]
struct Person {
    name: String,
    age: u32,
}

#[php_function]
fn json_encode_person(name: String, age: i64) -> String {
    let person = Person {
        name,
        age: age as u32,
    };
    serde_json::to_string(&person).unwrap_or_default()
}

#[php_function]
fn json_decode_person(json: String) -> String {
    match serde_json::from_str::<Person>(&json) {
        Ok(person) => format!("{} is {} years old", person.name, person.age),
        Err(e) => format!("Error: {}", e),
    }
}

#[php_function]
fn json_pretty(json: String) -> String {
    match serde_json::from_str::<serde_json::Value>(&json) {
        Ok(value) => serde_json::to_string_pretty(&value).unwrap_or_default(),
        Err(e) => format!("Error: {}", e),
    }
}
