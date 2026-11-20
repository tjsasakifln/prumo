//! Explicit legacy lifecycle adapter. Project operations remain in the same core.
use super::*;

pub const LEGACY_PROTOCOL: &str = "2025-11-25";

#[derive(Clone)]
pub struct LegacyPrumoMcp(pub PrumoMcp);

impl ServerHandler for LegacyPrumoMcp {
    async fn discover(
        &self,
        _context: RequestContext<RoleServer>,
    ) -> Result<rmcp::model::DiscoverResult, ErrorData> {
        Err(ErrorData::method_not_found::<
            rmcp::model::DiscoverRequestMethod,
        >())
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Borrowed(&[ProtocolVersion::V_2025_11_25])
    }

    fn get_info(&self) -> ServerConfig {
        self.0
            .get_info()
            .with_protocol_version(ProtocolVersion::V_2025_11_25)
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        self.0.get_tool(name)
    }

    async fn list_tools(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let mut result = self.0.list_tools(request, context).await?;
        result.result_type = None;
        result.ttl_ms = None;
        result.cache_scope = None;
        Ok(result)
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let response = self.0.call_tool(request, context).await?;
        match response {
            CallToolResponse::Complete(mut result) => {
                result.result_type = None;
                Ok(result.into())
            }
            _ => Err(ErrorData::invalid_request(
                "O perfil legado não admite retomada de tarefa.",
                None,
            )),
        }
    }

    async fn list_resources(
        &self,
        request: Option<PaginatedRequestParams>,
        context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, ErrorData> {
        let mut result = self.0.list_resources(request, context).await?;
        result.result_type = None;
        result.ttl_ms = None;
        result.cache_scope = None;
        Ok(result)
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResponse, ErrorData> {
        match self.0.read_resource(request, context).await? {
            ReadResourceResponse::Complete(mut result) => {
                result.result_type = None;
                result.ttl_ms = None;
                result.cache_scope = None;
                Ok(result.into())
            }
            _ => Err(ErrorData::invalid_request(
                "O perfil legado não admite retomada de tarefa.",
                None,
            )),
        }
    }
}
