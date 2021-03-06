#[cfg(not(debug_assertions))]
use std::io::Read;
use std::{
    error::Error,
    fmt::{self, Display},
    io,
};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::cell::RefCell;
#[cfg(debug_assertions)]
use std::{
    fs,
    io::{Read, Write},
};

use crossbeam_channel::Sender;
use dict_derive::IntoPyObject;
use futures_channel::oneshot;
use iban::{Iban, IbanLike};
use lazy_static::lazy_static;
use pyo3::prelude::*;
use qrcode::{render::svg, EcLevel, QrCode};
use serde::{Deserialize, Serialize};
use tera::Tera;
#[cfg(debug_assertions)]
use uuid::Uuid;
use validator::{Validate, ValidationError};
use warp::{Rejection, Reply};
use wkhtmltopdf::PdfApplication;

#[cfg(debug_assertions)]
use crate::error::Error::IoError;
use crate::error::Error::{
    InvalidRequestInputError, PdfError, PythonError, QrCodeError, TeraError,
};

macro_rules! format_qr_code_data {
    () => {
        r#"SPC
0200
1
{creditor_iban}
K
{creditor_name}
{creditor_address}
{creditor_zip_code} {creditor_city}


{creditor_country}







{amount}
{currency}
K
{debtor_name}
{debtor_address}
{debtor_zip_code} {debtor_city}


{debtor_country}
{reference_type}
{reference_number}
{additional_information}
EPD"#
    };
}

macro_rules! format_qr_swiss_cross {
    () => {
        r#"
<rect x="{outer}" y="{outer}" class="st0" width="36" height="36"/>
<rect x="{inner}" y="{inner}" width="24" height="24"/>
<rect x="{cross_wide}" y="{cross_short}" class="st0" width="16" height="4"/>
<rect x="{cross_short}" y="{cross_wide}" class="st0" width="4" height="16"/>
<style type="text/css">
.st0 {{
    fill:#FFFFFF;
}}
</style>
"#
    };
}

lazy_static! {
    pub static ref QR_SLIP_TEMPLATES: Tera = {
        match Tera::new("src/resources/templates/*.html") {
            Ok(tera) => tera,
            Err(e) => panic!("Could not load tera templates: '{}'", e),
        }
    };
    pub static ref PDF_APPLICATION_WORKER_MANAGER: PdfApplicationWorkerManager =
        PdfApplicationWorkerManager::new();
    pub static ref PDF_WORKER_POOL_SIZE: usize = {
        std::env::var("PDF_WORKER_POOL_SIZE").map_or(0, |val| {
            val.parse::<usize>()
                .expect("PDF_WORKER_POOL_SIZE is not a valid usize")
        })
    };
}

/// Struct containing all data used to generate a qr_slip and qr code. Each endpoint in this module
/// expects an array of json objects (or in the case of /dbg-qr-svg just one) that can be deserialized
/// to this struct. Additionally, this struct validates input data according to the
/// [six specification](https://www.paymentstandards.ch/dam/downloads/ig-qr-bill-de.pdf).
#[derive(Clone, Serialize, Deserialize, IntoPyObject, Debug, Validate)]
#[validate(schema(function = "validate_qr_data", skip_on_field_errors = true))]
pub struct QrData {
    creditor_iban: String,
    #[validate(length(min = 1, max = 70))]
    creditor_name: String,
    #[validate(length(min = 1, max = 70))]
    creditor_address: String,
    creditor_zip_code: String,
    creditor_city: String,
    #[validate(length(min = 2, max = 2))]
    creditor_country: String,
    #[validate(length(min = 1, max = 70))]
    debtor_name: String,
    #[validate(length(min = 1, max = 70))]
    debtor_address: String,
    debtor_zip_code: String,
    debtor_city: String,
    #[validate(length(min = 2, max = 2))]
    debtor_country: String,
    #[validate(custom = "validate_amount")]
    amount: String,
    #[validate(custom = "validate_currency")]
    currency: String,
    reference_type: String,
    reference_number: Option<String>,
    #[validate(length(max = 140))]
    additional_information: Option<String>,
}

