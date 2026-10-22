use super::{Client, ConnectionLease};
use crate::errors::Result;
use rmcp::{ErrorData, RoleServer, ServerHandler, model::*, service::RequestContext};
use serde_json::Value;

#[derive(Clone)]
pub struct Proxy {
    client: Client,
    info: ServerConfig,
    _connection: std::sync::Arc<ConnectionLease>,
}

impl Proxy {
    pub async fn connect(client: Client) -> Result<Self> {
        let info = client.info().await?;
        let connection = client.track_connection().await?;
        Ok(Self {
            client,
            info,
            _connection: std::sync::Arc::new(connection),
        })
    }
}

impl ServerHandler for Proxy {
    fn get_info(&self) -> ServerConfig {
        self.info.clone()
    }
    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> std::result::Result<ListToolsResult, ErrorData> {
        self.client
            .tools()
            .await
            .map(ListToolsResult::with_all_items)
            .map_err(|error| ErrorData::internal_error(error.message, None))
    }
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> std::result::Result<CallToolResponse, ErrorData> {
        Ok(self
            .client
            .call(
                request.name.into_owned(),
                Value::Object(request.arguments.unwrap_or_default()),
                context.ct,
            )
            .await
            .into())
    }
}
