# `open` 输出示例

## MCP 标准输出

`content[0].text` 的值如下。输出不含工具级最外层标签和缩进；开标签后和闭标签前各有一个换行符。标签内文本按原样输出，不进行 XML 实体转义。

```text
<RESULTS>
<RESULT>
<URL>
https://example.com/rust-web?format=md&lang=zh
</URL>
<PAGE>
<CHUNK>
0
</CHUNK>
<TOTAL_CHUNKS>
3
</TOTAL_CHUNKS>
<CONTENT>
# Rust & Web：当 x < y 时继续处理。
</CONTENT>
</PAGE>
</RESULT>
<RESULT>
<URL>
https://example.com/missing
</URL>
<ERROR>
page fetch failed
</ERROR>
</RESULT>
</RESULTS>
<WARNINGS>
<WARNING>
"requests[0].chunk" must be between 0 and 2; using 0
</WARNING>
</WARNINGS>
```

## MCP 结构化输出

`structuredContent` 的 JSON 值如下。

```json
{
  "results": [
    {
      "url": "https://example.com/rust-web?format=md&lang=zh",
      "page": {
        "chunk": 0,
        "total_chunks": 3,
        "content": "# Rust & Web：当 x < y 时继续处理。"
      }
    },
    {
      "url": "https://example.com/missing",
      "error": "page fetch failed"
    }
  ],
  "warning": [
    "\"requests[0].chunk\" must be between 0 and 2; using 0"
  ]
}
```
