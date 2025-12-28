use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rawzip::{ZipArchive, time::ZipDateTimeKind};
use serde::Serialize;
use std::fmt;
use std::fs::File;
use std::io::{self, BufRead, BufWriter, Read, Write};
use std::path::Path;

// A custom error type to distinguish I/O errors from size limit errors.
#[derive(Debug)]
pub enum ReadError {
    Io(io::Error),
    SizeLimitExceeded,
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::Io(e) => write!(f, "{}", e),
            ReadError::SizeLimitExceeded => write!(f, "file size exceeds limit"),
        }
    }
}

impl From<io::Error> for ReadError {
    fn from(err: io::Error) -> ReadError {
        ReadError::Io(err)
    }
}

#[derive(Serialize, Debug)]
pub struct Metadata {
    #[serde(rename = "ZipName")]
    pub zip_name: String,
}

#[derive(Serialize, Debug)]
pub struct Blob {
    pub name: String,
    pub content_type: String,
    pub content_encoding: String,
    pub content_transfer_encoding: String,
    pub body: String,
    pub metadata: Metadata,
    pub content_length: u64,
    pub last_modified: String,
}

fn zip_datetime_to_chrono_utc(zdt: &ZipDateTimeKind) -> DateTime<Utc> {
    let (year, month, day, hour, minute, second) = (
        zdt.year(),
        zdt.month(),
        zdt.day(),
        zdt.hour(),
        zdt.minute(),
        zdt.second(),
    );
    let naive_date =
        NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32).unwrap_or_default();
    let naive_time = chrono::NaiveTime::from_hms_opt(hour as u32, minute as u32, second as u32)
        .unwrap_or_default();
    let naive_dt = NaiveDateTime::new(naive_date, naive_time);
    DateTime::from_naive_utc_and_offset(naive_dt, Utc)
}

pub fn rdr2buf<R>(rdr: R, buf: &mut Vec<u8>, limit: u64) -> Result<(), ReadError>
where
    R: Read,
{
    let mut taken = rdr.take(limit + 1);
    buf.clear();
    taken.read_to_end(buf)?;
    if buf.len() as u64 > limit {
        return Err(ReadError::SizeLimitExceeded);
    }
    Ok(())
}

pub fn filename2buf<P>(filename: P, buf: &mut Vec<u8>, limit: u64) -> Result<(), ReadError>
where
    P: AsRef<Path>,
{
    let f = File::open(filename)?;
    rdr2buf(f, buf, limit)
}

fn rdr2filenames<R>(rdr: R) -> impl Iterator<Item = Result<String, io::Error>>
where
    R: BufRead,
{
    rdr.lines()
}

fn stdin2filenames() -> impl Iterator<Item = Result<String, io::Error>> {
    rdr2filenames(io::stdin().lock())
}

pub fn buf2zip2blobs2jsons2writer<W>(
    zip_name: &str,
    zipdata: &[u8],
    content_type: &str,
    content_encoding: &str,
    max_item_size: u64,
    verbose: bool,
    wtr: &mut BufWriter<W>,
) -> Result<(), io::Error>
where
    W: Write,
{
    let archive = ZipArchive::from_slice(zipdata).map_err(io::Error::other)?;

    for entry_result in archive.entries() {
        let entry_header = entry_result.map_err(io::Error::other)?;
        let wayfinder = entry_header.wayfinder();
        let entry = archive.get_entry(wayfinder).map_err(io::Error::other)?;
        let entry_data = entry.data();
        let file_name = String::from_utf8_lossy(entry_header.file_path().as_bytes()).to_string();

        if entry_data.len() as u64 > max_item_size {
            if verbose {
                eprintln!(
                    "level:warn\tstatus:item_skipped\treason:size_limit_exceeded\tpath:{}\titem:{}\tsize:{}",
                    zip_name,
                    file_name,
                    entry_data.len()
                );
            }
            continue;
        }

        let dt: DateTime<Utc> = zip_datetime_to_chrono_utc(&entry_header.last_modified());

        let blob = Blob {
            name: file_name,
            content_type: content_type.to_string(),
            content_encoding: content_encoding.to_string(),
            content_transfer_encoding: "base64".to_string(),
            body: general_purpose::STANDARD.encode(entry_data),
            metadata: Metadata {
                zip_name: zip_name.to_string(),
            },
            content_length: entry_data.len() as u64,
            last_modified: dt.to_rfc3339(),
        };

        serde_json::to_writer(&mut *wtr, &blob)?;
        writeln!(&mut *wtr)?;
    }

    Ok(())
}

