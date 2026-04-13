//! MCP server exposing Perplexity AI tools for search, research, and reasoning.

mod auth;
mod config;
mod server;
mod setup;
mod tty;

use perplexity_web_api::{Client, ReasonModel, SearchModel};
use rmcp::{ServiceExt, transport::stdio};
use std::{env, env::VarError};
use tracing_subscriber::fmt;

use crate::{
    auth::{AuthSource, ResolvedAuth, resolve_auth},
    server::PerplexityServer,
};

#[cfg(feature = "streamable-http")]
use rmcp::transport::streamable_http_server::{
    StreamableHttpService, session::local::LocalSessionManager,
};

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    match signal(SignalKind::terminate()) {
        Ok(mut sigterm) => {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = sigterm.recv() => {}
            }
        }
        Err(err) => {
            tracing::warn!("Failed to register SIGTERM handler: {}", err);
            if let Err(ctrl_c_err) = tokio::signal::ctrl_c().await {
                tracing::warn!("Failed to listen for SIGINT: {}", ctrl_c_err);
            }
        }
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        tracing::warn!("Failed to listen for shutdown signal: {}", err);
    }
}

/// Reads an optional string environment variable, returning `None` if not present.
fn optional_env(name: &str) -> Result<Option<String>, std::io::Error> {
    match env::var(name) {
        Ok(value) => {
            let trimmed = value.trim().to_owned();
            if trimmed.is_empty() { Ok(None) } else { Ok(Some(trimmed)) }
        }
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(std::io::Error::other(format!(
            "Environment variable {name} must be valid UTF-8"
        ))),
    }
}

/// Reads an optional default model from environment.
fn optional_model_env<T>(name: &str) -> Result<Option<T>, std::io::Error>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(name) {
        Ok(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }

            trimmed.parse::<T>().map(Some).map_err(|e| {
                std::io::Error::other(format!("Invalid environment variable {name}: {e}"))
            })
        }
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(std::io::Error::other(format!(
            "Environment variable {name} must be valid UTF-8"
        ))),
    }
}

/// Reads an optional boolean environment variable, returning `default` if not present.
fn optional_bool_env(name: &str, default: bool) -> Result<bool, std::io::Error> {
    optional_env(name)?.as_deref().map_or(Ok(default), |value| parse_bool_env(name, value))
}

fn parse_bool_env(name: &str, value: &str) -> Result<bool, std::io::Error> {
    if value.eq_ignore_ascii_case("true") {
        Ok(true)
    } else if value.eq_ignore_ascii_case("false") {
        Ok(false)
    } else {
        Err(std::io::Error::other(format!(
            "Invalid environment variable {name}: expected true/false"
        )))
    }
}

fn resolve_default_models(
    tokenless: bool,
) -> Result<(Option<SearchModel>, Option<ReasonModel>), std::io::Error> {
    if tokenless {
        if env::var("PERPLEXITY_ASK_MODEL").is_ok() {
            return Err(std::io::Error::other(format!(
                "PERPLEXITY_ASK_MODEL cannot be used without authentication.\n\n{}",
                authenticated_model_help()
            )));
        }
        if env::var("PERPLEXITY_REASON_MODEL").is_ok() {
            return Err(std::io::Error::other(format!(
                "PERPLEXITY_REASON_MODEL cannot be used without authentication.\n\n{}",
                authenticated_model_help()
            )));
        }
        Ok((None, None))
    } else {
        let ask = optional_model_env::<SearchModel>("PERPLEXITY_ASK_MODEL")?
            .unwrap_or(SearchModel::ProAuto);
        let reason = optional_model_env::<ReasonModel>("PERPLEXITY_REASON_MODEL")?;
        Ok((Some(ask), reason))
    }
}

fn authenticated_model_help() -> String {
    format!(
        "To use model configuration, provide both:\n  {}\n  {}\n\
         Or run the MCP binary once in an interactive terminal to complete local auth setup.",
        "PERPLEXITY_SESSION_TOKEN  - Perplexity session token",
        "PERPLEXITY_CSRF_TOKEN     - Perplexity CSRF token"
    )
}

