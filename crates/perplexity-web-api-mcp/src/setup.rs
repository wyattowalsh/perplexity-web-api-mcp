use dialoguer::{Confirm, Password, theme::ColorfulTheme};
use perplexity_web_api::{AuthCookies, Client};
use std::{io, path::Path};

use crate::{auth::AuthTokens, config};

pub(crate) async fn run_first_run_setup(config_path: &Path) -> io::Result<Option<AuthTokens>> {
    tracing::info!(
        "No Perplexity authentication found in the environment or local config cache"
    );
    tracing::info!(
        "Interactive first-run setup can save your session token and CSRF token locally for future runs"
    );

    let prompt_result = tokio::task::spawn_blocking(prompt_for_auth)
        .await
        .map_err(|err| io::Error::other(format!("Interactive setup task failed: {err}")))??;

    let Some(auth) = prompt_result else {
        return Ok(None);
    };

    validate_auth(&auth).await?;
    config::save_auth_to_path(config_path, &auth)?;

    tracing::info!("Saved validated Perplexity authentication to {}", config_path.display());

    Ok(Some(auth))
}

fn prompt_for_auth() -> io::Result<Option<AuthTokens>> {
    let theme = ColorfulTheme::default();

    let should_configure = Confirm::with_theme(&theme)
        .with_prompt("Set up saved Perplexity authentication now?")
        .default(true)
        .interact()
        .map_err(|err| {
            io::Error::other(format!("Failed to read setup confirmation: {err}"))
        })?;

    if !should_configure {
        return Ok(None);
    }

    let session_token = Password::with_theme(&theme)
        .with_prompt("Perplexity session token (leave blank to skip)")
        .allow_empty_password(true)
        .interact()
        .map_err(|err| io::Error::other(format!("Failed to read session token: {err}")))?;

    if session_token.trim().is_empty() {
        return Ok(None);
    }

    let csrf_token = Password::with_theme(&theme)
        .with_prompt("Perplexity CSRF token (leave blank to skip)")
        .allow_empty_password(true)
        .interact()
        .map_err(|err| io::Error::other(format!("Failed to read CSRF token: {err}")))?;

    if csrf_token.trim().is_empty() {
        return Ok(None);
    }

    AuthTokens::try_new(session_token, csrf_token).map(Some)
}

async fn validate_auth(auth: &AuthTokens) -> io::Result<()> {
    Client::builder()
        .cookies(AuthCookies::new(auth.session_token(), auth.csrf_token()))
        .build()
        .await
        .map(|_| ())
        .map_err(|err| {
            io::Error::other(format!(
                "Authentication validation failed: {err}. The provided tokens were not saved."
            ))
        })
}
