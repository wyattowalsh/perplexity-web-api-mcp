use perplexity_web_api::{
    Client, Model, SearchMode, SearchRequest, SearchWebResult, Source, parse_search_model,
};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

/// Request parameters shared by all Perplexity tools.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PerplexityRequest {
    /// The search query or question to ask.
    pub query: String,

    /// Information sources to search. Valid values: "web", "scholar", "social".
    /// Defaults to ["web"] if not specified.
    #[serde(default)]
    pub sources: Option<Vec<String>>,

    /// Language code (ISO 639), e.g., "en-US". Defaults to "en-US".
    #[serde(default)]
    pub language: Option<String>,
}

/// Request parameters for perplexity_search.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PerplexitySearchRequest {
    #[serde(flatten)]
    pub common: PerplexityRequest,

    /// Optional model to use for search.
    ///
    /// If not specified, the configured default model is used when set.
    #[serde(default)]
    pub model: Option<String>,
}

/// Response from Perplexity tools.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PerplexityResponse {
    /// The generated answer text.
    pub answer: Option<String>,

    /// Web search results/sources from the response.
    pub web_results: Vec<SearchWebResult>,

    /// Context for making follow-up queries.
    pub follow_up: FollowUpInfo,
}

/// Follow-up context information.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FollowUpInfo {
    /// Backend UUID for follow-up queries.
    pub backend_uuid: Option<String>,

    /// Attachment URLs from the response.
    pub attachments: Vec<String>,
}

/// MCP server wrapping Perplexity AI client.
#[derive(Clone)]
pub struct PerplexityServer {
    client: Client,
    default_model: Option<Model>,
    tool_router: ToolRouter<Self>,
}

/// Converts a `PerplexityResponse` into a `CallToolResult`.
fn response_to_tool_result(response: PerplexityResponse) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string_pretty(&response).map_err(|e| {
        McpError::internal_error(format!("JSON serialization error: {}", e), None)
    })?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

impl PerplexityServer {
    /// Creates a new server instance with the given Perplexity client.
    pub fn new(client: Client, default_model: Option<Model>) -> Self {
        Self { client, default_model, tool_router: Self::tool_router() }
    }

    /// Helper to execute a search with the given mode.
    async fn do_search(
        &self,
        params: PerplexityRequest,
        mode: SearchMode,
        requested_model: Option<Model>,
    ) -> Result<PerplexityResponse, McpError> {
        let resolved_model = requested_model.or(if mode == SearchMode::Auto {
            self.default_model
        } else {
            None
        });
        let effective_mode = if mode == SearchMode::Auto && resolved_model.is_some() {
            SearchMode::Pro
        } else {
            mode
        };

        let mut request =
            SearchRequest::new(&params.query).mode(effective_mode).incognito(true);

        if let Some(model) = resolved_model {
            request = request.model(model);
        }

        if let Some(sources) = params.sources
            && !sources.is_empty()
        {
            let parsed_sources: Vec<Source> =
                sources.iter().filter_map(|s| s.parse::<Source>().ok()).collect();
            if !parsed_sources.is_empty() {
                request = request.sources(parsed_sources);
            }
        }

        if let Some(language) = params.language {
            request = request.language(language);
        }

        let response = self.client.search(request).await.map_err(|e| {
            McpError::internal_error(format!("Perplexity API error: {}", e), None)
        })?;
        let perplexity_web_api::SearchResponse { answer, web_results, follow_up, .. } =
            response;

        Ok(PerplexityResponse {
            answer,
            web_results,
            follow_up: FollowUpInfo {
                backend_uuid: follow_up.backend_uuid,
                attachments: follow_up.attachments,
            },
        })
    }
}

#[tool_router]
impl PerplexityServer {
    /// Quick web search using Perplexity's turbo model or an optional Pro model.
    ///
    /// Best for: Quick questions, everyday searches, and conversational queries
    /// that benefit from web context.
    #[tool(
        name = "perplexity_search",
        description = "Quick web search using Perplexity AI. Best for: Quick questions, everyday searches, and conversational queries that benefit from web context."
    )]
    pub async fn perplexity_search(
        &self,
        Parameters(params): Parameters<PerplexitySearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        let requested_model = params
            .model
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(parse_search_model)
            .transpose()
            .map_err(|e| McpError::internal_error(e, None))?;

        response_to_tool_result(
            self.do_search(params.common, SearchMode::Auto, requested_model).await?,
        )
    }

    /// Deep, comprehensive research using Perplexity's sonar-deep-research model.
    ///
    /// Best for: Complex topics requiring detailed investigation, comprehensive reports,
    /// and in-depth analysis. Provides thorough analysis with citations.
    #[tool(
        name = "perplexity_research",
        description = "Deep, comprehensive research using Perplexity AI's sonar-deep-research model. Provides thorough analysis with citations. Best for: Complex topics requiring detailed investigation, comprehensive reports, and in-depth analysis."
    )]
    pub async fn perplexity_research(
        &self,
        Parameters(params): Parameters<PerplexityRequest>,
    ) -> Result<CallToolResult, McpError> {
        response_to_tool_result(self.do_search(params, SearchMode::DeepResearch, None).await?)
    }

    /// Advanced reasoning and problem-solving using Perplexity's sonar-reasoning-pro model.
    ///
    /// Best for: Logical problems, complex analysis, decision-making,
    /// and tasks requiring step-by-step reasoning.
    #[tool(
        name = "perplexity_reason",
        description = "Advanced reasoning and problem-solving using Perplexity AI's sonar-reasoning-pro model. Best for: Logical problems, complex analysis, decision-making, and tasks requiring step-by-step reasoning."
    )]
    pub async fn perplexity_reason(
        &self,
        Parameters(params): Parameters<PerplexityRequest>,
    ) -> Result<CallToolResult, McpError> {
        response_to_tool_result(self.do_search(params, SearchMode::Reasoning, None).await?)
    }
}

#[tool_handler]
impl ServerHandler for PerplexityServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Perplexity AI MCP server providing web search, deep research, and reasoning tools. \
                 Use perplexity_search for quick queries, perplexity_research for comprehensive analysis, \
                 and perplexity_reason for logical problem-solving."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
