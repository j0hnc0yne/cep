use thiserror::Error;

#[derive(Debug, Error)]
pub enum CepError {
    #[error("Invalid value '{value}' for --{option}. Possible values: {possible}")]
    InvalidOptionValue {
        option: String,
        value: String,
        possible: String,
    },

    #[error("Invalid date format '{input}': expected YYYYMMDDhhmm (e.g. 202505151430)")]
    InvalidDateFormat { input: String },

    #[error("--start must be before --end")]
    StartNotBeforeEnd,

    #[error("Invalid interval '{input}': must be like 5m, 2h, 1d (units: m/h/d, min 5m, max 7d)")]
    InvalidInterval { input: String },

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error: {0}")]
    Api(String),

    #[error("Output error: {0}")]
    Output(String),
}
