use rmcp::serde_json::{Value, json};
pub(crate) const HANDSHAKE: &str = "2025-06-18";
pub(crate) const STATELESS: &str = "2026-07-28";
pub(crate) fn request(version: &str, method: &str, mut params: Value) -> Value {
    if version == STATELESS {
        params . as_object_mut () . unwrap () . insert ("_meta" . to_owned () , json ! ({ "io.modelcontextprotocol/protocolVersion" : version , "io.modelcontextprotocol/clientCapabilities" : { } }) ,) ;
    }
    json ! ({ "jsonrpc" : "2.0" , "id" : 1_u64 , "method" : method , "params" : params })
}
pub(crate) fn initialize(version: &str) -> Value {
    json ! ({ "jsonrpc" : "2.0" , "id" : 0_u64 , "method" : "initialize" , "params" : { "protocolVersion" : version , "capabilities" : { } , "clientInfo" : { "name" : "contract-test" , "version" : "1" } } })
}