impl QrData {
    /// Verifies all conditions and additionally verifies and formats the IBAN
    pub fn verify(&mut self) -> Result<(), Rejection> {
        let iban = self.creditor_iban.parse::<Iban>().map_err(|e| {
            warp::reject::custom(InvalidRequestInputError(format!(
                "Provided IBAN '{}' is invalid: {}",
                &self.creditor_iban, e
            )))
        })?;
        let country = iban.country_code();

        if !(country == "CH" || country == "LI") {
            return Err(warp::reject::custom(InvalidRequestInputError(
                String::from("Country code of IBAN must be CH or LI"),
            )));
        }

        // replace with formatted string
        self.creditor_iban = iban.to_string();

        self.validate().map_err(|e| {
            warp::reject::custom(InvalidRequestInputError(format!(
                "Validation failed for QrData: {}",
                e
            )))
        })?;

        Ok(())
    }
}

/// Handler for the /generate-slip endpoint which creates a PDF containing a slip for all provided
/// [`QrData`] objects. The PDF is returned as bytes in the response body.
pub async fn generate_slip_handler(mut qr_data_vec: Vec<QrData>) -> Result<impl Reply, Rejection> {
    let qr_svg_vec = generate_qr_svg_for_all(&mut qr_data_vec)?;
    let html = generate_html_slip(qr_data_vec, qr_svg_vec)?;
    let pdf = PDF_APPLICATION_WORKER_MANAGER
        .generate_pdf_from_html(html)
        .await
        .map_err(|e| e.get_rejection())?;

    Ok(pdf)
}

/// Like [`generate_slip_handler`] but saves the PDF as a file in the local tmp/ directory.
/// Endpoint only available in debug mode.
#[cfg(debug_assertions)]
pub async fn dbg_qr_pdf_handler(mut qr_data_vec: Vec<QrData>) -> Result<impl Reply, Rejection> {
    let qr_svg_vec = generate_qr_svg_for_all(&mut qr_data_vec)?;
    let html = generate_html_slip(qr_data_vec, qr_svg_vec)?;
    let pdf = PDF_APPLICATION_WORKER_MANAGER
        .generate_pdf_from_html(html)
        .await
        .map_err(|e| e.get_rejection())?;

    save_bytes_to_file(&pdf, "pdf")?;

    Ok(warp::reply())
}

/// Like [`generate_slip_handler`] but only generates the html without converting it to PDF and saves
/// it as a file in the local tmp/ directory. Endpoint only available in debug mode.
#[cfg(debug_assertions)]
pub async fn dbg_qr_html_handler(mut qr_data_vec: Vec<QrData>) -> Result<impl Reply, Rejection> {
    let qr_svg_vec = generate_qr_svg_for_all(&mut qr_data_vec)?;
    let html = generate_html_slip(qr_data_vec, qr_svg_vec)?;

    save_bytes_to_file(html.as_bytes(), "html")?;

    Ok(warp::reply())
}

/// Like [`generate_slip_handler`] but only generates the qr code svg and saves it as a file in the
/// local tmp/ directory. Endpoint only available in debug mode.
#[cfg(debug_assertions)]
pub async fn dbg_qr_svg_handler(mut qr_data: QrData) -> Result<impl Reply, Rejection> {
    qr_data.verify()?;
    let qr_svg = generate_qr_svg(&qr_data)?;

    save_bytes_to_file(qr_svg.as_bytes(), "svg")?;

    Ok(warp::reply())
}

