-- The AI Agent mode, its MCP servers and the MCP endpoint's API tokens are
-- gone from the agent. Their tables, credentials included, go with them; a
-- settings backup taken before this is migrated by the same step on restore.
DROP TABLE IF EXISTS stack_mcp;
DROP TABLE IF EXISTS mcp_oauth;
DROP TABLE IF EXISTS mcp_policy;
DROP TABLE IF EXISTS api_tokens;
DROP TABLE IF EXISTS agent_skill;
DROP TABLE IF EXISTS agent_memory;
DROP TABLE IF EXISTS agent_flow;
DROP TABLE IF EXISTS agent_mcp_server;
DROP TABLE IF EXISTS agent_permissions;
DROP TABLE IF EXISTS agent_credential;
DROP TABLE IF EXISTS agent_provider;
DROP TABLE IF EXISTS agent_settings;
