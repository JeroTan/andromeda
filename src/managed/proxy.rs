use super::{Client, ConnectionLease};
use crate::errors::Result;
use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    model::*,
    service::{NotificationContext, RequestContext},
};
use serde_json::Value;

#[derive(Clone)]
pub struct Proxy {
    client: Client,
    info: ServerConfig,
    connection: std::sync::Arc<tokio::sync::OnceCell<ConnectionLease>>,
}

impl Proxy {
    pub async fn connect(client: Client) -> Result<Self> {
        let info = client.info().await?;
        Ok(Self {
            client,
            info,
            connection: std::sync::Arc::new(tokio::sync::OnceCell::new()),
        })
    }
}

impl ServerHandler for Proxy {
    async fn on_initialized(&self, _context: NotificationContext<RoleServer>) {
        if let Err(error) = self
            .connection
            .get_or_try_init(|| self.client.track_connection())
            .await
        {
            tracing::warn!(target: "andromeda::audit", code = error.code, "Managed connection status unavailable");
        }
    }
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
