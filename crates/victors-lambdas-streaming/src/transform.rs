//! Streaming transforms.

#[cfg(any(feature = "csv", feature = "gzip", feature = "zip"))]
use std::io::Read;
#[cfg(feature = "zip")]
use std::io::Seek;

/// Wraps a reader in a gzip decoder.
#[cfg(feature = "gzip")]
#[cfg_attr(docsrs, doc(cfg(feature = "gzip")))]
pub fn gzip_decoder<R>(reader: R) -> flate2::read::GzDecoder<R>
where
    R: Read,
{
    flate2::read::GzDecoder::new(reader)
}

/// Creates a CSV reader from a byte stream.
#[cfg(feature = "csv")]
#[cfg_attr(docsrs, doc(cfg(feature = "csv")))]
pub fn csv_reader<R>(reader: R) -> csv::Reader<R>
where
    R: Read,
{
    csv::Reader::from_reader(reader)
}

/// Creates a CSV reader from a byte stream using a configured builder.
#[cfg(feature = "csv")]
#[cfg_attr(docsrs, doc(cfg(feature = "csv")))]
pub fn csv_reader_with_builder<R>(reader: R, builder: &csv::ReaderBuilder) -> csv::Reader<R>
where
    R: Read,
{
    builder.from_reader(reader)
}

/// Creates a ZIP archive reader from a seekable byte stream.
///
/// # Errors
///
/// Returns an error when the stream does not contain a readable ZIP archive.
#[cfg(feature = "zip")]
#[cfg_attr(docsrs, doc(cfg(feature = "zip")))]
pub fn zip_archive<R>(reader: R) -> zip::result::ZipResult<zip::ZipArchive<R>>
where
    R: Read + Seek,
{
    zip::ZipArchive::new(reader)
}

#[cfg(test)]
mod tests {
    use std::io::Read as _;

    use crate::{BytesRangeSource, SeekableStream};

    #[cfg(feature = "csv")]
    #[test]
    fn reads_csv_rows() {
        let source = BytesRangeSource::new(b"name,quantity\ncoffee,2\ntea,3\n".to_vec());
        let stream = SeekableStream::new(source);
        let mut reader = super::csv_reader(stream);
        let rows = reader
            .records()
            .collect::<Result<Vec<_>, _>>()
            .expect("CSV should parse");

        assert_eq!(rows.len(), 2);
        assert_eq!(&rows[0][0], "coffee");
        assert_eq!(&rows[0][1], "2");
    }

    #[cfg(feature = "gzip")]
    #[test]
    fn reads_gzip_data() {
        use std::io::Write as _;

        let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(b"compressed").expect("write should work");
        let bytes = encoder.finish().expect("gzip should finish");
        let source = BytesRangeSource::new(bytes);
        let stream = SeekableStream::new(source);
        let mut decoder = super::gzip_decoder(stream);
        let mut output = String::new();

        decoder
            .read_to_string(&mut output)
            .expect("gzip should decode");

        assert_eq!(output, "compressed");
    }

    #[cfg(feature = "zip")]
    #[test]
    fn reads_zip_archives() {
        use std::io::{Cursor, Write as _};

        let mut buffer = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer
                .start_file("orders.txt", options)
                .expect("file should start");
            writer.write_all(b"order-1").expect("write should work");
            writer.finish().expect("zip should finish");
        }

        let source = BytesRangeSource::new(buffer.into_inner());
        let stream = SeekableStream::new(source);
        let mut archive = super::zip_archive(stream).expect("zip should open");
        let mut file = archive.by_name("orders.txt").expect("file should exist");
        let mut output = String::new();

        file.read_to_string(&mut output).expect("file should read");

        assert_eq!(output, "order-1");
    }
}
