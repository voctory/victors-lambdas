//! Buildable data masking snippet.

use serde_json::json;
use victors_lambdas::data_masking::{DataMasking, MaskingOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event = json!({
        "customer": {
            "name": "Ada",
            "password": "correct-horse-battery-staple",
            "card": "4111111111111111"
        }
    });

    let data_masking = DataMasking::new();
    let password_masked = data_masking.erase_fields(event, &["customer.password"])?;
    let fully_masked = data_masking.erase_fields_with(
        password_masked,
        &["customer.card"],
        &MaskingOptions::regex(r"\d{12}(\d{4})", "************$1"),
    )?;

    println!("{fully_masked}");

    Ok(())
}
