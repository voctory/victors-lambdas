use aws_lambda_powertools::metadata::get_lambda_metadata;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let metadata = get_lambda_metadata()?;

    if let Some(availability_zone_id) = metadata.availability_zone_id() {
        println!("availability_zone_id={availability_zone_id}");
    }

    Ok(())
}