#[cfg(debug_assertions)]
fn save_bytes_to_file(bytes: &[u8], extension: &str) -> Result<(), Rejection> {
    let file_id = Uuid::new_v4();

    if !std::path::Path::new("tmp/").exists() {
        std::fs::create_dir("tmp").map_err(|e| warp::reject::custom(IoError(e.to_string())))?;
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(format!("tmp/{}.{}", file_id, extension))
        .map_err(|e| warp::reject::custom(IoError(e.to_string())))?;
    file.write_all(bytes)
        .map_err(|e| warp::reject::custom(IoError(e.to_string())))?;

    Ok(())
}

/// Generates a qr code rendered as an svg graphic for all provided [`QrData`] objects.
pub fn generate_qr_svg_for_all(qr_data_vec: &mut [QrData]) -> Result<Vec<String>, Rejection> {
    qr_data_vec
        .iter_mut()
        .map(|qr_data| {
            qr_data.verify()?;
            generate_qr_svg(qr_data)
        })
        .collect::<Result<Vec<String>, Rejection>>()
}

/// Generates a qr code rendered as an svg graphic returned as String for the given [`QrData`] object.
///
/// If USE_PY_QR_GENERATOR is set to true, the script resources/py/qr_generator.py is used to generate
/// the svg, else the qrcode crate is used. The encoded qr code conforms to the
/// [six specification](https://www.paymentstandards.ch/dam/downloads/ig-qr-bill-de.pdf).
pub fn generate_qr_svg(qr_data: &QrData) -> Result<String, Rejection> {
    if *crate::USE_PY_QR_GENERATOR {
        Python::with_gil(|py| {
            let module = PyModule::import(py, crate::QR_GENERATOR_MODULE)
                .map_err(|e| py_err_into_rejection(e, py))?;

            let create_qr_code_fn = module
                .getattr("create_qr_code")
                .map_err(|e| py_err_into_rejection(e, py))?;
            let svg_string: String = create_qr_code_fn
                .call1((qr_data.clone(),))
                .map_err(|e| py_err_into_rejection(e, py))?
                .extract()
                .map_err(|e| py_err_into_rejection(e, py))?;
            Ok(svg_string)
        })
    } else {
        let qr_data = format!(
            format_qr_code_data!(),
            creditor_iban = &qr_data.creditor_iban,
            creditor_name = &qr_data.creditor_name,
            creditor_address = &qr_data.creditor_address,
            creditor_zip_code = &qr_data.creditor_zip_code,
            creditor_city = &qr_data.creditor_city,
            creditor_country = &qr_data.creditor_country,
            amount = &qr_data.amount,
            currency = &qr_data.currency,
            debtor_name = &qr_data.debtor_name,
            debtor_address = &qr_data.debtor_address,
            debtor_zip_code = &qr_data.debtor_zip_code,
            debtor_city = &qr_data.debtor_city,
            debtor_country = &qr_data.debtor_country,
            reference_type = &qr_data.reference_type,
            reference_number = qr_data.reference_number.as_deref().unwrap_or(""),
            additional_information = qr_data.additional_information.as_deref().unwrap_or(""),
        );

        let qr_code = QrCode::with_error_correction_level(qr_data, EcLevel::Q)
            .map_err(|e| warp::reject::custom(QrCodeError(e.to_string())))?;

        let module_pixels = 4;
        let mut qr_svg: String = qr_code
            .render::<svg::Color>()
            .module_dimensions(module_pixels, module_pixels)
            .build();

        let width_in_modules = qr_code.width();
        // add two quiet zones of 4 modules each
        let pixel_width = module_pixels as usize * (width_in_modules + 4 * 2);
        let center = pixel_width / 2;

        // insert swiss cross element ahead of the closing tag
        if let Some(pos) = qr_svg.rfind("</svg>") {
            qr_svg.insert_str(
                pos,
                &format!(
                    format_qr_swiss_cross!(),
                    outer = center - (36 / 2),
                    inner = center - (24 / 2),
                    cross_wide = center - (16 / 2),
                    cross_short = center - (4 / 2),
                ),
            );
        }

        Ok(qr_svg)
    }
}

/// Generates the HTML file containing all qr slips for the provided [`QrData`] elements. The `qr_svg_vec`
/// contains the generated qr code for each object in `qr_data_vec` in the same position. Both parameters
/// are supplied to the context for the tera template which produces the HTML output.
pub fn generate_html_slip(
    qr_data_vec: Vec<QrData>,
    qr_svg_vec: Vec<String>,
) -> Result<String, Rejection> {
    let mut context = tera::Context::new();
    context.insert("qr_data_vec", &qr_data_vec);
    context.insert("qr_code_vec", &qr_svg_vec);
    QR_SLIP_TEMPLATES
        .render("qr_slip.html", &context)
        .map_err(|e| {
            warp::reject::custom(TeraError(format!(
                "{}: {}",
                e,
                e.source().map_or("None".to_owned(), |s| s.to_string())
            )))
        })
}

#[inline]
fn py_err_into_rejection(e: PyErr, py: Python) -> Rejection {
    warp::reject::custom(PythonError(e.pvalue(py).to_string()))
}

pub type PdfResult = Result<Vec<u8>, PdfApplicationError>;

/// Struct managing a worker thread, or worker process pool if PDF_WORKER_POOL_SIZE is set to a non-zero
/// value, that manages a wkhtmltopdf PdfApplication which builds PDF files from HTML input.
/// The worker process pool option is only supported on macOS or Linux.
pub struct PdfApplicationWorkerManager {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pool: Option<procspawn::Pool>,
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    html_channel: Option<Sender<(String, oneshot::Sender<PdfResult>)>>,
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    html_channel: Sender<(String, oneshot::Sender<PdfResult>)>,
}

impl PdfApplicationWorkerManager {

    /// Creates a new PdfApplicationWorkerManager by spawning the worker thread if PDF_WORKER_POOL_SIZE
    /// is not set to a non-zero or sets up a process pool with the size provided by PDF_WORKER_POOL_SIZE.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub fn new() -> Self {
        if *PDF_WORKER_POOL_SIZE > 0 {
            let pool = match procspawn::Pool::new(*PDF_WORKER_POOL_SIZE) {
                Ok(pool) => pool,
                Err(e) => panic!(
                    "Failed to spawn PdfApplication worker process: '{}'",
                    e.to_string()
                ),
            };

            log::info!(
                "Set up PDF worker process pool with {} processes",
                *PDF_WORKER_POOL_SIZE
            );

            Self {
                pool: Some(pool),
                html_channel: None,
            }
        } else {
            Self::new_single_threaded_worker()
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    pub fn new() -> Self {
        if *PDF_WORKER_POOL_SIZE > 0 {
            log::warn!("PDF_WORKER_POOL_SIZE set but the current platform does not support procspawn, falling back to single worker thread.");
        }
        Self::new_single_threaded_worker()
    }

    /// Submits a task to the worker thread or process pool to convert the provided HTML string to
    /// a Vec of bytes representing a PDF file.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    pub async fn generate_pdf_from_html(&self, html: String) -> PdfResult {
        if let Some(ref pool) = self.pool {
            let join_handle: procspawn::JoinHandle<Result<Vec<u8>, String>> =
                pool.spawn(html, |html| {
                    std::thread_local! {
                        static PDF_APPLICATION: RefCell<PdfApplication> = match PdfApplication::new() {
                            Ok(p) => {
                                log::debug!("Initialised PdfApplication for worker process");
                                RefCell::new(p)
                            },
                            Err(e) => {
                                panic!("Failed to initialise PdfApplication: '{}'", e.to_string())
                            }
                        };
                    };

                    log::debug!("PDF worker process received html");
                    PDF_APPLICATION.with(|pdf_application| {
                        convert_html_to_pdf(&mut *pdf_application.borrow_mut(), &html).map_err(|e| e.to_string())
                    })
                });

            match join_handle.join() {
                Ok(result) => result.map_err(PdfApplicationError::RawError),
                Err(_) => Err(PdfApplicationError::SpawnError),
            }
        } else if let Some(ref html_channel) = self.html_channel {
            let (result_sender, result_receiver) = oneshot::channel::<PdfResult>();
            html_channel
                .send((html, result_sender))
                .expect("Html channel disconnected unexpectedly");
            result_receiver
                .await
                .expect("Pdf result channel has closed unexpectedly")
        } else {
            panic!("Either pool or html_channel must be set")
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    pub async fn generate_pdf_from_html(&self, html: String) -> PdfResult {
        let (result_sender, result_receiver) = oneshot::channel::<PdfResult>();
        self.html_channel
            .send((html, result_sender))
            .expect("Html channel disconnected unexpectedly");
        result_receiver
            .await
            .expect("Pdf result channel has closed unexpectedly")
    }

    fn new_single_threaded_worker() -> Self {
        let (html_sender, html_receiver) =
            crossbeam_channel::unbounded::<(String, oneshot::Sender<PdfResult>)>();

        std::thread::Builder::new()
            .name(String::from("pdf_worker"))
            .spawn(move || {
                let mut pdf_application = match PdfApplication::new() {
                    Ok(pdf_application) => {
                        log::debug!("Initialised PdfApplication for worker thread");
                        pdf_application
                    },
                    Err(e) => panic!("Failed to initialise wkhtmltopdf: {}", e.to_string()),
                };

                loop {
                    let (html, result_sender) = html_receiver
                        .recv()
                        .expect("Html channel disconnected unexpectedly");

                    log::debug!("PDF worker thread received html");
                    // flatten `Result<Result<T, E>, E>` to `Result<T, E>` manually as flatten() is currently nightly only
                    let flattened_pdf_result = convert_html_to_pdf(&mut pdf_application, &html);

                    result_sender
                        .send(flattened_pdf_result)
                        .expect("Pdf result channel has closed unexpectedly");
                }
            })
            .expect("Failed to spawn pdf_worker thread");

        log::info!("Set up PDF worker thread.");

        #[cfg(any(target_os = "macos", target_os = "linux"))]
        return Self {
            pool: None,
            html_channel: Some(html_sender),
        };

        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        return Self {
            html_channel: html_sender,
        };
    }
}

/// Convert the given HTML string to a Vec of bytes representing a PDF file using the given PdfApplication.
fn convert_html_to_pdf(pdf_application: &mut PdfApplication, html: &str) -> PdfResult {
    use wkhtmltopdf::Orientation;
    use wkhtmltopdf::Size;

    log::debug!("Converting html to pdf");
    let pdf_result = pdf_application
        .builder()
        .title("Qr Slip")
        .orientation(Orientation::Portrait)
        .margin(Size::Millimeters(0))
        .build_from_html(html)
        .map(|output| {
            output
                .bytes()
                .collect::<Result<Vec<u8>, io::Error>>()
                .map_err(PdfApplicationError::IoError)
        })
        .map_err(PdfApplicationError::WkhtmlError);

    log::debug!("Done converting html to pdf");

    // flatten `Result<Result<T, E>, E>` to `Result<T, E>` manually as flatten() is currently nightly only
    match pdf_result {
        Ok(Err(e)) => Err(e),
        Ok(Ok(bytes)) => Ok(bytes),
        Err(e) => Err(e),
    }
}

impl Default for PdfApplicationWorkerManager {
    fn default() -> Self {
        PdfApplicationWorkerManager::new()
    }
}

#[derive(Debug)]
pub enum PdfApplicationError {
    WkhtmlError(wkhtmltopdf::Error),
    IoError(io::Error),
    /// When an error is received across process boundaries only the message is serialised,
    /// the message is then wrapped in this enum variant to transform it back to a PdfApplicationError.
    RawError(String),
    /// Error returned when spawning a PDF worker process fails
    SpawnError,
}

impl std::error::Error for PdfApplicationError {}

impl Display for PdfApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PdfApplicationError::WkhtmlError(ref e) => formatter.write_str(&e.to_string()),
            PdfApplicationError::IoError(ref e) => formatter.write_str(&e.to_string()),
            PdfApplicationError::RawError(ref e) => formatter.write_str(e),
            PdfApplicationError::SpawnError => {
                formatter.write_str("Failed to spawn PdfApplication worker process")
            }
        }
    }
}

