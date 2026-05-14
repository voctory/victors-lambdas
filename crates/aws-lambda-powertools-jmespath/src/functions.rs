//! Powertools `JMESPath` functions.

use std::io::Read;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use flate2::read::GzDecoder;
use jmespath::{
    Context, ErrorReason, JmespathError as RuntimeJmespathError, Rcvar, Runtime, ToJmespath,
    Variable,
    functions::{ArgumentType, CustomFunction, Signature},
};
use serde_json::Value;

pub(crate) fn powertools_runtime() -> Runtime {
    let mut runtime = Runtime::new();
    runtime.register_builtin_functions();
    register_powertools_functions(&mut runtime);
    runtime
}

fn register_powertools_functions(runtime: &mut Runtime) {
    runtime.register_function(
        "powertools_json",
        Box::new(CustomFunction::new(
            string_signature(),
            Box::new(|args, ctx| {
                let value = string_arg(args, ctx)?;
                let parsed = serde_json::from_str::<Value>(value).map_err(|error| {
                    runtime_error(
                        ctx,
                        format!("powertools_json could not parse JSON: {error}"),
                    )
                })?;
                parsed.to_jmespath()
            }),
        )),
    );

    runtime.register_function(
        "powertools_base64",
        Box::new(CustomFunction::new(
            string_signature(),
            Box::new(|args, ctx| {
                let value = string_arg(args, ctx)?;
                let decoded = STANDARD.decode(value).map_err(|error| {
                    runtime_error(
                        ctx,
                        format!("powertools_base64 could not decode base64: {error}"),
                    )
                })?;
                let decoded = String::from_utf8(decoded).map_err(|error| {
                    runtime_error(
                        ctx,
                        format!("powertools_base64 decoded bytes are not UTF-8: {error}"),
                    )
                })?;
                Ok(Rcvar::new(Variable::String(decoded)))
            }),
        )),
    );

    runtime.register_function(
        "powertools_base64_gzip",
        Box::new(CustomFunction::new(
            string_signature(),
            Box::new(|args, ctx| {
                let value = string_arg(args, ctx)?;
                let compressed = STANDARD.decode(value).map_err(|error| {
                    runtime_error(
                        ctx,
                        format!("powertools_base64_gzip could not decode base64: {error}"),
                    )
                })?;
                let mut decoder = GzDecoder::new(compressed.as_slice());
                let mut uncompressed = String::new();
                decoder.read_to_string(&mut uncompressed).map_err(|error| {
                    runtime_error(
                        ctx,
                        format!("powertools_base64_gzip could not decompress UTF-8 text: {error}"),
                    )
                })?;
                Ok(Rcvar::new(Variable::String(uncompressed)))
            }),
        )),
    );
}

fn string_signature() -> Signature {
    Signature::new(vec![ArgumentType::String], None)
}

fn string_arg<'a>(args: &'a [Rcvar], ctx: &Context<'_>) -> Result<&'a str, RuntimeJmespathError> {
    args.first()
        .and_then(|value| value.as_string())
        .map(String::as_str)
        .ok_or_else(|| runtime_error(ctx, "expected a string argument"))
}

fn runtime_error(ctx: &Context<'_>, message: impl Into<String>) -> RuntimeJmespathError {
    RuntimeJmespathError::from_ctx(ctx, ErrorReason::Parse(message.into()))
}