pub struct Options<'a> {
    pub max_zip_size: u64,
    pub content_type: &'a str,
    pub content_encoding: &'a str,
    pub max_item_size: u64,
    pub verbose: bool,
}

pub fn zfilename2zip2blobs2jsons2writer<P, W>(
    zfilename: P,
    buf: &mut Vec<u8>,
    options: &Options,
    wtr: &mut BufWriter<W>,
) -> Result<(), io::Error>
where
    W: Write,
    P: AsRef<Path> + Clone,
{
    let zfn_for_err = zfilename.as_ref().to_string_lossy().to_string();
    match filename2buf(zfilename.as_ref(), buf, options.max_zip_size) {
        Ok(_) => {
            // Processing continues below
        }
        Err(e) => {
            if options.verbose {
                match e {
                    ReadError::SizeLimitExceeded => {
                        eprintln!(
                            "level:warn\tstatus:zip_skipped\treason:size_limit_exceeded\tpath:{}",
                            zfn_for_err
                        );
                    }
                    ReadError::Io(io_err) => {
                        eprintln!(
                            "level:warn\tstatus:zip_skipped\treason:read_error\tpath:{}\terror:{}",
                            zfn_for_err, io_err
                        );
                    }
                }
            }
            return Ok(()); // Skip to the next file
        }
    };

    let zip_name = zfilename.as_ref().to_string_lossy().to_string();

    if let Err(e) = buf2zip2blobs2jsons2writer(
        &zip_name,
        buf,
        options.content_type,
        options.content_encoding,
        options.max_item_size,
        options.verbose,
        wtr,
    ) && options.verbose
    {
        eprintln!(
            "level:warn\tstatus:zip_processing_failed\tpath:{}\treason:{}",
            zfn_for_err, e
        );
    }
    Ok(())
}

pub fn zfilenames2zip2blobs2jsons2writer<I, W>(
    zfilenames: I,
    buf: &mut Vec<u8>,
    options: &Options,
    wtr: &mut BufWriter<W>,
) -> Result<(), io::Error>
where
    W: Write,
    I: Iterator<Item = Result<String, io::Error>>,
{
    for zfilename_res in zfilenames {
        match zfilename_res {
            Ok(zfilename) => {
                if let Err(e) = zfilename2zip2blobs2jsons2writer(&zfilename, buf, options, wtr)
                    && options.verbose
                {
                    eprintln!(
                        "level:warn\tstatus:unrecoverable_error\tpath:{}\treason:{}",
                        zfilename, e
                    );
                }
            }
            Err(e) => {
                if options.verbose {
                    eprintln!("level:warn\tstatus:unrecoverable_error\treason:{}", e);
                }
            }
        }
    }
    Ok(())
}

pub fn stdin2zfilenames2zip2blobs2jsons2stdout(
    max_zip_size: u64,
    content_type: &str,
    content_encoding: &str,
    max_item_size: u64,
    verbose: bool,
) -> Result<(), io::Error> {
    let stdout = io::stdout();
    let mut writer = BufWriter::new(stdout.lock());
    let mut buf: Vec<u8> = Vec::with_capacity((1 << 20) * 2);
    let options = Options {
        max_zip_size,
        content_type,
        content_encoding,
        max_item_size,
        verbose,
    };

    zfilenames2zip2blobs2jsons2writer(stdin2filenames(), &mut buf, &options, &mut writer)?;

    writer.flush()
}