impl PdfApplicationError {
    pub fn get_rejection(&self) -> Rejection {
        warp::reject::custom(PdfError(self.to_string()))
    }
}

fn validate_qr_data(qr_data: &QrData) -> Result<(), ValidationError> {
    if qr_data.creditor_zip_code.len() + qr_data.creditor_city.len() > 69 {
        return Err(ValidationError::new(
            "Combined length of creditor zip code and city may not exceed 69",
        ));
    }

    if qr_data.debtor_zip_code.len() + qr_data.debtor_city.len() > 69 {
        return Err(ValidationError::new(
            "Combined length of debtor zip code and city may not exceed 69",
        ));
    }

    match qr_data.reference_type.as_str() {
        "QRR" => match qr_data.reference_number {
            Some(ref reference_number) if !reference_number.is_empty() => {
                if reference_number.len() != 27 {
                    return Err(ValidationError::new(
                        "Reference number must be of length 27 when the reference type is QRR",
                    ));
                }

                if !reference_number.chars().all(|c| c.is_digit(10)) {
                    return Err(ValidationError::new(
                        "Reference number must be numerical when the reference type is QRR",
                    ));
                }

                if !is_qr_iban(&qr_data.creditor_iban) {
                    return Err(ValidationError::new("IBAN must be a QR-IBAN (1-based position 5-9 must be between 30000 and 31999) when the reference type is QRR"));
                }
            }
            _ => {
                return Err(ValidationError::new(
                    "Reference number must be provided when the reference type is QRR",
                ));
            }
        },
        "SCOR" => {
            match qr_data.reference_number {
                Some(ref reference_number) if !reference_number.is_empty() => {
                    if reference_number.len() < 5 || reference_number.len() > 25 {
                        return Err(ValidationError::new("Reference number must be of length 5 - 25 when the reference type is SCOR"));
                    }

                    if !reference_number.chars().all(|c| c.is_alphanumeric()) {
                        return Err(ValidationError::new(
                            "Reference number must be alphanumeric when the reference type is SCOR",
                        ));
                    }

                    if is_qr_iban(&qr_data.creditor_iban) {
                        return Err(ValidationError::new(
                            "Reference type must be QRR if the IBAN is a QR-IBAN",
                        ));
                    }
                }
                _ => {
                    return Err(ValidationError::new(
                        "Reference number must be provided when the reference type is SCOR",
                    ));
                }
            }
        }
        "NON" => match qr_data.reference_number {
            Some(ref reference_number) if !reference_number.is_empty() => {
                return Err(ValidationError::new(
                    "Reference number must not be provided when the reference type is NON",
                ));
            }
            _ => {
                if is_qr_iban(&qr_data.creditor_iban) {
                    return Err(ValidationError::new(
                        "Reference type must be QRR if the IBAN is a QR-IBAN",
                    ));
                }
            }
        },
        _ => {
            return Err(ValidationError::new(
                "Reference type must be QRR, SCOR or NON",
            ));
        }
    }

    Ok(())
}