fn log_auth_source(resolved_auth: &ResolvedAuth) {
    match resolved_auth.source {
        AuthSource::Environment => {
            tracing::info!("Using Perplexity authentication from environment variables")
        }
        AuthSource::CachedConfig => {
            tracing::info!("Using Perplexity authentication from cached local config")
        }
        AuthSource::InteractiveSetup => {
            tracing::info!("Using Perplexity authentication from interactive first-run setup")
        }
        AuthSource::Tokenless => tracing::warn!(
            "Starting Perplexity MCP server in tokenless mode (only perplexity_search and perplexity_ask with the turbo model are available)"
        ),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing (logs to stderr to not interfere with stdio transport)
    fmt()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let resolved_auth = resolve_auth().await?;
    let tokenless = resolved_auth.tokenless_mode();
    let incognito = optional_bool_env("PERPLEXITY_INCOGNITO", true)?;
    let (default_ask_model, default_reason_model) = resolve_default_models(tokenless)?;

    log_auth_source(&resolved_auth);
    tracing::info!(
        "Perplexity request incognito mode is {}",
        if incognito { "enabled" } else { "disabled" }
    );

    let mut builder = Client::builder();
    if let Some(cookies) = resolved_auth.cookies {
        builder = builder.cookies(cookies);
    }

    let client = builder.build().await.map_err(|e| {
        tracing::error!("Failed to create Perplexity client: {}", e);
        e
    })?;

    tracing::info!("Perplexity client initialized");

    let server = PerplexityServer::new(
        client,
        default_ask_model,
        default_reason_model,
        tokenless,
        incognito,
    );

    let transport = optional_env("MCP_TRANSPORT")?.unwrap_or_else(|| "stdio".to_owned());

    match transport.as_str() {
        "stdio" => {
            let service = server.serve(stdio()).await.inspect_err(|e| {
                tracing::error!("Server error: {:?}", e);
            })?;

            tracing::info!("MCP server running on stdio");

            tokio::select! {
                result = service.waiting() => {
                    result?;
                }
                _ = shutdown_signal() => {
                    tracing::info!("Shutdown signal received, stopping MCP server");
                }
            }
        }
        #[cfg(feature = "streamable-http")]
        "streamable-http" => {
            let host = optional_env("MCP_HOST")?.unwrap_or_else(|| "0.0.0.0".to_owned());
            let port = optional_env("MCP_PORT")?.unwrap_or_else(|| "8080".to_owned());
            let addr = format!("{host}:{port}");

            let http_service = StreamableHttpService::new(
                move || Ok(server.clone()),
                LocalSessionManager::default().into(),
                Default::default(),
            );

            let app = axum::Router::new().nest_service("/mcp", http_service);
            let listener = tokio::net::TcpListener::bind(&addr).await?;
            tracing::info!("MCP server listening on http://{addr}/mcp");
            axum::serve(listener, app).with_graceful_shutdown(shutdown_signal()).await?;
        }
        #[cfg(not(feature = "streamable-http"))]
        "streamable-http" => {
            return Err(std::io::Error::other(
                "MCP_TRANSPORT=streamable-http requires building with the `streamable-http` cargo feature",
            )
            .into());
        }
        other => {
            #[cfg(feature = "streamable-http")]
            let valid_values = "'stdio', 'streamable-http'";
            #[cfg(not(feature = "streamable-http"))]
            let valid_values = "'stdio'";
            return Err(std::io::Error::other(format!(
                "Unknown MCP_TRANSPORT value: '{other}'. Valid values: {valid_values}"
            ))
            .into());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_bool_env, resolve_default_models};
    use std::env;

    #[test]
    fn parses_truthy_values() {
        for value in ["true", "TRUE"] {
            assert!(parse_bool_env("TEST_BOOL", value).unwrap());
        }
    }

    #[test]
    fn parses_falsy_values() {
        for value in ["false", "FALSE"] {
            assert!(!parse_bool_env("TEST_BOOL", value).unwrap());
        }
    }

    #[test]
    fn uses_default_when_value_is_missing() {
        assert!(optional_bool_env_value(None, true).unwrap());
        assert!(!optional_bool_env_value(None, false).unwrap());
    }

    #[test]
    fn rejects_invalid_values() {
        let error = parse_bool_env("TEST_BOOL", "maybe").unwrap_err();
        assert!(error.to_string().contains("TEST_BOOL"));
    }

    #[test]
    fn tokenless_mode_does_not_set_an_explicit_default_model() {
        unsafe {
            env::remove_var("PERPLEXITY_ASK_MODEL");
            env::remove_var("PERPLEXITY_REASON_MODEL");
        }

        let (ask, reason) = resolve_default_models(true).unwrap();
        assert!(ask.is_none());
        assert!(reason.is_none());
    }

    fn optional_bool_env_value(
        value: Option<&str>,
        default: bool,
    ) -> Result<bool, std::io::Error> {
        value.map_or(Ok(default), |value| parse_bool_env("TEST_BOOL", value))
    }
}
