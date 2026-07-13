# `search_query` 输出示例

## MCP 标准输出

`content[0].text` 的值如下。输出不含工具级最外层标签和缩进；开标签后和闭标签前各有一个换行符。标签内文本按原样输出，不进行 XML 实体转义。

```text
<RESULTS>
<RESULT>
<TITLE>
Rust & Web：请求延迟 < 10 ms
</TITLE>
<DATE>
2026-07-14
</DATE>
<URL>
https://example.com/rust-web?format=md&lang=zh
</URL>
<HIGHLIGHT>
介绍如何用 Rust 构建 MCP 服务，并比较 x < y 与 a & b。
</HIGHLIGHT>
</RESULT>
</RESULTS>
<WARNINGS>
<WARNING>
use "q" instead of "query"
</WARNING>
</WARNINGS>
```

## MCP 结构化输出

`structuredContent` 的 JSON 值如下。

```json
{
  "results": [
    {
      "title": "Rust & Web：请求延迟 < 10 ms",
      "date": "2026-07-14",
      "url": "https://example.com/rust-web?format=md&lang=zh",
      "highlight": "介绍如何用 Rust 构建 MCP 服务，并比较 x < y 与 a & b。"
    }
  ],
  "warning": [
    "use \"q\" instead of \"query\""
  ]
}
```