#[inline]
fn is_qr_iban(iban: &str) -> bool {
    let iid = match (&iban[4..9]).parse::<u32>() {
        Ok(iid) => iid,
        Err(_) => return false,
    };

    (30000..=31999).contains(&iid)
}

fn validate_amount(amount: &str) -> Result<(), ValidationError> {
    let split = amount.split('.').collect::<Vec<&str>>();

    if split.len() != 2 {
        return Err(ValidationError::new(
            "Decimal amount not formatted correctly, expected both integral and fractional parts",
        ));
    }

    let integral_str = split[0];
    let fractional_str = split[1];

    if integral_str.starts_with('0') {
        return Err(ValidationError::new(
            "Decimal amount not formatted correctly, amount must not start with leading 0s",
        ));
    }

    if fractional_str.len() != 2 {
        return Err(ValidationError::new(
            "Decimal amount not formatted correctly, amount must contain 2 fractional digits",
        ));
    }

    let integral = integral_str.parse::<u32>().map_err(|_| {
        ValidationError::new("Decimal amount not formatted correctly, integral is not a valid u32")
    })?;
    let fractional = fractional_str.parse::<u32>().map_err(|_| {
        ValidationError::new(
            "Decimal amount not formatted correctly, fractional is not a valid u32",
        )
    })?;

    if integral == 0 && fractional == 0 {
        return Err(ValidationError::new(
            "Decimal amount not formatted correctly, amount must be at least 0.01",
        ));
    }

    if integral > 999999999 {
        return Err(ValidationError::new(
            "Decimal amount not formatted correctly, amount may not exceed 999999999.99",
        ));
    }

    Ok(())
}

fn validate_currency(currency: &str) -> Result<(), ValidationError> {
    if !(currency == "CHF" || currency == "EUR") {
        return Err(ValidationError::new("Currency must be either CHF or EUR"));
    }

    Ok(())
}
