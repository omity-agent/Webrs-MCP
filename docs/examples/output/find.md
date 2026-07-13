# `find` 输出示例

## MCP 标准输出

`content[0].text` 的值如下。输出不含工具级最外层标签和缩进；开标签后和闭标签前各有一个换行符。标签内文本按原样输出，不进行 XML 实体转义。

```text
<PAGES>
<PAGE>
<TOTAL_CHUNKS>
3
</TOTAL_CHUNKS>
<MATCHES>
<MATCH>
<CHUNK>
0
</CHUNK>
<SNIPPET>
# Rust & Web：当 x < y 时继续处理。
</SNIPPET>
</MATCH>
<MATCH>
<CHUNK>
2
</CHUNK>
<SNIPPET>
MCP 客户端 & 服务端共享结构化 JSON。
</SNIPPET>
</MATCH>
</MATCHES>
</PAGE>
</PAGES>
<WARNINGS>
<WARNING>
"requests[0].snippet_tokens" exceeds chunk_tokens (4000); using 4000
</WARNING>
</WARNINGS>
```

## MCP 结构化输出

`structuredContent` 的 JSON 值如下。

```json
{
  "pages": [
    {
      "total_chunks": 3,
      "matches": [
        {
          "chunk": 0,
          "snippet": "# Rust & Web：当 x < y 时继续处理。"
        },
        {
          "chunk": 2,
          "snippet": "MCP 客户端 & 服务端共享结构化 JSON。"
        }
      ]
    }
  ],
  "warning": [
    "\"requests[0].snippet_tokens\" exceeds chunk_tokens (4000); using 4000"
  ]
}
```
