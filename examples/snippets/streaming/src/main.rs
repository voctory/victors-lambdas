//! Buildable streaming utility snippet.

use std::io::{Read as _, Seek as _, SeekFrom};

use aws_lambda_powertools::streaming::{BytesRangeSource, SeekableStream, csv_reader};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Order {
    item: String,
    quantity: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = BytesRangeSource::new(b"item,quantity\ncoffee,2\ntea,3\n".to_vec());
    let mut stream = SeekableStream::new(source);
    let mut prefix = [0; 4];
    stream.read_exact(&mut prefix)?;
    stream.seek(SeekFrom::Start(0))?;

    let mut reader = csv_reader(stream);
    for row in reader.deserialize::<Order>() {
        let order = row?;
        println!("item={} quantity={}", order.item, order.quantity);
    }

    Ok(())
}
